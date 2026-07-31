#[cfg(target_os = "linux")]
use evdev::{Device, KeyCode};
use crate::event::{AppEvent, InputEvent};
use std::sync::mpsc::Sender;

#[cfg(target_os = "linux")]
pub fn spawn_keyboard_tracker(tx: Sender<AppEvent>) {
    use std::fs;
    use std::thread;
    use std::time::Duration;

    thread::spawn(move || {
        let mut keyboard_paths = Vec::new();

        // Scan for keyboards
        if let Ok(entries) = fs::read_dir("/dev/input") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("event") {
                    let path = entry.path();
                    if let Ok(device) = Device::open(&path) {
                        if device
                            .supported_keys()
                            .map(|keys| keys.contains(KeyCode::KEY_A))
                            .unwrap_or(false)
                        {
                            log::info!(
                                "Detected keyboard device: {:?} ({})",
                                path,
                                device.name().unwrap_or("Unnamed")
                            );
                            keyboard_paths.push(path);
                        }
                    }
                }
            }
        }

        if keyboard_paths.is_empty() {
            log::warn!(
                "No keyboard devices detected in /dev/input. Typing animations will be disabled."
            );
            return;
        }

        for path in keyboard_paths {
            let tx_clone = tx.clone();
            thread::spawn(move || {
                loop {
                    match Device::open(&path) {
                        Ok(mut device) => loop {
                            match device.fetch_events() {
                                Ok(events) => {
                                    for event in events {
                                        if event.event_type() == evdev::EventType::KEY {
                                            let _ =
                                                tx_clone.send(AppEvent::Input(InputEvent::Typing));
                                        }
                                    }
                                }
                                Err(_) => {
                                    break;
                                }
                            }
                        },
                        Err(_) => {
                            thread::sleep(Duration::from_secs(5));
                        }
                    }
                }
            });
        }
    });
}

#[cfg(not(target_os = "linux"))]
pub fn spawn_keyboard_tracker(_tx: Sender<AppEvent>) {
    log::warn!("Keyboard tracking is not supported on this platform. Typing animations will be disabled.");
}
