use crate::event::{AppEvent, InputEvent};
use evdev::{Device, KeyCode};
use std::fs;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

pub fn spawn_keyboard_tracker(tx: Sender<AppEvent>) {
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
                        // A device is likely a keyboard if it supports standard key event codes
                        if device.supported_keys().map(|keys| keys.contains(KeyCode::KEY_A)).unwrap_or(false) {
                            println!("Detected keyboard device: {:?} ({})", path, device.name().unwrap_or("Unnamed"));
                            keyboard_paths.push(path);
                        }
                    }
                }
            }
        }

        if keyboard_paths.is_empty() {
            eprintln!("No keyboard devices detected in /dev/input. Typing animations will be disabled.");
            return;
        }

        for path in keyboard_paths {
            let tx_clone = tx.clone();
            thread::spawn(move || {
                loop {
                    match Device::open(&path) {
                        Ok(mut device) => {
                            loop {
                                match device.fetch_events() {
                                    Ok(events) => {
                                        for event in events {
                                            if event.event_type() == evdev::EventType::KEY {
                                                // value == 1: Press, value == 2: Hold/Repeat
                                                if event.value() == 1 || event.value() == 2 {
                                                    let _ = tx_clone.send(AppEvent::Input(InputEvent::Typing));
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        break;
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            thread::sleep(Duration::from_secs(5));
                        }
                    }
                }
            });
        }
    });
}