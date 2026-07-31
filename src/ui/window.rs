use crate::config::{ColorMode, Config};
use crate::event::{AppEvent, InputEvent, TrayAction};
use crate::state::RoutineState;
use crate::ui::bubble::SpeechBubble;
use cairo::{Context, Format, ImageSurface, LinearGradient, RadialGradient};
use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::prelude::*;
#[cfg(target_os = "linux")]
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

#[cfg(not(target_os = "linux"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Bottom,
    Left,
    Right,
    Top,
}

#[cfg(not(target_os = "linux"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardMode {
    None,
}

#[cfg(not(target_os = "linux"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Overlay,
}

#[cfg(not(target_os = "linux"))]
pub trait LayerShell {
    fn init_layer_shell(&self);
    fn set_layer(&self, layer: Layer);
    fn set_keyboard_mode(&self, mode: KeyboardMode);
    fn set_namespace(&self, namespace: Option<&str>);
    fn set_exclusive_zone(&self, zone: i32);
    fn set_anchor(&self, edge: Edge, anchor: bool);
    fn set_margin(&self, edge: Edge, margin: i32);
    fn set_monitor(&self, monitor: Option<&gtk4::gdk::Monitor>);
}

#[cfg(not(target_os = "linux"))]
impl LayerShell for gtk4::ApplicationWindow {
    fn init_layer_shell(&self) {}
    fn set_layer(&self, _layer: Layer) {}
    fn set_keyboard_mode(&self, _mode: KeyboardMode) {}
    fn set_namespace(&self, _namespace: Option<&str>) {}
    fn set_exclusive_zone(&self, _zone: i32) {}
    fn set_anchor(&self, _edge: Edge, _anchor: bool) {}
    fn set_margin(&self, _edge: Edge, _margin: i32) {}
    fn set_monitor(&self, _monitor: Option<&gtk4::gdk::Monitor>) {}
}
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs::File;
use std::io::Cursor;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveProp {
    None,
    WutheringWaves,
    Reverse1999,
    SublimeKitty,
    VSCode,
    Browser,
    Discord,
    Minecraft,
    Steam,
    Spotify,
}

#[derive(Clone, Copy, Debug)]
pub enum PropId {
    MusicNote,
    Rain,
    Threat,
    Snowflake,
    Crumb,
    Code(u8),
    Spark(bool),
}

