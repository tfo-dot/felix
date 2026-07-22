use crate::config::Config;
use crate::event::{AppEvent, TrayAction};
use crate::state::RoutineState;
use crate::ui::bubble::SpeechBubble;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::fs::{self, File};
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskItem {
    pub id: String,
    pub text: String,
    pub completed: bool,
}

pub fn get_tasks_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").expect("Home should be defined");
    std::path::PathBuf::from(home).join(".config/pet-app/tasks.json")
}

pub fn load_tasks() -> Vec<TaskItem> {
    let path = get_tasks_path();
    if !path.exists() {
        return vec![
            TaskItem { id: "1".into(), text: "Focus on coding 🦀".into(), completed: false },
            TaskItem { id: "2".into(), text: "Pet Felix for good luck! 🐾".into(), completed: false },
        ];
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save_tasks(tasks: &[TaskItem]) {
    let path = get_tasks_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json_str) = serde_json::to_string_pretty(tasks) {
        let _ = fs::write(&path, json_str);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveProp {
    None,
    WutheringWaves,
    Reverse1999,
    SublimeKitty,
}

#[derive(Clone, Copy)]
pub struct PropParticle {
    pub x: f64,
    pub y: f64,
    pub alpha: f64,
    pub size: f64,
    pub speed_x: f64,
    pub speed_y: f64,
    pub value: u8,
    pub time: f64,
}

#[derive(Clone, Copy)]
pub struct Heart {
    pub x: f64,
    pub y: f64,
    pub alpha: f64,
    pub size: f64,
    pub speed_y: f64,
    pub osc_x: f64,
    pub osc_speed: f64,
    pub time: f64,
}

pub struct PetWindow {
    pub window: gtk4::ApplicationWindow,
    pub bubble: SpeechBubble,
    pub drawing_area: gtk4::DrawingArea,
    pub prop_overlay: gtk4::DrawingArea,
    pub frame_coords: Rc<RefCell<(i32, i32)>>,
    pub pet_scale: Rc<Cell<f64>>,
    pub pet_size: Rc<Cell<i32>>,
    pub hearts: Rc<RefCell<Vec<Heart>>>,
    pub current_pet_state: Rc<Cell<crate::state::PetState>>,
    pub active_prop: Rc<Cell<ActiveProp>>,
    pub prop_particles: Rc<RefCell<Vec<PropParticle>>>,
    pub checklist_card: gtk4::Box,
    #[allow(dead_code)]
    pub tasks: Rc<RefCell<Vec<TaskItem>>>,
    pub current_weather: Rc<RefCell<String>>,
    pub current_routine: Rc<RefCell<RoutineState>>,
}

impl PetWindow {
    pub fn new(app: &gtk4::Application, config: &Config, tx: Sender<AppEvent>) -> Self {
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Felix Desktop Pet")
            .build();
        window.add_css_class("pet-window-class");

        // Add click gesture
        let gesture = gtk4::GestureClick::new();
        let tx_click = tx.clone();
        let gesture_clone = gesture.clone();
        gesture.connect_pressed(move |_, n_press, _x, _y| {
            let button = gesture_clone.current_button();
            if button == 1 {
                // Left Click
                if n_press == 1 {
                    let _ = tx_click.send(AppEvent::Tray(TrayAction::TogglePause));
                } else if n_press == 2 {
                    let _ = tx_click.send(AppEvent::Tray(TrayAction::ResetTimer));
                }
            }
        });
        window.add_controller(gesture);

        // 1. Initialize Layer Shell
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::None);
        window.set_decorated(false);
        window.set_namespace(Some("felix-desktop-pet"));
        window.set_exclusive_zone(0);
        window.set_default_widget(None::<&gtk4::Widget>);

        window.connect_realize(|window| {
            if let Some(surface) = window.surface() {
                surface.set_opaque_region(None::<&cairo::Region>);
            }
        });

        // Apply configuration anchors/margins
        update_window_properties(&window, config);

        // 2. Load the processed sprite sheet as a Cairo surface
        let surface = match File::open("assets/pet_spritesheet.png") {
            Ok(mut file) => match gtk4::cairo::ImageSurface::create_from_png(&mut file) {
                Ok(surf) => Some(surf),
                Err(e) => {
                    eprintln!("Failed to parse PNG sprite sheet: {:?}", e);
                    None
                }
            },
            Err(e) => {
                eprintln!("Failed to open sprite sheet: {:?}", e);
                None
            }
        };

        // Load the wuthering waves terminal gourd asset
        let wuthering_surface = match File::open("assets/wuthering_terminal.png") {
            Ok(mut file) => match gtk4::cairo::ImageSurface::create_from_png(&mut file) {
                Ok(surf) => Some(surf),
                Err(e) => {
                    eprintln!("Failed to parse PNG wuthering terminal: {:?}", e);
                    None
                }
            },
            Err(e) => {
                eprintln!("Failed to open wuthering terminal: {:?}", e);
                None
            }
        };

        // Shared state for frame coordinates and sizing
        let frame_coords = Rc::new(RefCell::new((0, 0)));
        let pet_scale = Rc::new(Cell::new(config.pet.scale));
        let pet_size = Rc::new(Cell::new(config.pet.size));
        let hearts = Rc::new(RefCell::new(Vec::<Heart>::new()));
        let current_pet_state = Rc::new(Cell::new(crate::state::PetState::Idle));
        let active_prop = Rc::new(Cell::new(ActiveProp::None));
        let current_weather = Rc::new(RefCell::new(config.weather.clone()));
        let current_routine = Rc::new(RefCell::new(RoutineState::None));
        let prop_particles = Rc::new(RefCell::new(Vec::<PropParticle>::new()));

        // 3. Create the DrawingArea for the pet sprite
        let drawing_area = gtk4::DrawingArea::builder()
            .width_request(config.pet.size)
            .height_request(config.pet.size)
            .halign(gtk4::Align::Center)
            .build();

        // Create the DrawingArea for the prop overlay
        let prop_overlay = gtk4::DrawingArea::builder()
            .width_request(config.pet.size)
            .height_request(config.pet.size)
            .halign(gtk4::Align::Center)
            .build();

        prop_overlay.set_can_target(false);

        // 4. Motion controller for Petting detection
        let motion = gtk4::EventControllerMotion::new();
        let tx_motion = tx.clone();
        let last_motion_x = Cell::new(0.0);
        let direction_changes = Cell::new(0);
        let last_dir = Cell::new(0.0);
        let last_stroke_time = Cell::new(Instant::now());

        motion.connect_motion(move |_, x, _y| {
            let now = Instant::now();
            let lx = last_motion_x.get();
            let dx = x - lx;
            last_motion_x.set(x);

            if dx.abs() > 3.0 {
                let dir = if dx > 0.0 { 1.0 } else { -1.0 };
                let ldir = last_dir.get();
                if ldir != 0.0 && dir != ldir {
                    let lst = last_stroke_time.get();
                    if now.duration_since(lst) < Duration::from_millis(500) {
                        let count = direction_changes.get() + 1;
                        if count >= 4 {
                            let _ = tx_motion.send(AppEvent::Petting);
                            direction_changes.set(0);
                        } else {
                            direction_changes.set(count);
                        }
                    } else {
                        direction_changes.set(1);
                    }
                    last_stroke_time.set(now);
                    last_dir.set(dir);
                } else if ldir == 0.0 {
                    last_dir.set(dir);
                    last_stroke_time.set(now);
                }
            }
        });
        drawing_area.add_controller(motion);

        if let Some(surf) = surface {
            let frame_coords_clone = frame_coords.clone();
            let pet_scale_clone = pet_scale.clone();
            let pet_size_clone = pet_size.clone();
            let surf_clone = surf.clone();
            let hearts_clone = hearts.clone();
            let current_pet_state_clone = current_pet_state.clone();
            let current_weather_clone = current_weather.clone();
            let current_routine_clone = current_routine.clone();
            let start_time = Instant::now();

            drawing_area.set_draw_func(move |_area, cr, width, height| {
                cr.save().unwrap();
                cr.set_operator(cairo::Operator::Clear);
                cr.paint().unwrap();
                cr.restore().unwrap();

                let (fx, fy) = *frame_coords_clone.borrow();
                let scale_val = pet_scale_clone.get();
                let size_val = pet_size_clone.get();
                let current_state = current_pet_state_clone.get();
                let elapsed_ms = start_time.elapsed().as_millis() as f64;
                cr.save().unwrap();

                // Scale coordinate space from base 256x256 size
                let s = scale_val * (size_val as f64 / 256.0);
                cr.scale(s, s);

                // Apply wobble/bobbing animations for walking/climbing/dancing
                let mut offset_x = 0.0;
                let mut offset_y = 0.0;
                let mut rotation = 0.0;
                let mut portal_scale = 1.0;
                let mut portal_opacity = 1.0;

                let frame_idx = (fx / 256) as u32;

                use crate::state::PetState;
                match current_state {
                    PetState::WalkingLeft | PetState::WalkingRight => {
                        // Bob up and down (feet stepping)
                        offset_y = 6.0 * (elapsed_ms / 120.0).sin().abs();
                        // Wobble side to side
                        rotation = 0.06 * (elapsed_ms / 100.0).sin();
                    }
                    PetState::ClimbingUp | PetState::ClimbingDown => {
                        // Body wiggle left/right
                        offset_x = 5.0 * (elapsed_ms / 80.0).sin();
                        // Bob up and down in the climb direction
                        offset_y = 4.0 * (elapsed_ms / 80.0).cos().abs();
                        // Rotate slightly
                        rotation = 0.05 * (elapsed_ms / 80.0).sin();
                    }
                    PetState::Dancing => {
                        // Bouncy dance!
                        offset_y = 12.0 * (elapsed_ms / 90.0).sin().abs() - 6.0;
                        rotation = 0.18 * (elapsed_ms / 100.0).sin();
                    }
                    PetState::PortalOut => {
                        if frame_idx < 2 {
                            // idle
                        } else if frame_idx >= 2 && frame_idx <= 5 {
                            let progress = (frame_idx - 2) as f64 / 3.0;
                            offset_y = progress * 60.0;
                            portal_scale = (1.0 - progress * 0.95).max(0.001);
                            portal_opacity = 1.0 - progress;
                        } else {
                            portal_opacity = 0.0;
                        }
                    }
                    PetState::PortalIn => {
                        if frame_idx < 2 {
                            portal_opacity = 0.0;
                        } else if frame_idx >= 2 && frame_idx <= 5 {
                            let progress = (frame_idx - 2) as f64 / 3.0;
                            offset_y = (1.0 - progress) * 60.0;
                            portal_scale = progress.max(0.001);
                            portal_opacity = progress;
                        } else {
                            let progress = (frame_idx - 6) as f64 / 1.0;
                            offset_y = -8.0 * (1.0 - progress);
                        }
                    }
                    _ => {}
                }

                // Draw the portal vortex under Felix's feet if in portal state
                if current_state == PetState::PortalOut || current_state == PetState::PortalIn {
                    cr.save().unwrap();
                    let vortex_max_radius = 45.0;
                    let vortex_radius;
                    
                    if current_state == PetState::PortalOut {
                        if frame_idx <= 2 {
                            let progress = frame_idx as f64 / 2.0;
                            vortex_radius = progress * vortex_max_radius;
                        } else if frame_idx > 2 && frame_idx <= 5 {
                            vortex_radius = vortex_max_radius;
                        } else {
                            let progress = (7 - frame_idx) as f64 / 2.0;
                            vortex_radius = progress * vortex_max_radius;
                        }
                    } else {
                        if frame_idx <= 2 {
                            let progress = frame_idx as f64 / 2.0;
                            vortex_radius = progress * vortex_max_radius;
                        } else if frame_idx > 2 && frame_idx <= 5 {
                            vortex_radius = vortex_max_radius;
                        } else {
                            let progress = (7 - frame_idx) as f64 / 2.0;
                            vortex_radius = progress * vortex_max_radius;
                        }
                    }
                    
                    if vortex_radius > 0.0 {
                        cr.translate(128.0, 165.0);
                        cr.scale(1.0, 0.35);
                        
                        let rad_grad = cairo::RadialGradient::new(
                            0.0, 0.0, vortex_radius * 0.2,
                            0.0, 0.0, vortex_radius
                        );
                        rad_grad.add_color_stop_rgba(0.0, 0.1, 0.0, 0.3, 0.8);
                        rad_grad.add_color_stop_rgba(0.5, 0.6, 0.2, 0.9, 0.95);
                        rad_grad.add_color_stop_rgba(0.9, 0.2, 0.5, 1.0, 0.9);
                        rad_grad.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 0.0);
                        
                        cr.set_source(&rad_grad).unwrap();
                        cr.rotate(elapsed_ms * 0.007);
                        cr.arc(0.0, 0.0, vortex_radius, 0.0, 2.0 * std::f64::consts::PI);
                        cr.fill().unwrap();
                        
                        cr.set_source_rgba(0.9, 0.8, 1.0, 0.9);
                        for i in 0..4 {
                            let angle = (i as f64 * std::f64::consts::PI / 2.0) + (elapsed_ms * 0.005);
                            let sx = angle.cos() * (vortex_radius * 0.6);
                            let sy = angle.sin() * (vortex_radius * 0.6);
                            cr.arc(sx, sy, 3.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                        }
                    }
                    cr.restore().unwrap();
                }

                if rotation != 0.0 || offset_x != 0.0 || offset_y != 0.0 || portal_scale != 1.0 {
                    cr.translate(128.0 + offset_x, 160.0 + offset_y);
                    cr.scale(portal_scale, portal_scale);
                    cr.rotate(rotation);
                    cr.translate(-128.0, -160.0);
                }

                // Clip drawing to a single 256x256 frame
                cr.rectangle(0.0, 0.0, 256.0, 256.0);
                cr.clip();

                // Draw surface offset by current frame coordinates
                cr.set_source_surface(&surf_clone, -fx as f64, -fy as f64)
                    .unwrap();
                cr.set_operator(cairo::Operator::Over);
                cr.paint_with_alpha(portal_opacity).unwrap();

                // Draw accessories that attach to the pet (bobbing and rotating with it)
                let hour = gtk4::glib::DateTime::now_local().map(|dt| dt.hour()).unwrap_or(12);
                let is_night = hour >= 21 || hour < 6;
                // 1. Draw Nightcap if it is night and pet is sleeping
                if is_night && current_state == PetState::Sleeping {
                    cr.save().unwrap();
                    cr.set_line_width(2.0);

                    // Cap body (cone leaning to the left)
                    cr.set_source_rgba(0.35, 0.3, 0.7, 1.0); // purple
                    cr.move_to(100.0, 80.0);
                    cr.curve_to(110.0, 45.0, 115.0, 40.0, 120.0, 38.0); // top point
                    cr.line_to(140.0, 75.0);
                    cr.close_path();
                    cr.fill().unwrap();

                    // White brim
                    cr.set_source_rgba(0.95, 0.95, 0.95, 1.0);
                    cr.set_line_width(6.0);
                    cr.move_to(96.0, 80.0);
                    cr.line_to(144.0, 76.0);
                    cr.stroke().unwrap();

                    // Pompom at the top point
                    cr.arc(120.0, 34.0, 6.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.fill().unwrap();
                    cr.restore().unwrap();
                }

                // 2. Draw Sunglasses if weather is sunny
                let weather = current_weather_clone.borrow().to_lowercase();
                if weather == "sunny" {
                    cr.save().unwrap();
                    cr.set_source_rgba(0.12, 0.12, 0.12, 0.95); // dark lenses

                    // Left Lens
                    cr.arc(114.0, 120.0, 10.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.fill().unwrap();
                    cr.set_source_rgba(0.9, 0.75, 0.1, 1.0); // Gold rim
                    cr.set_line_width(1.5);
                    cr.arc(114.0, 120.0, 10.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.stroke().unwrap();

                    // Right Lens
                    cr.set_source_rgba(0.12, 0.12, 0.12, 0.95);
                    cr.arc(142.0, 120.0, 10.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.fill().unwrap();
                    cr.set_source_rgba(0.9, 0.75, 0.1, 1.0);
                    cr.arc(142.0, 120.0, 10.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.stroke().unwrap();

                    // Bridge
                    cr.move_to(124.0, 120.0);
                    cr.line_to(132.0, 120.0);
                    cr.stroke().unwrap();

                    // Glare reflection
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.4);
                    cr.move_to(110.0, 115.0);
                    cr.line_to(116.0, 123.0);
                    cr.move_to(138.0, 115.0);
                    cr.line_to(144.0, 123.0);
                    cr.set_line_width(1.5);
                    cr.stroke().unwrap();

                    cr.restore().unwrap();
                }

                // 3. Draw Scarf if weather is snowy
                if weather == "snowy" {
                    cr.save().unwrap();
                    cr.set_source_rgba(0.85, 0.15, 0.15, 1.0); // Red scarf

                    // Main neck loop
                    cr.set_line_width(12.0);
                    cr.set_line_cap(cairo::LineCap::Round);
                    cr.move_to(104.0, 148.0);
                    cr.curve_to(116.0, 154.0, 140.0, 154.0, 152.0, 148.0);
                    cr.stroke().unwrap();

                    // Dangling ends
                    cr.set_line_width(8.0);
                    cr.move_to(142.0, 152.0);
                    cr.line_to(146.0, 175.0);
                    cr.stroke().unwrap();

                    cr.move_to(148.0, 151.0);
                    cr.line_to(156.0, 170.0);
                    cr.stroke().unwrap();

                    cr.restore().unwrap();
                }

                // 4. Draw Umbrella if weather is rainy
                if weather == "rainy" {
                    cr.save().unwrap();

                    // Stick
                    cr.set_source_rgba(0.4, 0.4, 0.4, 1.0);
                    cr.set_line_width(3.0);
                    cr.move_to(80.0, 50.0);
                    cr.line_to(80.0, 160.0);
                    cr.stroke().unwrap();

                    // Handle
                    cr.arc(84.0, 160.0, 4.0, 180.0 * std::f64::consts::PI / 180.0, 0.0);
                    cr.stroke().unwrap();

                    // Canopy dome
                    cr.set_source_rgba(0.0, 0.6, 0.85, 0.85);
                    cr.arc(80.0, 80.0, 45.0, 180.0 * std::f64::consts::PI / 180.0, 0.0);
                    cr.close_path();
                    cr.fill().unwrap();

                    // Umbrella tip
                    cr.set_source_rgba(0.3, 0.3, 0.3, 1.0);
                    cr.rectangle(78.0, 30.0, 4.0, 6.0);
                    cr.fill().unwrap();

                    cr.restore().unwrap();
                }

                // 5. Draw Daily Routine Props (Steaming coffee mug, bento box sandwich, book)
                let routine = current_routine_clone.borrow().clone();
                if current_state == PetState::Idle {
                    match routine {
                        RoutineState::Coffee => {
                            cr.save().unwrap();
                            cr.set_line_width(1.5);

                            cr.set_source_rgba(0.95, 0.95, 0.95, 1.0);
                            cr.rectangle(84.0, 142.0, 16.0, 18.0);
                            cr.fill().unwrap();
                            
                            cr.arc(84.0, 151.0, 4.0, 90.0 * std::f64::consts::PI / 180.0, 270.0 * std::f64::consts::PI / 180.0);
                            cr.stroke().unwrap();
                            
                            cr.set_source_rgba(0.45, 0.25, 0.15, 1.0);
                            cr.save().unwrap();
                            cr.translate(92.0, 142.0);
                            cr.scale(1.0, 0.3);
                            cr.arc(0.0, 0.0, 7.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                            cr.restore().unwrap();
                            
                            cr.set_source_rgba(0.9, 0.9, 0.9, 0.65);
                            cr.set_line_width(1.2);
                            let time_factor = elapsed_ms * 0.006;
                            for i in 0..2 {
                                let sx = 89.0 + (i as f64 * 6.0);
                                cr.move_to(sx, 137.0);
                                cr.curve_to(
                                    sx + (time_factor + i as f64).sin() * 2.0, 131.0,
                                    sx - (time_factor + i as f64).sin() * 2.0, 126.0,
                                    sx, 120.0
                                );
                                cr.stroke().unwrap();
                            }
                            
                            cr.restore().unwrap();
                        }
                        RoutineState::Lunch => {
                            cr.save().unwrap();
                            
                            cr.set_source_rgba(0.85, 0.7, 0.5, 1.0);
                            cr.move_to(84.0, 160.0);
                            cr.line_to(102.0, 160.0);
                            cr.line_to(93.0, 144.0);
                            cr.close_path();
                            cr.fill().unwrap();
                            
                            cr.set_source_rgba(0.3, 0.8, 0.2, 1.0);
                            cr.set_line_width(3.0);
                            cr.move_to(86.0, 158.0);
                            cr.line_to(100.0, 158.0);
                            cr.stroke().unwrap();
                            
                            cr.set_source_rgba(0.9, 0.2, 0.2, 1.0);
                            cr.set_line_width(2.0);
                            cr.move_to(89.0, 156.0);
                            cr.line_to(97.0, 156.0);
                            cr.stroke().unwrap();
                            
                            cr.set_source_rgba(0.9, 0.75, 0.55, 1.0);
                            cr.move_to(86.0, 160.0);
                            cr.line_to(104.0, 160.0);
                            cr.line_to(95.0, 146.0);
                            cr.close_path();
                            cr.fill().unwrap();
                            
                            cr.restore().unwrap();
                        }
                        RoutineState::Reading => {
                            cr.save().unwrap();
                            
                            cr.set_source_rgba(0.5, 0.25, 0.1, 1.0);
                            cr.rectangle(110.0, 152.0, 36.0, 10.0);
                            cr.fill().unwrap();
                            
                            cr.set_source_rgba(0.96, 0.96, 0.96, 1.0);
                            cr.move_to(128.0, 152.0);
                            cr.curve_to(123.0, 148.0, 116.0, 148.0, 112.0, 152.0);
                            cr.line_to(112.0, 160.0);
                            cr.curve_to(116.0, 156.0, 123.0, 156.0, 128.0, 160.0);
                            cr.close_path();
                            cr.fill().unwrap();
                            
                            cr.move_to(128.0, 152.0);
                            cr.curve_to(133.0, 148.0, 140.0, 148.0, 144.0, 152.0);
                            cr.line_to(144.0, 160.0);
                            cr.curve_to(140.0, 156.0, 133.0, 156.0, 128.0, 160.0);
                            cr.close_path();
                            cr.fill().unwrap();
                            
                            cr.set_source_rgba(0.2, 0.2, 0.2, 0.8);
                            cr.set_line_width(1.0);
                            
                            cr.move_to(115.0, 152.0); cr.line_to(123.0, 152.0); cr.stroke().unwrap();
                            cr.move_to(114.0, 154.0); cr.line_to(124.0, 154.0); cr.stroke().unwrap();
                            cr.move_to(115.0, 156.0); cr.line_to(121.0, 156.0); cr.stroke().unwrap();
                            
                            cr.move_to(133.0, 152.0); cr.line_to(141.0, 152.0); cr.stroke().unwrap();
                            cr.move_to(132.0, 154.0); cr.line_to(142.0, 154.0); cr.stroke().unwrap();
                            cr.move_to(135.0, 156.0); cr.line_to(141.0, 156.0); cr.stroke().unwrap();
                            
                            cr.restore().unwrap();
                        }
                        _ => {}
                    }
                }

                cr.restore().unwrap();

                // Render floating hearts in the absolute drawing area bounds
                let hearts_list = hearts_clone.borrow();
                for heart in hearts_list.iter() {
                    cr.save().unwrap();
                    cr.set_source_rgba(1.0, 0.35, 0.45, heart.alpha);

                    let hx = heart.x * (width as f64);
                    let hy = heart.y * (height as f64);
                    let sz = heart.size;

                    // Cairo Heart Path
                    cr.move_to(hx, hy + sz / 4.0);
                    cr.curve_to(
                        hx - sz / 2.0,
                        hy - sz / 2.0,
                        hx - sz,
                        hy + sz / 3.0,
                        hx,
                        hy + sz,
                    );
                    cr.curve_to(
                        hx + sz,
                        hy + sz / 3.0,
                        hx + sz / 2.0,
                        hy - sz / 2.0,
                        hx,
                        hy + sz / 4.0,
                    );
                    cr.close_path();

                    cr.fill().unwrap();
                    cr.restore().unwrap();
                }
            });
        }

        // Draw func for prop_overlay
        {
            let wuthering_surf_clone = wuthering_surface.clone();
            let active_prop_clone = active_prop.clone();
            let prop_particles_clone = prop_particles.clone();
            let pet_scale_clone = pet_scale.clone();
            let pet_size_clone = pet_size.clone();
            let current_pet_state_clone2 = current_pet_state.clone();
            let start_time = Instant::now();

            prop_overlay.set_draw_func(move |_area, cr, _width, _height| {
                let active = active_prop_clone.get();
                let scale_val = pet_scale_clone.get();
                let size_val = pet_size_clone.get();
                let elapsed_ms = start_time.elapsed().as_millis() as f64;
                let current_state = current_pet_state_clone2.get();

                cr.save().unwrap();
                let s = scale_val * (size_val as f64 / 256.0);
                cr.scale(s, s);

                // Draw active window props
                if active != ActiveProp::None {
                    match active {
                        ActiveProp::WutheringWaves => {
                            // Draw Wuthering Waves Gourd Terminal
                            if let Some(ref surf) = wuthering_surf_clone {
                                cr.save().unwrap();

                                // Terminal floats and bobs
                                let bob_y = 5.0 * (elapsed_ms / 300.0).sin();
                                let rot = 0.05 * (elapsed_ms / 400.0).cos();

                                // Position it next to the pet (top right: x = 185.0, y = 85.0)
                                let tx = 185.0;
                                let ty = 85.0 + bob_y;

                                cr.translate(tx, ty);
                                cr.rotate(rot);

                                let surf_w = surf.width() as f64;
                                let surf_h = surf.height() as f64;
                                let target_sz = 60.0;
                                let img_scale = target_sz / surf_w.max(surf_h);

                                cr.scale(img_scale, img_scale);
                                cr.set_source_surface(surf, -surf_w / 2.0, -surf_h / 2.0)
                                    .unwrap();
                                cr.paint().unwrap();
                                cr.restore().unwrap();

                                // Cyan soundwave/echo ripples expanding from the gourd
                                cr.save().unwrap();
                                cr.set_line_width(1.5);
                                let ripple_count = 3;
                                for i in 0..ripple_count {
                                    let phase = ((elapsed_ms / 10.0) + (i as f64 * 40.0)) % 120.0;
                                    let radius = 10.0 + phase * 0.4;
                                    let alpha = (1.0 - phase / 120.0).clamp(0.0, 1.0) * 0.4;
                                    cr.set_source_rgba(0.0, 0.9, 1.0, alpha);
                                    cr.arc(tx, ty, radius, 0.0, 2.0 * std::f64::consts::PI);
                                    cr.stroke().unwrap();
                                }
                                cr.restore().unwrap();
                            }
                        }
                        ActiveProp::Reverse1999 => {
                            // Draw ticking golden pocket watch next to pet (top left: x = 70, y = 90)
                            cr.save().unwrap();
                            let bob_y = 4.0 * (elapsed_ms / 250.0).cos();
                            let tx = 70.0;
                            let ty = 90.0 + bob_y;

                            cr.translate(tx, ty);

                            // Ring loop on top
                            cr.set_source_rgba(0.9, 0.7, 0.2, 1.0); // gold
                            cr.set_line_width(2.5);
                            cr.arc(0.0, -18.0, 6.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.stroke().unwrap();

                            // Outer gold casing
                            cr.arc(0.0, 0.0, 18.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();

                            // White dial face
                            cr.set_source_rgba(0.98, 0.96, 0.9, 1.0);
                            cr.arc(0.0, 0.0, 14.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();

                            // Watch ticks/hour markers (12, 3, 6, 9)
                            cr.set_source_rgba(0.2, 0.15, 0.1, 1.0);
                            cr.set_line_width(1.5);
                            for angle in [0.0, 90.0, 180.0, 270.0] {
                                let rad = angle * std::f64::consts::PI / 180.0;
                                cr.move_to(rad.cos() * 11.0, rad.sin() * 11.0);
                                cr.line_to(rad.cos() * 14.0, rad.sin() * 14.0);
                                cr.stroke().unwrap();
                            }

                            // Clock hands
                            let time_sec = elapsed_ms / 1000.0;
                            let min_angle = time_sec * 0.05; // slowly rotate
                            let hour_angle = min_angle / 12.0;

                            // Hour hand (short, thicker)
                            cr.set_line_width(2.0);
                            cr.move_to(0.0, 0.0);
                            cr.line_to(hour_angle.sin() * 7.0, -hour_angle.cos() * 7.0);
                            cr.stroke().unwrap();

                            // Minute hand (longer, thinner)
                            cr.set_line_width(1.0);
                            cr.move_to(0.0, 0.0);
                            cr.line_to(min_angle.sin() * 11.0, -min_angle.cos() * 11.0);
                            cr.stroke().unwrap();

                            // Center pin
                            cr.set_source_rgba(0.2, 0.15, 0.1, 1.0);
                            cr.arc(0.0, 0.0, 2.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();

                            cr.restore().unwrap();
                        }
                        ActiveProp::SublimeKitty => {
                            // Draw a mini retro terminal screen at the bottom right (x = 165, y = 175)
                            cr.save().unwrap();
                            let tx = 165.0;
                            let ty = 175.0;
                            cr.translate(tx, ty);

                            // Terminal glass casing (black box with rounded corners and border)
                            cr.set_source_rgba(0.1, 0.1, 0.14, 0.95);
                            let w = 75.0;
                            let h = 48.0;
                            let r = 6.0;
                            cr.new_sub_path();
                            cr.arc(w - r, r, r, -90.0 * std::f64::consts::PI / 180.0, 0.0);
                            cr.arc(w - r, h - r, r, 0.0, 90.0 * std::f64::consts::PI / 180.0);
                            cr.arc(
                                r,
                                h - r,
                                r,
                                90.0 * std::f64::consts::PI / 180.0,
                                180.0 * std::f64::consts::PI / 180.0,
                            );
                            cr.arc(
                                r,
                                r,
                                r,
                                180.0 * std::f64::consts::PI / 180.0,
                                270.0 * std::f64::consts::PI / 180.0,
                            );
                            cr.close_path();
                            cr.fill().unwrap();

                            // Terminal glow border (neon cyan)
                            cr.set_source_rgba(0.0, 0.9, 0.7, 0.7);
                            cr.set_line_width(1.5);
                            cr.new_sub_path();
                            cr.arc(w - r, r, r, -90.0 * std::f64::consts::PI / 180.0, 0.0);
                            cr.arc(w - r, h - r, r, 0.0, 90.0 * std::f64::consts::PI / 180.0);
                            cr.arc(
                                r,
                                h - r,
                                r,
                                90.0 * std::f64::consts::PI / 180.0,
                                180.0 * std::f64::consts::PI / 180.0,
                            );
                            cr.arc(
                                r,
                                r,
                                r,
                                180.0 * std::f64::consts::PI / 180.0,
                                270.0 * std::f64::consts::PI / 180.0,
                            );
                            cr.close_path();
                            cr.stroke().unwrap();

                            // Terminal header bar
                            cr.set_source_rgba(0.2, 0.2, 0.25, 0.8);
                            cr.rectangle(1.0, 1.0, w - 2.0, 8.0);
                            cr.fill().unwrap();

                            // Title dots
                            cr.set_source_rgba(1.0, 0.4, 0.4, 1.0); // red dot
                            cr.arc(6.0, 5.0, 2.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                            cr.set_source_rgba(1.0, 0.8, 0.3, 1.0); // yellow dot
                            cr.arc(11.0, 5.0, 2.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                            cr.set_source_rgba(0.3, 0.8, 0.4, 1.0); // green dot
                            cr.arc(16.0, 5.0, 2.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();

                            // Prompt string: "$ felix"
                            cr.set_source_rgba(0.0, 0.9, 0.5, 1.0);
                            cr.select_font_face(
                                "Monospace",
                                gtk4::cairo::FontSlant::Normal,
                                gtk4::cairo::FontWeight::Bold,
                            );
                            cr.set_font_size(9.0);
                            cr.move_to(6.0, 22.0);
                            cr.show_text("$ ").unwrap();

                            cr.set_source_rgba(0.9, 0.9, 0.95, 1.0);
                            cr.show_text("felix").unwrap();

                            // Blinking cursor
                            let show_cursor = (elapsed_ms / 400.0) as i32 % 2 == 0;
                            if show_cursor {
                                cr.set_source_rgba(0.0, 0.9, 0.7, 0.9);
                                cr.rectangle(48.0, 14.0, 5.0, 9.0);
                                cr.fill().unwrap();
                            }

                            // Mock output lines
                            cr.set_source_rgba(0.4, 0.6, 1.0, 0.85);
                            cr.rectangle(6.0, 28.0, 45.0, 4.0);
                            cr.fill().unwrap();

                            cr.set_source_rgba(1.0, 0.6, 0.2, 0.85);
                            cr.rectangle(6.0, 36.0, 25.0, 4.0);
                            cr.fill().unwrap();

                            cr.restore().unwrap();
                        }
                        _ => {}
                    }
                }

                // Draw a floating vinyl record next to Felix if music is playing (Dancing state)
                use crate::state::PetState;
                if current_state == PetState::Dancing && active == ActiveProp::None {
                    cr.save().unwrap();
                    let bob_y = 6.0 * (elapsed_ms / 200.0).cos();
                    let tx = 190.0;
                    let ty = 110.0 + bob_y;
                    cr.translate(tx, ty);

                    // Rotate record
                    let rot = elapsed_ms * 0.005;
                    cr.rotate(rot);

                    // Record outer ring (black)
                    cr.set_source_rgba(0.08, 0.08, 0.08, 1.0);
                    cr.arc(0.0, 0.0, 16.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.fill().unwrap();

                    // Groove lines
                    cr.set_source_rgba(0.2, 0.2, 0.2, 1.0);
                    cr.set_line_width(1.0);
                    cr.arc(0.0, 0.0, 12.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.stroke().unwrap();
                    cr.arc(0.0, 0.0, 8.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.stroke().unwrap();

                    // Center label (pink/purple)
                    cr.set_source_rgba(0.9, 0.3, 0.6, 1.0);
                    cr.arc(0.0, 0.0, 5.0, 0.0, 2.0 * std::f64::consts::PI);
                    cr.fill().unwrap();

                    // Center hole
                    cr.set_source_rgba(0.12, 0.12, 0.12, 1.0);
                    cr.arc(0.0, 0.0, 1.5, 0.0, 2.0 * std::f64::consts::PI);
                    cr.fill().unwrap();

                    cr.restore().unwrap();
                }

                // Draw particles!
                let particles = prop_particles_clone.borrow();
                for p in particles.iter() {
                    cr.save().unwrap();

                    if p.value == 50 {
                        // Floating music notes
                        cr.set_source_rgba(0.9, 0.35, 0.85, p.alpha); // Pink/purple
                        cr.select_font_face(
                            "Sans",
                            gtk4::cairo::FontSlant::Normal,
                            gtk4::cairo::FontWeight::Bold,
                        );
                        cr.set_font_size(p.size);
                        cr.move_to(p.x, p.y);
                        let sym = match ((p.x + p.y) as i32) % 4 {
                            0 => "♩",
                            1 => "♪",
                            2 => "♫",
                            _ => "♬",
                        };
                        cr.show_text(sym).unwrap();
                    } else if p.value == 60 {
                        // Regular rain falling down slanted
                        cr.set_source_rgba(0.4, 0.65, 0.9, p.alpha * 0.6);
                        cr.set_line_width(1.2);
                        cr.move_to(p.x, p.y);
                        cr.line_to(p.x + p.speed_x * 2.0, p.y + p.size);
                        cr.stroke().unwrap();
                    } else if p.value == 70 {
                        // Snowflakes
                        cr.set_source_rgba(0.95, 0.98, 1.0, p.alpha * 0.8);
                        cr.arc(p.x, p.y, p.size / 2.0, 0.0, 2.0 * std::f64::consts::PI);
                        cr.fill().unwrap();
                    } else if p.value == 80 {
                        // Exploding confetti stars
                        let colors = [
                            (1.0, 0.8, 0.0), // gold
                            (1.0, 0.3, 0.3), // red
                            (0.3, 0.8, 1.0), // cyan
                            (0.4, 1.0, 0.4), // green
                            (0.9, 0.4, 1.0), // purple
                        ];
                        let col = colors[(p.x as usize + p.y as usize) % colors.len()];
                        cr.set_source_rgba(col.0, col.1, col.2, p.alpha);

                        let px = p.x;
                        let py = p.y;
                        let sz = p.size;

                        cr.move_to(px, py - sz);
                        cr.line_to(px + sz * 0.3, py - sz * 0.3);
                        cr.line_to(px + sz, py);
                        cr.line_to(px + sz * 0.3, py + sz * 0.3);
                        cr.line_to(px, py + sz);
                        cr.line_to(px - sz * 0.3, py + sz * 0.3);
                        cr.line_to(px - sz, py);
                        cr.line_to(px - sz * 0.3, py - sz * 0.3);
                        cr.close_path();
                        cr.fill().unwrap();
                    } else {
                        match active {
                            ActiveProp::WutheringWaves => {
                                // Golden/Cyan sparkles
                                cr.set_source_rgba(
                                    if p.value % 2 == 0 { 1.0 } else { 0.0 }, // gold vs cyan
                                    if p.value % 2 == 0 { 0.85 } else { 0.9 },
                                    if p.value % 2 == 0 { 0.2 } else { 1.0 },
                                    p.alpha,
                                );

                                let px = p.x;
                                let py = p.y;
                                let sz = p.size;

                                cr.move_to(px, py - sz);
                                cr.line_to(px, py + sz);
                                cr.move_to(px - sz, py);
                                cr.line_to(px + sz, py);
                                cr.set_line_width(1.5);
                                cr.stroke().unwrap();
                            }
                            ActiveProp::Reverse1999 => {
                                // Upside-down raindrops moving upwards
                                cr.set_source_rgba(0.4, 0.75, 1.0, p.alpha * 0.7);
                                cr.set_line_width(1.0);
                                cr.move_to(p.x, p.y);
                                cr.line_to(p.x, p.y - p.size);
                                cr.stroke().unwrap();
                            }
                            ActiveProp::SublimeKitty => {
                                // Floating code symbols
                                cr.set_source_rgba(0.2, 0.85, 0.95, p.alpha);
                                cr.select_font_face(
                                    "Monospace",
                                    gtk4::cairo::FontSlant::Normal,
                                    gtk4::cairo::FontWeight::Normal,
                                );
                                cr.set_font_size(p.size);
                                cr.move_to(p.x, p.y);

                                let sym = match p.value {
                                    0 => "{}",
                                    1 => ";",
                                    2 => "fn",
                                    3 => "=>",
                                    4 => "let",
                                    5 => "git",
                                    6 => "cat",
                                    _ => "x",
                                };
                                cr.show_text(sym).unwrap();
                            }
                            _ => {}
                        }
                    }

                    cr.restore().unwrap();
                }

                cr.restore().unwrap();
            });
        }

        // 5. Create the speech bubble
        let bubble = SpeechBubble::new();

        // Checklist State and UI Box
        let tasks = Rc::new(RefCell::new(load_tasks()));

        let checklist_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(vec!["checklist-card".to_string()])
            .spacing(6)
            .valign(gtk4::Align::End)
            .visible(config.checklist_visible)
            .build();

        let checklist_title = gtk4::Label::builder()
            .label("Task Checklist 🐾")
            .css_classes(vec!["checklist-title".to_string()])
            .halign(gtk4::Align::Start)
            .build();
        checklist_card.append(&checklist_title);

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .min_content_height(100)
            .max_content_height(200)
            .propagate_natural_height(true)
            .build();

        let task_list_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();
        scrolled.set_child(Some(&task_list_box));
        checklist_card.append(&scrolled);

        let entry = gtk4::Entry::builder()
            .placeholder_text("Add a task... ✍️")
            .css_classes(vec!["checklist-entry".to_string()])
            .build();
        checklist_card.append(&entry);

        // Populate initial tasks
        {
            let list = tasks.borrow().clone();
            for item in list {
                let item_box = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(6)
                    .build();

                let check = gtk4::CheckButton::builder()
                    .active(item.completed)
                    .build();

                let label = gtk4::Label::builder()
                    .label(&item.text)
                    .halign(gtk4::Align::Start)
                    .hexpand(true)
                    .css_classes(vec!["task-label".to_string()])
                    .build();

                if item.completed {
                    label.add_css_class("task-completed");
                }

                let del_btn = gtk4::Button::builder()
                    .label("×")
                    .has_frame(false)
                    .css_classes(vec!["task-del-btn".to_string()])
                    .build();

                item_box.append(&check);
                item_box.append(&label);
                item_box.append(&del_btn);
                task_list_box.append(&item_box);

                let tasks_inner = tasks.clone();
                let tx_inner = tx.clone();
                let label_inner = label.clone();
                let item_id = item.id.clone();
                check.connect_toggled(move |btn| {
                    let active = btn.is_active();
                    let mut list = tasks_inner.borrow_mut();
                    if let Some(task) = list.iter_mut().find(|t| t.id == item_id) {
                        task.completed = active;
                        if active {
                            label_inner.add_css_class("task-completed");
                            let _ = tx_inner.send(AppEvent::TaskCompleted);
                        } else {
                            label_inner.remove_css_class("task-completed");
                        }
                    }
                    save_tasks(&list);
                });

                let tasks_inner_del = tasks.clone();
                let list_box_inner = task_list_box.clone();
                let item_box_inner = item_box.clone();
                let item_id_del = item.id.clone();
                del_btn.connect_clicked(move |_| {
                    list_box_inner.remove(&item_box_inner);
                    let mut list = tasks_inner_del.borrow_mut();
                    list.retain(|t| t.id != item_id_del);
                    save_tasks(&list);
                });
            }
        }

        // Connect entry activated callback
        {
            let tasks_clone = tasks.clone();
            let task_list_box_clone = task_list_box.clone();
            let tx_clone = tx.clone();
            entry.connect_activate(move |ent| {
                let text = ent.text().to_string();
                if !text.trim().is_empty() {
                    ent.set_text("");
                    let id = gtk4::glib::random_int().to_string();
                    let new_item = TaskItem {
                        id: id.clone(),
                        text: text.clone(),
                        completed: false,
                    };

                    let mut list = tasks_clone.borrow_mut();
                    list.push(new_item);
                    save_tasks(&list);

                    let item_box = gtk4::Box::builder()
                        .orientation(gtk4::Orientation::Horizontal)
                        .spacing(6)
                        .build();

                    let check = gtk4::CheckButton::builder()
                        .active(false)
                        .build();

                    let label = gtk4::Label::builder()
                        .label(&text)
                        .halign(gtk4::Align::Start)
                        .hexpand(true)
                        .css_classes(vec!["task-label".to_string()])
                        .build();

                    let del_btn = gtk4::Button::builder()
                        .label("×")
                        .has_frame(false)
                        .css_classes(vec!["task-del-btn".to_string()])
                        .build();

                    item_box.append(&check);
                    item_box.append(&label);
                    item_box.append(&del_btn);
                    task_list_box_clone.append(&item_box);

                    let tasks_inner = tasks_clone.clone();
                    let tx_inner = tx_clone.clone();
                    let label_inner = label.clone();
                    let item_id = id.clone();
                    check.connect_toggled(move |btn| {
                        let active = btn.is_active();
                        let mut list = tasks_inner.borrow_mut();
                        if let Some(task) = list.iter_mut().find(|t| t.id == item_id) {
                            task.completed = active;
                            if active {
                                label_inner.add_css_class("task-completed");
                                let _ = tx_inner.send(AppEvent::TaskCompleted);
                            } else {
                                label_inner.remove_css_class("task-completed");
                            }
                        }
                        save_tasks(&list);
                    });

                    let tasks_inner_del = tasks_clone.clone();
                    let list_box_inner = task_list_box_clone.clone();
                    let item_box_inner = item_box.clone();
                    let item_id_del = id.clone();
                    del_btn.connect_clicked(move |_| {
                        list_box_inner.remove(&item_box_inner);
                        let mut list = tasks_inner_del.borrow_mut();
                        list.retain(|t| t.id != item_id_del);
                        save_tasks(&list);
                    });
                }
            });
        }

        // 7. Vertical layout with GTK::Overlay for pet + props
        let overlay = gtk4::Overlay::new();
        overlay.set_width_request(config.pet.size);
        overlay.set_height_request(config.pet.size);

        overlay.set_child(Some(&drawing_area));
        overlay.add_overlay(&prop_overlay);

        let vbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::End)
            .width_request(config.pet.size)
            .build();

        vbox.append(&bubble.container);
        vbox.append(&overlay);

        // 8. Horizontal layout to put checklist next to pet
        let hbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk4::Align::End)
            .valign(gtk4::Align::End)
            .height_request(config.pet.size)
            .build();

        hbox.append(&checklist_card);
        hbox.append(&vbox);
        window.set_child(Some(&hbox));

        // 9. Inject CSS for transparency, speech bubble, and checklist
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(
            "
            /* Force transparency on window containers, drawing areas, boxes and overlays using high specificity class chains */
            window.pet-window-class,
            window.pet-window-class box,
            window.pet-window-class drawingarea,
            window.pet-window-class overlay {
                background-color: rgba(0,0,0,0);
                background-image: none;
                box-shadow: none;
                border: none;
            }
            
            /* Fallback generic overrides */
            window, window.background, .background, .csd, .ssd, .titlebar, headerbar, drawingarea, box, overlay {
                background-color: rgba(0,0,0,0);
                background-image: none;
                box-shadow: none;
                border: none;
            }
            
            .bubble-body {
                background-color: rgba(30, 30, 40, 0.95);
                border-radius: 12px;
                border: 1px solid rgba(255, 255, 255, 0.15);
                padding: 8px 12px;
                box-shadow: 0 4px 10px rgba(0, 0, 0, 0.3);
            }
            .bubble-body label {
                color: #ffffff;
                font-family: 'Inter', 'Outfit', sans-serif;
                font-size: 13px;
                font-weight: 500;
            }
            .checklist-card {
                background-color: rgba(25, 25, 35, 0.93);
                border-radius: 12px;
                border: 1px solid rgba(255, 255, 255, 0.15);
                padding: 12px;
                min-width: 220px;
                box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
            }
            .checklist-title {
                color: #ffa500;
                font-family: 'Inter', 'Outfit', sans-serif;
                font-size: 14px;
                font-weight: 700;
                margin-bottom: 6px;
            }
            .task-completed {
                text-decoration: line-through;
                color: rgba(255, 255, 255, 0.45);
            }
            .task-label {
                color: #ffffff;
                font-family: 'Inter', sans-serif;
                font-size: 13px;
            }
            .checklist-entry {
                background-color: rgba(255, 255, 255, 0.08);
                border: 1px solid rgba(255, 255, 255, 0.15);
                border-radius: 6px;
                color: white;
                padding: 6px;
                font-size: 12px;
                margin-top: 4px;
            }
            .task-del-btn {
                color: #ff5555;
                font-weight: bold;
                font-size: 16px;
                padding: 0px 4px;
                background-color: rgba(0,0,0,0);
                border: none;
                margin-left: 4px;
            }
            .task-del-btn:hover {
                color: #ff2222;
                background-color: rgba(255, 85, 85, 0.15);
                border-radius: 4px;
            }
            ",
        );

        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        }

        let window_obj = Self {
            window,
            bubble,
            drawing_area,
            prop_overlay,
            frame_coords,
            pet_scale,
            pet_size,
            hearts,
            current_pet_state,
            active_prop,
            prop_particles,
            checklist_card,
            tasks,
            current_weather,
            current_routine,
        };

        window_obj
    }

    pub fn update_config(&self, config: &Config) {
        update_window_properties(&self.window, config);
        self.pet_scale.set(config.pet.scale);
        self.pet_size.set(config.pet.size);
        self.drawing_area
            .set_size_request(config.pet.size, config.pet.size);
        self.prop_overlay
            .set_size_request(config.pet.size, config.pet.size);
        self.checklist_card.set_visible(config.checklist_visible);
        *self.current_weather.borrow_mut() = config.weather.clone();
        self.window.queue_resize();
        self.drawing_area.queue_draw();
        self.prop_overlay.queue_draw();
    }

    pub fn set_position(&self, x: i32, y: i32) {
        self.window.set_anchor(Edge::Bottom, true);
        self.window.set_anchor(Edge::Left, true);
        self.window.set_anchor(Edge::Top, false);
        self.window.set_anchor(Edge::Right, false);

        self.window.set_margin(Edge::Left, x);
        self.window.set_margin(Edge::Bottom, y);
    }

    pub fn reset_position_to_anchor(&self, config: &Config) {
        update_window_properties(&self.window, config);
    }
}

pub fn update_window_properties(window: &gtk4::ApplicationWindow, config: &Config) {
    window.set_anchor(Edge::Bottom, config.anchor.edge_bottom);
    window.set_anchor(Edge::Right, config.anchor.edge_right);
    window.set_anchor(Edge::Top, config.anchor.edge_top);
    window.set_anchor(Edge::Left, config.anchor.edge_left);

    window.set_margin(Edge::Bottom, config.anchor.margin_bottom);
    window.set_margin(Edge::Right, config.anchor.margin_right);
    window.set_margin(Edge::Top, config.anchor.margin_top);
    window.set_margin(Edge::Left, config.anchor.margin_left);
}
