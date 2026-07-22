#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveWindowGeometry {
    pub address: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub class: String,
    pub title: String,
    pub floating: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum InputEvent {
    Typing,
    CursorPos { x: f64, y: f64 },
    ActiveWindow { class: String, title: String },
    ActiveWindowGeom(ActiveWindowGeometry),
    ActiveWindowGeomNone,
}

#[derive(Debug, Clone, Copy)]
pub enum TrayAction {
    TogglePause,
    ResetTimer,
    ReloadConfig,
    ToggleChecklist,
    Quit,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum AppEvent {
    Input(InputEvent),
    Tray(TrayAction),
    Tick,
    Petting,
    TrackChanged(String),
    TaskCompleted,
    WeatherChanged(String),
    WorkspaceChanged,
}