#[derive(Clone, Copy, Debug)]
pub struct PropParticle {
    pub x: f64,
    pub y: f64,
    pub alpha: f64,
    pub size: f64,
    pub speed_x: f64,
    pub speed_y: f64,
    pub time: f64,
    pub id: PropId,
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
    pub current_weather: Rc<RefCell<crate::event::WeatherCondition>>,
    pub current_routine: Rc<RefCell<RoutineState>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Assets {
    x: i32,
    y: i32,
    name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PetMeta {
    x: f64,
    y: f64,
    rotation: f64,
    tile_col: i64,
    tile_row: i64,
    tile_index: i64,
}

const PET_JSON_META: &[u8] = include_bytes!("../../assets/pet_meta.json");
const ASSETS_SPRITESHEET_BYTES: &[u8] = include_bytes!("../../assets/assets_spritesheet.png");
const ASSETS_SPRITESHEET_JSON_BYTES: &[u8] = include_bytes!("../../assets/assets_spritesheet.json");

fn load_surface(file_path: &str, embedded_bytes: &'static [u8]) -> Option<ImageSurface> {
    // Try local file first
    if let Ok(mut file) = File::open(file_path) {
        log::info!("Loading asset from filesystem: {}", file_path);
        match ImageSurface::create_from_png(&mut file) {
            Ok(surf) => return Some(surf),
            Err(e) => {
                log::error!("Failed to parse PNG from {}: {:?}", file_path, e);
            }
        }
    }

    // Fallback to embedded bytes
    log::info!("Loading embedded asset for {}", file_path);
    let mut cursor = Cursor::new(embedded_bytes);
    match ImageSurface::create_from_png(&mut cursor) {
        Ok(surf) => Some(surf),
        Err(e) => {
            log::error!("Failed to parse embedded PNG for {}: {:?}", file_path, e);
            None
        }
    }
}

fn set_color_source(ctx: &Context, mode: &ColorMode, w: f64, h: f64) {
    match mode {
        ColorMode::Static { r, g, b } => {
            ctx.set_source_rgb(*r / 255.0, *g / 255.0, *b / 255.0);
        }
        ColorMode::Linear { angle_deg, stops } => {
            let angle = angle_deg.to_radians();
            let (cx, cy) = (w / 2.0, h / 2.0);
            let len = w.hypot(h) / 2.0;
            let (dx, dy) = (angle.cos() * len, angle.sin() * len);
            let grad = LinearGradient::new(cx - dx, cy - dy, cx + dx, cy + dy);
            for (offset, r, g, b) in stops {
                grad.add_color_stop_rgb(*offset, *r, *g, *b);
            }
            ctx.set_source(&grad).unwrap();
        }
        ColorMode::Radial { stops } => {
            let grad = RadialGradient::new(w / 2.0, h / 2.0, 0.0, w / 2.0, h / 2.0, w.max(h) / 2.0);
            for (offset, r, g, b) in stops {
                grad.add_color_stop_rgb(*offset, *r, *g, *b);
            }
            ctx.set_source(&grad).unwrap();
        }
    }
}

fn colorize_cell(sheet: &Pixbuf, col: i32, row: i32, mode: &ColorMode) -> ImageSurface {
    let surface = ImageSurface::create(Format::ARgb32, 1024, 1024).unwrap();
    let ctx = Context::new(&surface).unwrap();

    ctx.set_source_pixbuf(sheet, -(col * 256) as f64, -(row * 256) as f64);
    let mask_pattern = ctx.source();

    set_color_source(&ctx, mode, 256 as f64, 256 as f64);

    ctx.mask(&mask_pattern).unwrap();

    surface
}

fn compose_sheet(layers: &[ImageSurface]) -> ImageSurface {
    let out = ImageSurface::create(Format::ARgb32, 1024, 1024).unwrap();
    let ctx = Context::new(&out).unwrap();

    for layer in layers {
        ctx.set_operator(cairo::Operator::Over);
        ctx.set_source_surface(layer, 0.0, 0.0).unwrap();
        ctx.paint().unwrap();
    }

    out
}

fn build_colorized_sheet(sheet: &Pixbuf, mode: &ColorMode) -> ImageSurface {
    let out = ImageSurface::create(Format::ARgb32, 1024, 1024).unwrap();
    let ctx = Context::new(&out).unwrap();

    for row in 0..4 {
        for col in 0..4 {
            let cell = colorize_cell(sheet, col, row, mode);
            ctx.set_source_surface(&cell, (col * 256) as f64, (row * 256) as f64)
                .unwrap();
            ctx.paint().unwrap();
        }
    }

    out
}

impl PetWindow {
    pub fn new(app: &gtk4::Application, config: &Config, tx: Sender<AppEvent>) -> Self {
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Felix Desktop Pet")
            .build();

        window.add_css_class("pet-window-class");

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
            #[cfg(target_os = "windows")]
            {
                let title: Vec<u16> = "Felix Desktop Pet\0".encode_utf16().collect();
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::*;
                    let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
                    if hwnd != 0 {
                        let mut style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
                        style |= WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TOOLWINDOW;
                        SetWindowLongW(hwnd, GWL_EXSTYLE, style as i32);
                        
                        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, 
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED);
                    }
                }
            }
        });

        update_window_properties(&window, config);

        let mut buffers = vec![];

        for layer in &config.texture.clone() {
            let path = format!("assets/layers/{}.png", layer.layer);

            log::info!("Loading asset from filesystem: {}", path);

            let pb = Pixbuf::from_file(path).unwrap();

            buffers.push(build_colorized_sheet(&pb, &layer.color));
        }

        let surface = Some(compose_sheet(&buffers));

        let assets_surface =
            load_surface("assets/assets_spritesheet.png", ASSETS_SPRITESHEET_BYTES);

        let assets_meta: Vec<Assets> =
            serde_json::from_slice(ASSETS_SPRITESHEET_JSON_BYTES).expect("Should be a valid json");

        let pet_meta: HashMap<String, Vec<PetMeta>> =
            serde_json::from_slice(PET_JSON_META).expect("Should be a valid json");

        let frame_coords = Rc::new(RefCell::new((0, 0)));
        let pet_scale = Rc::new(Cell::new(config.pet.scale));
        let pet_size = Rc::new(Cell::new(config.pet.size));
        let hearts = Rc::new(RefCell::new(Vec::<Heart>::new()));
        let current_pet_state = Rc::new(Cell::new(crate::state::PetState::Idle));
        let active_prop = Rc::new(Cell::new(ActiveProp::None));
        let current_weather = Rc::new(RefCell::new(crate::event::WeatherCondition::Unknown));
        let current_routine = Rc::new(RefCell::new(RoutineState::None));
        let prop_particles = Rc::new(RefCell::new(Vec::<PropParticle>::new()));

        let drawing_area = gtk4::DrawingArea::builder()
            .width_request(config.pet.size)
            .height_request(config.pet.size)
            .halign(gtk4::Align::Center)
            .build();

        let prop_overlay = gtk4::DrawingArea::builder()
            .width_request(config.pet.size)
            .height_request(config.pet.size)
            .halign(gtk4::Align::Center)
            .can_target(false)
            .build();

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

        let dragged_threshold_crossed = Rc::new(Cell::new(false));

        let drag_gesture = gtk4::GestureDrag::new();
        let tx_drag = tx.clone();
        let drag_threshold_clone = dragged_threshold_crossed.clone();
        drag_gesture.connect_drag_begin(move |_, _, _| {
            drag_threshold_clone.set(false);
            let _ = tx_drag.send(AppEvent::Input(InputEvent::DragBegin));
        });

        let tx_drag_update = tx.clone();
        let drag_threshold_clone2 = dragged_threshold_crossed.clone();
        drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
            if offset_x.abs() > 4.0 || offset_y.abs() > 4.0 {
                drag_threshold_clone2.set(true);
            }
            let _ = tx_drag_update.send(AppEvent::Input(InputEvent::DragUpdate {
                offset_x,
                offset_y,
            }));
        });

        let tx_drag_end = tx.clone();
        drag_gesture.connect_drag_end(move |_, _offset_x, _offset_y| {
            let _ = tx_drag_end.send(AppEvent::Input(InputEvent::DragEnd));
        });
        drawing_area.add_controller(drag_gesture);

        let gesture_click = gtk4::GestureClick::new();
        gesture_click.set_button(0);
        let tx_click = tx.clone();
        let gesture_click_clone = gesture_click.clone();
        let drag_threshold_clone3 = dragged_threshold_crossed.clone();
        gesture_click.connect_released(move |_, n_press, x, y| {
            if drag_threshold_clone3.get() {
                return;
            }
            let button = gesture_click_clone.current_button();
            if button == 1 {
                if n_press == 1 {
                    let _ = tx_click.send(AppEvent::Tray(TrayAction::TogglePause));
                } else if n_press == 2 {
                    let _ = tx_click.send(AppEvent::Tray(TrayAction::ResetTimer));
                }
            } else if button == 2 {
                let _ = tx_click.send(AppEvent::Feed { x, y });
            }
        });
        drawing_area.add_controller(gesture_click);

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
            let assets_surface_clone = assets_surface.as_ref().unwrap().clone();
            let assets_meta_clone = assets_meta.clone();
            let pet_meta_clone = pet_meta.clone();

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

                let mut offset_y = 0.0;
                let mut rotation = 0.0;
                let mut portal_scale = 1.0;
                let mut portal_opacity = 1.0;

                let frame_idx = (fx / 256) as u32;

                use crate::state::PetState;
                match current_state {
                    PetState::WalkingLeft | PetState::WalkingRight => {
                        offset_y = 6.0 * (elapsed_ms / 120.0).sin().abs();
                        rotation = 0.06 * (elapsed_ms / 100.0).sin();
                    }
                    PetState::Dancing => {
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
                            0.0,
                            0.0,
                            vortex_radius * 0.2,
                            0.0,
                            0.0,
                            vortex_radius,
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
                            let angle =
                                (i as f64 * std::f64::consts::PI / 2.0) + (elapsed_ms * 0.005);
                            let sx = angle.cos() * (vortex_radius * 0.6);
                            let sy = angle.sin() * (vortex_radius * 0.6);
                            cr.arc(sx, sy, 3.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                        }
                    }
                    cr.restore().unwrap();
                }

                if rotation != 0.0 || offset_y != 0.0 || portal_scale != 1.0 {
                    cr.translate(128.0, 160.0 + offset_y);
                    cr.scale(portal_scale, portal_scale);
                    cr.rotate(rotation);
                    cr.translate(-128.0, -160.0);
                }

                cr.rectangle(0.0, 0.0, 256.0, 256.0);
                cr.clip();

                cr.set_source_surface(&surf_clone, -fx as f64, -fy as f64)
                    .unwrap();
                cr.set_operator(cairo::Operator::Over);
                cr.paint_with_alpha(portal_opacity).unwrap();

                let hour = gtk4::glib::DateTime::now_local()
                    .map(|dt| dt.hour())
                    .unwrap_or(12);

                let draw_asset =
                    |asset_id: &str, attach_location: &str, anchor_x: f64, anchor_y: f64| {
                        let asset = assets_meta_clone
                            .iter()
                            .find(|a| a.name == asset_id)
                            .unwrap();

                        let entry = pet_meta_clone
                            .get(attach_location)
                            .unwrap()
                            .get(((fx / 256) + ((fy * 4) / 256)) as usize);

                        if entry.is_some() {
                            let entry = entry.unwrap();

                            let scale = match attach_location {
                                "head" => 0.3,
                                "eyes" => 0.5,
                                "neck" => 0.45,
                                "left_paw" => 0.5,
                                _ => 1.0,
                            };

                            cr.save().unwrap();

                            cr.translate(entry.x, entry.y);
                            cr.rotate(entry.rotation);
                            cr.scale(scale, scale);
                            cr.translate(-anchor_x, -anchor_y);

                            cr.set_source_surface(
                                &assets_surface_clone,
                                -asset.x as f64,
                                -asset.y as f64,
                            )
                            .unwrap();
                            cr.set_operator(cairo::Operator::Over);

                            cr.rectangle(0.0, 0.0, 256.0, 256.0);
                            cr.clip();
                            cr.paint().unwrap();

                            cr.restore().unwrap();
                        }
                    };

                if (hour >= 21 || hour < 6) && current_state == PetState::Sleeping {
                    draw_asset("sleeping_hat", "head", 128.0, 220.0);
                }

                match *current_weather_clone.borrow() {
                    crate::event::WeatherCondition::Sunny => {
                        draw_asset("sunglasses", "eyes", 128.0, 128.0);
                    }
                    crate::event::WeatherCondition::Snowy => {
                        draw_asset("scarf", "neck", 128.0, 90.0);
                    }
                    crate::event::WeatherCondition::Rainy => {
                        draw_asset("umbrella", "left_space", 134.0, 222.0);
                    }
                    _ => (),
                }

                let routine = current_routine_clone.borrow().clone();
                if current_state == PetState::Idle {
                    match routine {
                        RoutineState::Coffee => {
                            draw_asset("coffee_cup", "left_paw", 143.0, 143.0);
                        }
                        RoutineState::Lunch => {
                            draw_asset("sandwich", "left_paw", 128.0, 208.0);
                        }
                        RoutineState::Reading => {
                            draw_asset("book", "left_paw", 128.0, 163.0);
                        }
                        _ => {}
                    }
                }

                cr.restore().unwrap();

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

        {
            let active_prop_clone = active_prop.clone();
            let prop_particles_clone = prop_particles.clone();
            let pet_scale_clone = pet_scale.clone();
            let pet_size_clone = pet_size.clone();
            let current_pet_state_clone2 = current_pet_state.clone();
            let assets_surface_clone = assets_surface.unwrap().clone();
            let assets_meta_clone = assets_meta.clone();
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

                let draw_asset = |asset_id: &str,
                                  pos_x: f64,
                                  pos_y: f64,
                                  anchor_x: f64,
                                  anchor_y: f64| {
                    let asset = assets_meta_clone
                        .iter()
                        .find(|a| a.name == asset_id)
                        .unwrap();

                    cr.save().unwrap();

                    cr.translate(pos_x, pos_y);
                    cr.scale(0.2, 0.2);
                    cr.translate(-anchor_x, -anchor_y);

                    cr.set_source_surface(&assets_surface_clone, -asset.x as f64, -asset.y as f64)
                        .unwrap();
                    cr.set_operator(cairo::Operator::Over);

                    cr.rectangle(0.0, 0.0, 256.0, 256.0);
                    cr.clip();
                    cr.paint().unwrap();

                    cr.restore().unwrap();
                };

                if active != ActiveProp::None {
                    match active {
                        ActiveProp::WutheringWaves => {
                            let bob_y = 5.0 * (elapsed_ms / 300.0).sin();

                            draw_asset("wuthering_waves_gourd", 40.0, 90.0 + bob_y, 0.0, 0.0);

                            let tx = 185.0;
                            let ty = 85.0 + bob_y;

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
                        ActiveProp::Reverse1999 => {
                            let bob_y = 4.0 * (elapsed_ms / 250.0).cos();

                            draw_asset("reverse1999_clock", 40.0, 90.0 + bob_y, 128.0, 140.0);
                        }
                        ActiveProp::SublimeKitty => {
                            draw_asset("terminal", 165.0, 175.0, 128.0, 128.0);
                        }
                        ActiveProp::VSCode => {
                            let bob_y = 4.0 * (elapsed_ms / 300.0).sin();

                            draw_asset("vscode", 200.0, 85.0 + bob_y, 128.0, 128.0);
                        }
                        ActiveProp::Browser => {
                            let bob_y = 4.0 * (elapsed_ms / 350.0).sin();

                            draw_asset("browser", 200.0, 85.0 + bob_y, 128.0, 128.0);
                        }
                        ActiveProp::Discord => {
                            draw_asset("discord", 200.0, 100.0, 128.0, 128.0);
                        }
                        ActiveProp::Minecraft => {
                            let bob_y = 3.0 * (elapsed_ms / 400.0).sin();

                            draw_asset("minecraft", 200.0, 85.0 + bob_y, 128.0, 128.0);
                        }
                        ActiveProp::Steam => {
                            let bob_y = 4.0 * (elapsed_ms / 300.0).sin();

                            draw_asset("steam", 200.0, 85.0 + bob_y, 128.0, 128.0);
                        }
                        ActiveProp::Spotify => {
                            let bob_y = 4.0 * (elapsed_ms / 250.0).sin();

                            draw_asset("spotify", 200.0, 85.0 + bob_y, 128.0, 128.0);
                        }
                        _ => {}
                    }
                }

                // Draw a floating vinyl record next to Felix if music is playing (Dancing state)
                use crate::state::PetState;
                if current_state == PetState::Dancing && active == ActiveProp::None {
                    let bob_y = 6.0 * (elapsed_ms / 200.0).cos();

                    draw_asset("record", 200.0, 110.0 + bob_y, 128.0, 128.0);
                }

                let particles = prop_particles_clone.borrow();
                for p in particles.iter() {
                    cr.save().unwrap();

                    match p.id {
                        PropId::MusicNote => {
                            cr.set_source_rgba(0.9, 0.35, 0.85, p.alpha);
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
                        }
                        PropId::Rain => {
                            cr.set_source_rgba(0.4, 0.65, 0.9, p.alpha * 0.6);
                            cr.set_line_width(1.2);
                            cr.move_to(p.x, p.y);
                            cr.line_to(p.x + p.speed_x * 2.0, p.y + p.size);
                            cr.stroke().unwrap();
                        }
                        PropId::Snowflake => {
                            cr.set_source_rgba(0.95, 0.98, 1.0, p.alpha * 0.8);
                            cr.arc(p.x, p.y, p.size / 2.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                        }
                        PropId::Threat => {
                            cr.save().unwrap();
                            cr.set_source_rgba(0.9, 0.5, 0.2, p.alpha); // Orange/brown fish
                            let px = p.x;
                            let py = p.y;
                            let sz = p.size;
                            cr.translate(px, py);
                            cr.scale(1.0, 0.6);
                            cr.arc(0.0, 0.0, sz / 2.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                            cr.scale(1.0, 1.0 / 0.6); // reset scale
                            cr.move_to(-sz / 2.0, 0.0);
                            cr.line_to(-sz, -sz / 3.0);
                            cr.line_to(-sz, sz / 3.0);
                            cr.close_path();
                            cr.fill().unwrap();
                            cr.set_source_rgba(1.0, 1.0, 1.0, p.alpha);
                            cr.arc(sz / 4.0, -sz / 8.0, 1.5, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                            cr.restore().unwrap();
                        }
                        PropId::Crumb => {
                            cr.save().unwrap();
                            cr.set_source_rgba(0.9, 0.6, 0.3, p.alpha);
                            cr.arc(p.x, p.y, p.size, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                            cr.restore().unwrap();
                        }
                        _ => (),
                    }

                    match active {
                        ActiveProp::WutheringWaves => {
                            let (r, g, b): (f64, f64, f64) = if let PropId::Spark(b) = p.id {
                                if b { (1.0, 0.85, 0.2) } else { (0.0, 0.9, 1.0) }
                            } else {
                                (0.0, 0.9, 1.0)
                            };

                            cr.set_source_rgba(r, g, b, p.alpha);

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
                            cr.set_source_rgba(0.4, 0.75, 1.0, p.alpha * 0.7);
                            cr.set_line_width(1.0);
                            cr.move_to(p.x, p.y);
                            cr.line_to(p.x, p.y - p.size);
                            cr.stroke().unwrap();
                        }
                        ActiveProp::SublimeKitty => {
                            cr.set_source_rgba(0.2, 0.85, 0.95, p.alpha);
                            cr.select_font_face(
                                "Monospace",
                                gtk4::cairo::FontSlant::Normal,
                                gtk4::cairo::FontWeight::Normal,
                            );
                            cr.set_font_size(p.size);
                            cr.move_to(p.x, p.y);

                            let sym = if let PropId::Code(cid) = p.id {
                                match cid % 8 {
                                    0 => "{}",
                                    1 => ";",
                                    2 => "fn",
                                    3 => "=>",
                                    4 => "let",
                                    5 => "git",
                                    6 => "cat",
                                    _ => "x",
                                }
                            } else {
                                "cat"
                            };
                            cr.show_text(sym).unwrap();
                        }
                        ActiveProp::VSCode => {
                            let colors = [
                                (0.2, 0.6, 1.0),
                                (1.0, 0.6, 0.2),
                                (1.0, 0.8, 0.2),
                                (0.4, 0.8, 0.4),
                            ];
                            let col = colors[(p.x as usize + p.y as usize) % colors.len()];
                            cr.set_source_rgba(col.0, col.1, col.2, p.alpha);
                            cr.arc(p.x, p.y, p.size / 2.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                        }
                        ActiveProp::Browser => {
                            cr.set_source_rgba(0.1, 0.7, 1.0, p.alpha);
                            cr.select_font_face(
                                "Monospace",
                                gtk4::cairo::FontSlant::Normal,
                                gtk4::cairo::FontWeight::Bold,
                            );
                            cr.set_font_size(p.size);
                            cr.move_to(p.x, p.y);

                            let sym = match ((p.x + p.y) as i32) % 2 {
                                0 => "0",
                                1 => "1",
                                _ => "0",
                            };
                            cr.show_text(sym).unwrap();
                        }
                        ActiveProp::Discord => {
                            cr.set_source_rgba(0.3, 0.8, 0.5, p.alpha * 0.8);
                            cr.arc(p.x, p.y, p.size / 2.0, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                        }
                        ActiveProp::Minecraft => {
                            cr.set_source_rgba(0.4, 0.75, 0.3, p.alpha);
                            cr.rectangle(p.x, p.y, p.size, p.size);
                            cr.fill().unwrap();
                        }
                        ActiveProp::Steam => {
                            cr.set_source_rgba(0.9, 0.9, 0.95, p.alpha * 0.4);
                            cr.arc(p.x, p.y, p.size, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().unwrap();
                        }
                        ActiveProp::Spotify => {
                            cr.set_source_rgba(0.1, 0.8, 0.35, p.alpha);
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
                        }
                        _ => {}
                    }
                }

                cr.restore().unwrap();
            });
        }

        let bubble = SpeechBubble::new();

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
            .height_request(config.pet.size)
            .build();

        vbox.append(&bubble.container);
        vbox.append(&overlay);

        let hbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk4::Align::End)
            .valign(gtk4::Align::End)
            .height_request(config.pet.size)
            .width_request(config.pet.size)
            .build();

        hbox.append(&vbox);
        window.set_child(Some(&hbox));

        let provider = gtk4::CssProvider::new();
        provider.load_from_data(
            "
            window.pet-window-class,
            window.pet-window-class box,
            window.pet-window-class drawingarea,
            window.pet-window-class overlay {
                background-color: rgba(0,0,0,0);
                background-image: none;
                box-shadow: none;
                border: none;
            }
            
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
        *self.current_weather.borrow_mut() = crate::event::WeatherCondition::Unknown;
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
