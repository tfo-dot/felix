#[cfg(all(target_os = "linux", feature = "hyprland"))]
pub mod hyprland;
pub mod keyboard;
#[cfg(target_os = "windows")]
pub mod windows;
