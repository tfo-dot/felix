use crate::event::{ActiveWindowGeometry, AppEvent, InputEvent};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

fn get_hyprland_socket_path(socket_name: &str) -> std::io::Result<PathBuf> {
    let runtime_dir =
        std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());

    if let Ok(sig) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        let p = PathBuf::from(&runtime_dir)
            .join("hypr")
            .join(&sig)
            .join(socket_name);
        if p.exists() {
            return Ok(p);
        }
        let p_tmp = PathBuf::from("/tmp/hypr").join(&sig).join(socket_name);
        if p_tmp.exists() {
            return Ok(p_tmp);
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Hyprland socket {} not found", socket_name),
    ))
}

pub fn is_hyprland_available() -> bool {
    get_hyprland_socket_path(".socket.sock").is_ok()
}

pub fn spawn_event_listener(tx: Sender<AppEvent>) {
    thread::spawn(move || {
        loop {
            match get_hyprland_socket_path(".socket2.sock") {
                Ok(path) => {
                    log::info!("Connecting to Hyprland event socket: {:?}", path);
                    match UnixStream::connect(&path) {
                        Ok(stream) => {
                            let reader = BufReader::new(stream);
                            for line in reader.lines() {
                                match line {
                                    Ok(l) => {
                                        if l.starts_with("activewindow>>") {
                                            let parts = l.splitn(2, ">>").nth(1).unwrap_or("");
                                            let mut class_title = parts.splitn(2, ",");
                                            let class =
                                                class_title.next().unwrap_or("").to_string();
                                            let title =
                                                class_title.next().unwrap_or("").to_string();
                                            let _ = tx.send(AppEvent::Input(
                                                InputEvent::ActiveWindow { class, title },
                                            ));
                                        } else if l.starts_with("workspace>>")
                                            || l.starts_with("focusedmon>>")
                                        {
                                            let _ = tx.send(AppEvent::WorkspaceChanged);
                                        }
                                    }
                                    Err(_) => break, // Disconnect
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to connect to event socket: {:?}. Retrying...", e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to locate event socket: {:?}. Retrying...", e);
                }
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
}

pub fn spawn_cursor_poller(tx: Sender<AppEvent>) {
    thread::spawn(move || {
        loop {
            match get_hyprland_socket_path(".socket.sock") {
                Ok(path) => loop {
                    match UnixStream::connect(&path) {
                        Ok(mut stream) => {
                            if stream.write_all(b"cursorpos").is_ok() {
                                let mut reader = BufReader::new(stream);
                                let mut response = String::new();
                                if reader.read_line(&mut response).is_ok() {
                                    if let Some((x, y)) = response.split_once(',') {
                                        let _ = tx.send(AppEvent::Input(InputEvent::CursorPos {
                                            x: x.trim().parse().unwrap_or_default(),
                                            y: y.trim().parse().unwrap_or_default(),
                                        }));
                                    };
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to query cursor position: {:?}. Reconnecting socket...",
                                e
                            );
                            break;
                        }
                    }
                    thread::sleep(Duration::from_millis(33));
                },
                Err(e) => {
                    log::warn!("Failed to locate command socket: {:?}. Retrying...", e);
                }
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
}

#[derive(serde::Deserialize, Debug, Clone)]
struct HyprActiveWindowJson {
    address: String,
    at: [i32; 2],
    size: [i32; 2],
    class: String,
    title: String,
    floating: bool,
}

pub fn spawn_active_window_poller(tx: Sender<AppEvent>) {
    thread::spawn(move || {
        loop {
            match get_hyprland_socket_path(".socket.sock") {
                Ok(path) => loop {
                    if let Some(geom) = query_active_window_geometry_internal(&path) {
                        let _ = tx.send(AppEvent::Input(InputEvent::ActiveWindowGeom(geom)));
                    } else {
                        let _ = tx.send(AppEvent::Input(InputEvent::ActiveWindowGeomNone));
                    }
                    thread::sleep(Duration::from_millis(150));
                },
                Err(e) => {
                    log::warn!("Failed to locate command socket: {:?}. Retrying...", e);
                }
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
}

fn query_active_window_geometry_internal(path: &std::path::Path) -> Option<ActiveWindowGeometry> {
    let mut stream = UnixStream::connect(path).ok()?;
    stream.write_all(b"j/activewindow").ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    if response.trim().is_empty() || response.trim() == "null" {
        return None;
    }
    let parsed: HyprActiveWindowJson = serde_json::from_str(&response).ok()?;

    // If window size is 0x0, it's not a real window (could be desktop or invalid)
    if parsed.size[0] <= 0 || parsed.size[1] <= 0 {
        return None;
    }

    Some(ActiveWindowGeometry {
        address: parsed.address,
        x: parsed.at[0],
        y: parsed.at[1],
        width: parsed.size[0],
        height: parsed.size[1],
        class: parsed.class,
        title: parsed.title,
        floating: parsed.floating,
    })
}
