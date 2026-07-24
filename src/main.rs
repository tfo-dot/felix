mod config;
mod event;
mod input;
mod state;
mod ui;

use config::{load_or_create_config, watch_config};
use event::{AppEvent, InputEvent, TrayAction, WeatherCondition};
use gtk4_layer_shell::LayerShell;
#[cfg(feature = "hyprland")]
use input::hyprland::{spawn_active_window_poller, spawn_cursor_poller, spawn_event_listener};
use input::keyboard::spawn_keyboard_tracker;
use reqwest::Url;
use reqwest::blocking::Client;
use state::{PetAnimationState, PetState, PomodoroState, PomodoroTimer};
use ui::tray::spawn_tray;
use ui::window::PetWindow;

use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting Felix Desktop Pet...");

    let mut retry_count = 0;
    while let Err(e) = gtk4::init() {
        if retry_count >= 10 {
            log::error!("Could not initialize GTK after 10 attempts: {:?}", e);
            std::process::exit(1);
        }
        log::warn!(
            "Wayland/X11 display not ready yet (attempt {}/10): {:?}. Retrying in 1 second...",
            retry_count + 1,
            e
        );
        std::thread::sleep(std::time::Duration::from_secs(1));
        retry_count += 1;
    }
    log::info!("GTK initialized successfully.");

    let app = gtk4::Application::builder()
        .application_id("org.felix.desktop-pet")
        .build();

    app.connect_activate(move |app| {
        let config = Rc::new(RefCell::new(load_or_create_config()));

        let (tx, rx) = std::sync::mpsc::channel::<AppEvent>();

        #[cfg(feature = "hyprland")]
        {
            if input::hyprland::is_hyprland_available() {
                spawn_event_listener(tx.clone());
                spawn_cursor_poller(tx.clone());
                spawn_active_window_poller(tx.clone());
            } else {
                log::warn!("Hyprland is not running. Disabling window interaction and active window polling.");
            }
        }

        #[cfg(not(feature = "hyprland"))]
        {
            log::info!("Hyprland support compiled out. Disabling window interaction and active window polling.");
        }

        spawn_keyboard_tracker(tx.clone());

        let ha_addres = std::sync::Arc::new(std::sync::RwLock::new(config.borrow().ha_address.clone()));
        let ha_addres_poller = ha_addres.clone();

        let ha_key = std::sync::Arc::new(std::sync::RwLock::new(config.borrow().ha_key.clone()));
        let ha_key_poller = ha_key.clone();

        let tx_media = tx.clone();
        std::thread::spawn(move || {
            let mut last_track = String::new();
            let mut last_weather_fetch = Instant::now() - Duration::from_secs(1800);

            loop {
                let mut is_playing = false;
                let mut track_info = String::new();

                if let Ok(out) = std::process::Command::new("playerctl")
                    .args(&["-a", "metadata", "--format", "{{ status }}|||{{ artist }} - {{ title }}"])
                    .output()
                {
                    let stdout_str = String::from_utf8_lossy(&out.stdout);
                    for line in stdout_str.lines() {
                        let parts: Vec<&str> = line.split("|||").collect();
                        if parts.len() == 2 {
                            let status = parts[0].trim().to_lowercase();
                            let meta = parts[1].trim();
                            if status.contains("playing") {
                                is_playing = true;
                                let cleaned = meta.replace(" - ", "").replace("-", "").trim().to_string();
                                if !cleaned.is_empty() && meta != "-" && meta != " - " {
                                    track_info = meta.to_string();
                                }
                            }
                        }
                    }
                }

                if is_playing {
                    if track_info.is_empty() {
                        track_info = "Music".to_string();
                    }

                    if track_info != last_track {
                        last_track = track_info.clone();
                        let _ = tx_media.send(AppEvent::TrackChanged(track_info));
                    }
                } else {
                    if !last_track.is_empty() {
                        last_track.clear();
                        let _ = tx_media.send(AppEvent::TrackChanged(String::new()));
                    }
                }

                if ha_addres.read().is_ok_and(|r| !r.is_empty()) {
                    if last_weather_fetch.elapsed() > Duration::from_secs(600) {
                        let url = Url::parse(&ha_addres_poller.read().unwrap()).expect("The HA url should be valid url if provided");

                        let response = Client::new()
                        .get(url)
                        .header("Authorization", format!("Bearer {}", ha_key_poller.read().unwrap()))
                        .header("Content-Type", "application/json").send();

                        let weather = response.expect("Home Assistant down").json::<event::WeatherState>().expect("Request didn't go through to HA API");

                        last_weather_fetch = Instant::now();

                        let _ = tx_media.send(AppEvent::WeatherChanged(weather.state));
                    }
                }

                std::thread::sleep(Duration::from_secs(1));
            }
        });

        let tx_config = tx.clone();
        let _watcher = watch_config(move || {
            let _ = tx_config.send(AppEvent::Tray(TrayAction::ReloadConfig));
        });

        Box::leak(Box::new(_watcher));

        let tray_handle = spawn_tray(tx.clone());

        let pet_window = Rc::new(PetWindow::new(app, &config.borrow(), tx.clone()));
        pet_window.window.present();

        let pet_state = Rc::new(RefCell::new(PetAnimationState::new()));
        let pomodoro = Rc::new(RefCell::new(PomodoroTimer::new(&config.borrow())));

        let last_frame_time = Rc::new(RefCell::new(Instant::now()));
        let last_cursor_pos = Rc::new(RefCell::new((0.0, 0.0)));
        let last_mouse_move_time = Rc::new(RefCell::new(Instant::now()));
        let bubble_timeout = Rc::new(RefCell::new(0u32));
        let bubble_text = Rc::new(RefCell::new(String::new()));
        let bubble_priority = Rc::new(RefCell::new(0u32));
        let current_pos = Rc::new(RefCell::new((0.0, 0.0)));
        let last_monitor_and_win = Rc::new(RefCell::new((None::<gtk4::gdk::Monitor>, None::<String>)));
        let last_resolved_monitor = Rc::new(RefCell::new(None::<gtk4::gdk::Monitor>));
        let pending_monitor = Rc::new(RefCell::new(None::<gtk4::gdk::Monitor>));

        if config.borrow().pomodoro.start_paused {
            *bubble_text.borrow_mut() = "Hi! I'm Felix. Click me to start the timer! 🐾".to_string();
            *bubble_timeout.borrow_mut() = 10;
            *bubble_priority.borrow_mut() = 0;
        }

        let tx_tick_anim = tx.clone();
        gtk4::glib::timeout_add_local(Duration::from_millis(30), move || {
            let _ = tx_tick_anim.send(AppEvent::Tick);
            gtk4::glib::ControlFlow::Continue
        });

        let tx_tick_pomo = tx.clone();
        gtk4::glib::timeout_add_local(Duration::from_secs(1), move || {
            let _ = tx_tick_pomo.send(AppEvent::Tick);
            gtk4::glib::ControlFlow::Continue
        });

        let pet_window_clone = pet_window.clone();
        let config_clone = config.clone();
        let pet_state_clone = pet_state.clone();
        let pomodoro_clone = pomodoro.clone();
        let last_cursor_pos_clone = last_cursor_pos.clone();
        let last_mouse_move_time_clone = last_mouse_move_time.clone();
        let last_frame_time_clone = last_frame_time.clone();
        let bubble_timeout_clone = bubble_timeout.clone();
        let bubble_text_clone = bubble_text.clone();
        let bubble_priority_clone = bubble_priority.clone();
        let current_pos_clone = current_pos.clone();
        let last_monitor_and_win_clone = last_monitor_and_win.clone();
        let last_resolved_monitor_clone = last_resolved_monitor.clone();
        let pending_monitor_clone = pending_monitor.clone();

        let mut last_pomodoro_second = Instant::now();

        let rx = Rc::new(RefCell::new(rx));
        let rx_clone = rx.clone();

        gtk4::glib::timeout_add_local(Duration::from_millis(16), move || {
            while let Ok(event) = rx_clone.borrow().try_recv() {
                let current_config = config_clone.borrow().clone();
                let mut state = pet_state_clone.borrow_mut();
                let mut pomo = pomodoro_clone.borrow_mut();

                let show_bubble = |text: String, timeout_val: u32, priority: u32| {
                    let current_timeout = *bubble_timeout_clone.borrow();
                    let current_priority = *bubble_priority_clone.borrow();
                    if current_timeout == 0 || priority >= current_priority {
                        *bubble_text_clone.borrow_mut() = text;
                        *bubble_timeout_clone.borrow_mut() = timeout_val;
                        *bubble_priority_clone.borrow_mut() = priority;
                    }
                };

                match event {
                    AppEvent::Input(input_event) => match input_event {
                        InputEvent::Typing => {
                            state.register_keystroke();
                        }
                        InputEvent::ActiveWindow { class, title } => {
                            state.register_activity();
                            if !class.is_empty() {
                                let class_lower = class.to_lowercase();
                                let title_lower = title.to_lowercase();

                                let display_text = if class_lower.contains("wuthering") || title_lower.contains("wuthering") ||
                                                      class_lower.contains("waves") || title_lower.contains("waves") {
                                    let msgs = [
                                        "Absorbing Echoes! 🌌",
                                        "Rover, look out! ⚔️",
                                        "Time to farm echo substats... 😭",
                                        "Defeating the Crownless! ⚔️",
                                    ];
                                    let idx = gtk4::glib::random_int_range(0, msgs.len() as i32) as usize;
                                    msgs[idx].to_string()
                                } else if class_lower.contains("reverse: 1999") || title_lower.contains("reverse: 1999") ||
                                          class_lower.contains("reverse 1999") || title_lower.contains("reverse1999") ||
                                          class_lower.contains("reverse:1999") {
                                    let msgs = [
                                        "The Storm is coming! 🌧️",
                                        "Time keeps moving backwards...",
                                        "Would you like some tea, Vertin? ☕",
                                        "Regulus is spinning records! 📻",
                                    ];
                                    let idx = gtk4::glib::random_int_range(0, msgs.len() as i32) as usize;
                                    msgs[idx].to_string()
                                } else if class_lower.contains("sublime") || title_lower.contains("sublime") ||
                                          class_lower.contains("kitty") || title_lower.contains("kitty") {
                                    let msgs = [
                                        "Compile successful! 🚀",
                                        "Fixing bugs... 🐛",
                                        "Writing some clean Rust code! 🦀",
                                        "cat /dev/urandom 🐱",
                                    ];
                                    let idx = gtk4::glib::random_int_range(0, msgs.len() as i32) as usize;
                                    msgs[idx].to_string()
                                } else {
                                    if title.is_empty() {
                                        format!("Using {}! 💻", class)
                                    } else {
                                        format!("Using {} - {}! 💻", class, title)
                                    }
                                };
                                show_bubble(display_text, 5, 1);
                            }
                        }
                        InputEvent::CursorPos { x, y } => {
                            let (lx, ly) = *last_cursor_pos_clone.borrow();
                            if x != lx || y != ly {
                                *last_cursor_pos_clone.borrow_mut() = (x, y);
                                *last_mouse_move_time_clone.borrow_mut() = Instant::now();
                            }
                        }
                        InputEvent::ActiveWindowGeom(geom) => {
                            state.active_window = Some(geom);
                        }
                        InputEvent::ActiveWindowGeomNone => {
                            state.active_window = None;
                        }
                    },
                    AppEvent::Petting => {
                        state.register_petting();
                        show_bubble("Purrr... 💓".to_string(), 3, 2);
                    }
                    AppEvent::Tray(tray_action) => match tray_action {
                        TrayAction::TogglePause => {
                            pomo.toggle_pause(&current_config);
                            let msg = if matches!(pomo.state, PomodoroState::Paused { .. }) {
                                "Timer Paused"
                            } else {
                                "Timer Resumed"
                            };
                            show_bubble(msg.to_string(), 3, 2);
                        }
                        TrayAction::ResetTimer => {
                            pomo.state = PomodoroState::Work;
                            pomo.seconds_remaining =
                                current_config.pomodoro.work_duration_mins * 60;
                            show_bubble("Timer Reset".to_string(), 3, 2);
                        }
                        TrayAction::ToggleChecklist => {
                            let visible = !pet_window_clone.checklist_card.get_visible();
                            pet_window_clone.checklist_card.set_visible(visible);
                            pet_window_clone.window.queue_resize();
                            let mut cfg = config_clone.borrow_mut();
                            cfg.checklist_visible = visible;
                            if let Ok(toml_str) = toml::to_string_pretty(&*cfg) {
                                let path = config::get_config_path();
                                let _ = std::fs::write(&path, toml_str);
                            }
                        }
                        TrayAction::ReloadConfig => {
                            let new_cfg = load_or_create_config();
                            *config_clone.borrow_mut() = new_cfg.clone();
                            pet_window_clone.update_config(&new_cfg);
                            show_bubble("Config Reloaded".to_string(), 3, 2);
                        }
                        TrayAction::Quit => {
                           std::process::exit(0)
                        }
                        TrayAction::TogglePetVisibility => {
                            let is_visible = pet_window_clone.window.is_visible();
                            pet_window_clone.window.set_visible(!is_visible);
                        }
                    },
                    AppEvent::TrackChanged(track) => {
                        if !track.is_empty() {
                            state.music_playing = true;
                            if track != "Music" {
                                show_bubble(format!("Listening to: {} 🎶", track), 12, 0);
                            }
                        } else {
                            state.music_playing = false;
                        }
                    }
                    AppEvent::TaskCompleted => {
                        state.last_task_completed = Some(Instant::now());

                        let mut particles = pet_window_clone.prop_particles.borrow_mut();
                        for _ in 0..20 {
                            let px = 80.0 + gtk4::glib::random_double() * 96.0;
                            let py = 120.0 + (gtk4::glib::random_double() - 0.5) * 40.0;
                            let size = 6.0 + gtk4::glib::random_double() * 8.0;
                            let angle = gtk4::glib::random_double() * 2.0 * std::f64::consts::PI;
                            let speed = 1.0 + gtk4::glib::random_double() * 2.5;
                            let speed_x = angle.cos() * speed;
                            let speed_y = angle.sin() * speed - 1.5;
                            particles.push(ui::window::PropParticle {
                                x: px,
                                y: py,
                                alpha: 1.0,
                                size,
                                speed_x,
                                speed_y,
                                value: 80,
                                time: 0.0,
                            });
                        }

                        let msgs = [
                            "Awesome! Task complete! 🌟",
                            "One step closer! Woohoo! 🚀",
                            "Keep it up! You're doing great! 🎉",
                            "Done and dusted! 🐾",
                        ];
                        let idx = gtk4::glib::random_int_range(0, msgs.len() as i32) as usize;
                        show_bubble(msgs[idx].to_string(), 4, 2);
                    }
                    AppEvent::WeatherChanged(w) => {
                        *pet_window_clone.current_weather.borrow_mut() = w;
                        pet_window_clone.prop_overlay.queue_draw();
                        pet_window_clone.drawing_area.queue_draw();
                    }
                    AppEvent::WorkspaceChanged => {
                        if state.current_state != PetState::PortalOut && state.current_state != PetState::PortalIn {
                            state.start_portal_transition();

                            let msgs = [
                                "Travelling to hyperspace... 🌌",
                                "🌀 Wheee! New workspace!",
                                "Warp drive engaged! 🚀",
                                "Wooooosh! 🛸",
                            ];
                            let idx = gtk4::glib::random_int_range(0, msgs.len() as i32) as usize;
                            show_bubble(msgs[idx].to_string(), 4, 1);
                        }
                    }
                    AppEvent::Tick => {
                        let now = Instant::now();

                        // A. Update Pomodoro state once per second
                        if now.duration_since(last_pomodoro_second) >= Duration::from_secs(1) {
                            last_pomodoro_second = now;
                            if let Some(alert_msg) = pomo.tick(&current_config) {
                                show_bubble(alert_msg, 8, 2);
                            }

                            // B. Update Daily Routine state
                            let hour = gtk4::glib::DateTime::now_local().map(|dt| dt.hour()).unwrap_or(12);
                            let resolved_routine = if hour >= 7 && hour < 9 {
                                state::RoutineState::Coffee
                            } else if hour >= 12 && hour < 13 {
                                state::RoutineState::Lunch
                            } else if hour >= 15 && hour < 16 {
                                state::RoutineState::Slump
                            } else if hour >= 18 && hour < 21 {
                                state::RoutineState::Reading
                            } else {
                                state::RoutineState::None
                            };

                            let mut current_r = pet_window_clone.current_routine.borrow_mut();
                            if *current_r != resolved_routine {
                                *current_r = resolved_routine;
                                pet_window_clone.drawing_area.queue_draw();
                            }

                            // Revert custom text/notification timer
                            let mut timeout = *bubble_timeout_clone.borrow();
                            if timeout > 0 {
                                timeout -= 1;
                                *bubble_timeout_clone.borrow_mut() = timeout;
                                if timeout == 0 {
                                    *bubble_priority_clone.borrow_mut() = 0;
                                }
                            } else {
                                // Periodic app-specific comments (5% chance every second)
                                let active = pet_window_clone.active_prop.get();
                                if active != ui::window::ActiveProp::None {
                                    let rand_pct = gtk4::glib::random_double();
                                    if rand_pct < 0.05 {
                                        let msg = match active {
                                            ui::window::ActiveProp::WutheringWaves => {
                                                let msgs = [
                                                    "Checking Echo stats... Max Crit Rate? 🤔",
                                                    "Rover, let's complete our daily commissions! 📋",
                                                    "This Tacet Field is active! ⚡",
                                                    "Listen to the sound of waves... 🌊",
                                                ];
                                                msgs[gtk4::glib::random_int_range(0, msgs.len() as i32) as usize]
                                            }
                                            ui::window::ActiveProp::Reverse1999 => {
                                                let msgs = [
                                                    "Is the rain falling up? 🌧️",
                                                    "Vertin, Vertin! Look at the spinning pocket watch! 🕰️",
                                                    "Let's brew some black tea. ☕",
                                                    "Keep moving, don't get caught in the Storm! 🌪️",
                                                ];
                                                msgs[gtk4::glib::random_int_range(0, msgs.len() as i32) as usize]
                                            }
                                            ui::window::ActiveProp::SublimeKitty => {
                                                let msgs = [
                                                    "Code compiles cleanly! Crab power! 🦀",
                                                    "Compiling... perfect time for a quick pet? 🥰",
                                                    "git commit -m 'pet the kitty' 🐾",
                                                    "Blinking terminal cursor is soothing... 💻",
                                                ];
                                                msgs[gtk4::glib::random_int_range(0, msgs.len() as i32) as usize]
                                            }
                                            _ => "",
                                        };
                                        if !msg.is_empty() {
                                            show_bubble(msg.to_string(), 5, 0);
                                        }
                                    }
                                }
                            }

                            // Update System Tray Tooltip info
                            let status = match &pomo.state {
                                PomodoroState::Paused { .. } => "Paused".to_string(),
                                s => {
                                    let mins = pomo.seconds_remaining / 60;
                                    let secs = pomo.seconds_remaining % 60;
                                    format!("{}: {:02}:{:02}", s.label(), mins, secs)
                                }
                            };
                            let tray_status = format!("Felix - {}", status);
                            tray_handle.update(move |tray| {
                                tray.status_text = tray_status;
                            });
                        }

                        // B. Re-evaluate pet state transitions
                        let last_move = *last_mouse_move_time_clone.borrow();
                        let is_tracking = now.duration_since(last_move) < Duration::from_secs(5);

                        // Find monitor containing active window or cursor
                        let display = gtk4::gdk::Display::default().expect("GDK display not initialized");
                        let monitors = display.monitors();

                        let mut active_monitor = None;

                        if current_config.pet.window_interaction && state.active_window.is_some() {
                            let win = state.active_window.as_ref().unwrap();
                            let wx = win.x + win.width / 2;
                            let wy = win.y + win.height / 2;
                            for i in 0..monitors.n_items() {
                                if let Some(mon) = monitors
                                    .item(i)
                                    .and_then(|item| item.downcast::<gtk4::gdk::Monitor>().ok())
                                {
                                    let geom = mon.geometry();
                                    let mx = geom.x();
                                    let my = geom.y();
                                    let mw = geom.width();
                                    let mh = geom.height();
                                    if wx >= mx && wx < mx + mw && wy >= my && wy < my + mh {
                                        active_monitor = Some(mon);
                                        break;
                                    }
                                }
                            }
                        }

                        // If not found above, fallback to containing the cursor
                        if active_monitor.is_none() && is_tracking {
                            let (cx, cy) = *last_cursor_pos_clone.borrow();
                            for i in 0..monitors.n_items() {
                                if let Some(mon) = monitors
                                    .item(i)
                                    .and_then(|item| item.downcast::<gtk4::gdk::Monitor>().ok())
                                {
                                    let geom = mon.geometry();
                                    let mx = geom.x() as f64;
                                    let my = geom.y() as f64;
                                    let mw = geom.width() as f64;
                                    let mh = geom.height() as f64;
                                    if cx >= mx && cx < mx + mw && cy >= my && cy < my + mh {
                                        active_monitor = Some(mon);
                                        break;
                                    }
                                }
                            }
                        }

                        let mut target_monitor = active_monitor.clone()
                            .or_else(|| last_resolved_monitor_clone.borrow().clone())
                            .unwrap_or_else(|| {
                                monitors
                                    .item(0)
                                    .and_then(|item| item.downcast::<gtk4::gdk::Monitor>().ok())
                                    .expect("No monitors found")
                            });

                        // Monitor change flow control
                        let current_resolved = last_resolved_monitor_clone.borrow().clone();

                        if let Some(old_mon) = current_resolved.as_ref() {
                            if old_mon != &target_monitor {
                                if pending_monitor_clone.borrow().is_none() {
                                    *pending_monitor_clone.borrow_mut() = Some(target_monitor.clone());
                                    if state.current_state != PetState::PortalOut && state.current_state != PetState::PortalIn {
                                        state.start_portal_transition();

                                        let msgs = [
                                            "Teleporting to another screen... 🌌",
                                            "🌀 Moving over!",
                                            "Crossing monitor borders! 🚀",
                                        ];
                                        let idx = gtk4::glib::random_int_range(0, msgs.len() as i32) as usize;
                                        show_bubble(msgs[idx].to_string(), 4, 1);
                                    }
                                }
                            }
                        }

                        // If a monitor change is pending, delay moving the window until we transition to PortalIn!
                        let opt_pending = pending_monitor_clone.borrow().clone();
                        if let Some(pending) = opt_pending {
                            if state.current_state == PetState::PortalIn {
                                *last_resolved_monitor_clone.borrow_mut() = Some(pending.clone());
                                target_monitor = pending;
                                *pending_monitor_clone.borrow_mut() = None;
                            } else if let Some(old_mon) = current_resolved {
                                target_monitor = old_mon;
                            }
                        } else {
                            if let Some(ref mon) = active_monitor {
                                *last_resolved_monitor_clone.borrow_mut() = Some(mon.clone());
                            }
                        }

                        let monitor = target_monitor;

                        let geom = monitor.geometry();
                        let m_x = geom.x() as f64;
                        let m_y = geom.y() as f64;
                        let m_w = geom.width() as f64;
                        let m_h = geom.height() as f64;
                        let monitor_width = geom.width();

                        // Set the window monitor
                        pet_window_clone.window.set_monitor(Some(&monitor));

                        let pet_size = current_config.pet.size;

                        // Classify active window into corresponding ActiveProp
                        let mut active_prop = ui::window::ActiveProp::None;
                        if let Some(ref win) = state.active_window {
                            let class_lower = win.class.to_lowercase();
                            let title_lower = win.title.to_lowercase();
                            if class_lower.contains("wuthering") || title_lower.contains("wuthering") ||
                               class_lower.contains("waves") || title_lower.contains("waves") {
                                active_prop = ui::window::ActiveProp::WutheringWaves;
                            } else if class_lower.contains("reverse: 1999") || title_lower.contains("reverse: 1999") ||
                                      class_lower.contains("reverse 1999") || title_lower.contains("reverse1999") ||
                                      class_lower.contains("reverse:1999") {
                                active_prop = ui::window::ActiveProp::Reverse1999;
                            } else if class_lower.contains("sublime") || title_lower.contains("sublime") ||
                                      class_lower.contains("kitty") || title_lower.contains("kitty") {
                                active_prop = ui::window::ActiveProp::SublimeKitty;
                            }
                        }
                        pet_window_clone.active_prop.set(active_prop);

                        // Update interaction state machine
                        state.update_interaction(&current_config, pet_size, monitor_width);

                        // Position the window
                        if current_config.pet.window_interaction && state.active_window.is_some() {
                            let active_window = state.active_window.as_ref().unwrap();
                            let (target_x, target_y) = match &state.interaction_state {
                                state::InteractionState::Sitting { x_rel, .. } => {
                                    (active_window.x + x_rel, active_window.y + 16)
                                }
                                state::InteractionState::Walking { x_rel, .. } => {
                                    (active_window.x + (*x_rel as i32), active_window.y + 16)
                                }
                                state::InteractionState::Climbing { y_rel, side, .. } => {
                                    let x = match side {
                                        state::ClimbSide::Left => active_window.x - pet_size + 20,
                                        state::ClimbSide::Right => active_window.x + active_window.width - 20,
                                    };
                                    (x, active_window.y + (*y_rel as i32))
                                }
                                _ => (0, 0),
                            };

                            let local_x = target_x - (m_x as i32);
                            let local_y = ((m_y + m_h) as i32) - target_y;

                            // Reset interpolation if window address or monitor changes
                            let win_address = Some(active_window.address.clone());
                            let mon_ref = Some(monitor.clone());
                            let mut last_mw = last_monitor_and_win_clone.borrow_mut();
                            if last_mw.0.as_ref() != mon_ref.as_ref() || last_mw.1 != win_address {
                                *last_mw = (mon_ref, win_address);
                                *current_pos_clone.borrow_mut() = (local_x as f64, local_y as f64);
                            }

                            // Smooth interpolation
                            let mut pos = current_pos_clone.borrow_mut();
                            let (cx, cy) = *pos;
                            let (new_x, new_y) = if cx == 0.0 && cy == 0.0 {
                                (local_x as f64, local_y as f64)
                            } else {
                                let dx = local_x as f64 - cx;
                                let dy = local_y as f64 - cy;
                                (cx + dx * 0.15, cy + dy * 0.15)
                            };
                            *pos = (new_x, new_y);

                            pet_window_clone.set_position(new_x as i32, new_y as i32);
                        } else {
                            *current_pos_clone.borrow_mut() = (0.0, 0.0);
                            *last_monitor_and_win_clone.borrow_mut() = (None, None);
                            pet_window_clone.reset_position_to_anchor(&current_config);
                        }

                        let tracking_coords = if is_tracking {
                            let (cx, cy) = *last_cursor_pos_clone.borrow();
                            let size = current_config.pet.size as f64;

                            let (win_cx, win_cy) = if current_config.pet.window_interaction && state.active_window.is_some() {
                                let active_window = state.active_window.as_ref().unwrap();
                                let (target_x, target_y) = match &state.interaction_state {
                                    state::InteractionState::Sitting { x_rel, .. } => {
                                        (active_window.x + x_rel, active_window.y + 16)
                                    }
                                    state::InteractionState::Walking { x_rel, .. } => {
                                        (active_window.x + (*x_rel as i32), active_window.y + 16)
                                    }
                                    state::InteractionState::Climbing { y_rel, side, .. } => {
                                        let x = match side {
                                            state::ClimbSide::Left => active_window.x - pet_size + 20,
                                            state::ClimbSide::Right => active_window.x + active_window.width - 20,
                                        };
                                        (x, active_window.y + (*y_rel as i32))
                                    }
                                    _ => (0, 0),
                                };
                                (target_x as f64 + size / 2.0, (target_y as f64) - size / 2.0)
                            } else {
                                let margin_r = current_config.anchor.margin_right as f64;
                                let margin_b = current_config.anchor.margin_bottom as f64;
                                (m_x + m_w - margin_r - size / 2.0, m_y + m_h - margin_b - size / 2.0)
                            };

                            let dx = cx - win_cx;
                            let dy = cy - win_cy;
                            Some((dx, dy))
                        } else {
                            None
                        };

                        state.update_state(&current_config, tracking_coords);

                        // C. Update and tick heart particles
                        {
                            let mut hearts = pet_window_clone.hearts.borrow_mut();
                            for heart in hearts.iter_mut() {
                                heart.y -= heart.speed_y;
                                heart.time += heart.osc_speed;
                                heart.x += heart.osc_x * heart.time.sin() * 0.005;
                                heart.alpha -= 0.015;
                            }
                            hearts.retain(|h| h.alpha > 0.0);

                            // If petting state is active, randomly spawn a heart
                            if state.current_state == PetState::Petting {
                                let rand_val: f64 = gtk4::glib::random_double();
                                if rand_val < 0.25 {
                                    let hx: f64 = 0.3 + 0.4 * gtk4::glib::random_double();
                                    let hy: f64 = 0.8;
                                    let h_alpha = 1.0;
                                    let h_size = 8.0 + 8.0 * gtk4::glib::random_double();
                                    let h_speed = 0.008 + 0.012 * gtk4::glib::random_double();
                                    let h_osc = 0.5 + 1.0 * gtk4::glib::random_double();
                                    let h_osc_speed = 0.1 + 0.15 * gtk4::glib::random_double();

                                    hearts.push(ui::window::Heart {
                                        x: hx,
                                        y: hy,
                                        alpha: h_alpha,
                                        size: h_size,
                                        speed_y: h_speed,
                                        osc_x: h_osc,
                                        osc_speed: h_osc_speed,
                                        time: 0.0,
                                    });
                                }
                            }
                        }

                        // C2. Update and tick prop particles
                        {
                            let active = pet_window_clone.active_prop.get();
                            let mut particles = pet_window_clone.prop_particles.borrow_mut();

                            // Tick existing particles
                            for p in particles.iter_mut() {
                                p.x += p.speed_x;
                                p.y += p.speed_y;
                                p.time += 0.05;

                                if p.value == 50 {
                                    // Music note particle
                                    p.y -= 1.0;
                                    p.x += p.time.sin() * 0.4;
                                    p.alpha -= 0.015;
                                } else if p.value == 60 {
                                    // Raindrop falling down
                                    p.y += p.speed_y;
                                    p.x += p.speed_x;
                                    if p.y > 256.0 {
                                        p.alpha = 0.0;
                                    }
                                } else if p.value == 70 {
                                    // Snowflake drifting down
                                    p.y += p.speed_y;
                                    p.x += p.time.sin() * 0.6;
                                    if p.y > 256.0 {
                                        p.alpha = 0.0;
                                    }
                                } else if p.value == 80 {
                                    // Confetti star particle
                                    p.speed_y += 0.12; // gravity
                                    p.alpha -= 0.025;
                                } else {
                                    match active {
                                        ui::window::ActiveProp::WutheringWaves => {
                                            p.x += p.time.sin() * 0.5;
                                            p.alpha -= 0.02;
                                        }
                                        ui::window::ActiveProp::Reverse1999 => {
                                            if p.y < 0.0 {
                                                p.alpha = 0.0;
                                            }
                                        }
                                        ui::window::ActiveProp::SublimeKitty => {
                                            p.x += p.speed_x * 0.1;
                                            p.alpha -= 0.015;
                                        }
                                        _ => {
                                            p.alpha = 0.0;
                                        }
                                    }
                                }
                            }

                            particles.retain(|p| p.alpha > 0.0);

                            // Spawn new particles
                            let rand_val: f64 = gtk4::glib::random_double();
                            if active != ui::window::ActiveProp::None {
                                match active {
                                    ui::window::ActiveProp::WutheringWaves => {
                                        if rand_val < 0.25 {
                                            let px = 185.0 + (gtk4::glib::random_double() - 0.5) * 35.0;
                                            let py = 85.0 + (gtk4::glib::random_double() - 0.5) * 35.0;
                                            let size = 4.0 + gtk4::glib::random_double() * 6.0;
                                            let speed_y = -0.3 - gtk4::glib::random_double() * 0.8;
                                            let speed_x = (gtk4::glib::random_double() - 0.5) * 0.6;
                                            let value = (gtk4::glib::random_int_range(0, 100) % 2) as u8;

                                            particles.push(ui::window::PropParticle {
                                                x: px,
                                                y: py,
                                                alpha: 1.0,
                                                size,
                                                speed_x,
                                                speed_y,
                                                value,
                                                time: 0.0,
                                            });
                                        }
                                    }
                                    ui::window::ActiveProp::Reverse1999 => {
                                        if rand_val < 0.35 {
                                            let px = gtk4::glib::random_double() * 256.0;
                                            let py = 250.0;
                                            let size = 10.0 + gtk4::glib::random_double() * 15.0;
                                            let speed_y = -4.0 - gtk4::glib::random_double() * 4.0;

                                            particles.push(ui::window::PropParticle {
                                                x: px,
                                                y: py,
                                                alpha: 1.0,
                                                size,
                                                speed_x: 0.0,
                                                speed_y,
                                                value: 0,
                                                time: 0.0,
                                            });
                                        }
                                    }
                                    ui::window::ActiveProp::SublimeKitty => {
                                        if rand_val < 0.15 {
                                            let px = 100.0 + gtk4::glib::random_double() * 140.0;
                                            let py = 150.0 + gtk4::glib::random_double() * 50.0;
                                            let size = 9.0 + gtk4::glib::random_double() * 4.0;
                                            let speed_y = -0.5 - gtk4::glib::random_double() * 0.7;
                                            let speed_x = -0.3 - gtk4::glib::random_double() * 0.5;
                                            let value = gtk4::glib::random_int_range(0, 7) as u8;

                                            particles.push(ui::window::PropParticle {
                                                x: px,
                                                y: py,
                                                alpha: 1.0,
                                                size,
                                                speed_x,
                                                speed_y,
                                                value,
                                                time: 0.0,
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            {
                                let px = gtk4::glib::random_double() * 256.0;
                                let py = 0.0;

                                match *pet_window_clone.current_weather.borrow() {
                                                            WeatherCondition::Rainy | WeatherCondition::Pouring if rand_val < 0.3 => {
                                                                let size = 8.0 + gtk4::glib::random_double() * 12.0;
                                                                let speed_y = 4.0 + gtk4::glib::random_double() * 3.0;
                                                                let speed_x = -0.5 + gtk4::glib::random_double() * 1.0;
                                                                particles.push(ui::window::PropParticle {
                                                                    x: px,
                                                                    y: py,
                                                                    alpha: 1.0,
                                                                    size,
                                                                    speed_x,
                                                                    speed_y,
                                                                    value: 60,
                                                                    time: 0.0,
                                                                });
                                                            },
                                                            WeatherCondition::Snowy | WeatherCondition::SnowyRainy if rand_val < 0.15 => {
                                                                let size = 3.0 + gtk4::glib::random_double() * 4.0;
                                                                let speed_y = 0.8 + gtk4::glib::random_double() * 0.8;
                                                                let speed_x = -0.2 + gtk4::glib::random_double() * 0.4;
                                                                particles.push(ui::window::PropParticle {
                                                                    x: px,
                                                                    y: py,
                                                                    alpha: 1.0,
                                                                    size,
                                                                    speed_x,
                                                                    speed_y,
                                                                    value: 70,
                                                                    time: 0.0,
                                                                });
                                                            },
                                                            _ => (),
                                                        }
                                                    }

                            // Spawn music notes if dancing
                            let current_state = state.current_state;
                            use crate::state::PetState;
                            if current_state == PetState::Dancing {
                                if rand_val < 0.08 {
                                    let px = 90.0 + gtk4::glib::random_double() * 80.0;
                                    let py = 140.0 + gtk4::glib::random_double() * 30.0;
                                    let size = 12.0 + gtk4::glib::random_double() * 6.0;
                                    particles.push(ui::window::PropParticle {
                                        x: px,
                                        y: py,
                                        alpha: 1.0,
                                        size,
                                        speed_x: 0.0,
                                        speed_y: 0.0,
                                        value: 50,
                                        time: 0.0,
                                    });
                                }
                            }
                        }

                        // D. Tick animations if elapsed time exceeds the state's tick interval
                        let elapsed = now
                            .duration_since(*last_frame_time_clone.borrow())
                            .as_millis() as u64;

                        let has_hearts = pet_window_clone.hearts.borrow().is_empty();
                        let has_prop_particles = pet_window_clone.prop_particles.borrow().is_empty();

                        if elapsed >= state.get_tick_interval() {
                            *last_frame_time_clone.borrow_mut() = now;

                            let (fx, fy) = state.get_sprite_coordinates();
                            *pet_window_clone.frame_coords.borrow_mut() = (fx, fy);
                            pet_window_clone.current_pet_state.set(state.current_state);
                            pet_window_clone.drawing_area.queue_draw();
                            pet_window_clone.prop_overlay.queue_draw();
                        } else if has_hearts || has_prop_particles {
                            pet_window_clone.current_pet_state.set(state.current_state);
                            pet_window_clone.drawing_area.queue_draw();
                            pet_window_clone.prop_overlay.queue_draw();
                        } else {
                            let active = pet_window_clone.active_prop.get();
                            if active != ui::window::ActiveProp::None {
                                pet_window_clone.prop_overlay.queue_draw();
                            }
                        }

                        // D. Update Speech Bubble Display text
                        let current_timeout = *bubble_timeout_clone.borrow();
                        if current_timeout > 0 {
                            pet_window_clone
                                .bubble
                                .set_text(&bubble_text_clone.borrow());
                        } else if !matches!(pomo.state, PomodoroState::Paused { .. }) {
                            // Display active pomodoro timer
                            let mins = pomo.seconds_remaining / 60;
                            let secs = pomo.seconds_remaining % 60;
                            let pomo_text =
                                format!("{}: {:02}:{:02}", pomo.state.label(), mins, secs);
                            pet_window_clone.bubble.set_text(&pomo_text);
                        } else {
                            pet_window_clone.bubble.hide();
                        }
                    }
                }
            }
            gtk4::glib::ControlFlow::Continue
        });
    });

    app.run();
}
