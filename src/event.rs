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
    DragBegin,
    DragUpdate { offset_x: f64, offset_y: f64 },
    DragEnd,
}

#[derive(Debug, Clone, Copy)]
pub enum TrayAction {
    TogglePause,
    ResetTimer,
    ReloadConfig,
    ToggleChecklist,
    TogglePetVisibility,
    Quit,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum AppEvent {
    Input(InputEvent),
    Tray(TrayAction),
    Tick,
    Petting,
    Feed { x: f64, y: f64 },
    TrackChanged(String),
    TaskCompleted,
    WeatherChanged(WeatherCondition),
    WorkspaceChanged,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WeatherState {
    pub entity_id: String,
    pub state: WeatherCondition,
    pub attributes: Attributes,
    pub last_changed: String,
    pub last_reported: String,
    pub last_updated: String,
    pub context: Context,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum WeatherCondition {
    #[serde(rename = "clear-night")]
    ClearNight,
    #[serde(rename = "cloudy")]
    Cloudy,
    #[serde(rename = "fog")]
    Fog,
    #[serde(rename = "hail")]
    Hail,
    #[serde(rename = "lightning")]
    Lightning,
    #[serde(rename = "lightning-rainy")]
    LightningRainy,
    #[serde(rename = "partlycloudy")]
    PartlyCloudy,
    #[serde(rename = "pouring")]
    Pouring,
    #[serde(rename = "rainy")]
    Rainy,
    #[serde(rename = "snowy")]
    Snowy,
    #[serde(rename = "snowy-rainy")]
    SnowyRainy,
    #[serde(rename = "sunny")]
    Sunny,
    #[serde(rename = "windy")]
    Windy,
    #[serde(rename = "windy-variant")]
    WindyVariant,
    #[serde(rename = "exceptional")]
    Exceptional,
    // Acts as a catch-all for any unmapped strings to prevent deserialization panics
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Attributes {
    pub temperature: f64,
    pub dew_point: f64,
    pub temperature_unit: String,
    pub humidity: f64,
    pub cloud_coverage: f64,
    pub uv_index: f64,
    pub pressure: f64,
    pub pressure_unit: String,
    pub wind_bearing: f64,
    pub wind_speed: f64,
    pub wind_speed_unit: String,
    pub visibility_unit: String,
    pub precipitation_unit: String,
    pub attribution: String,
    pub friendly_name: String,
    pub supported_features: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Context {
    pub id: String,
    pub parent_id: Option<String>,
    pub user_id: Option<String>,
}
