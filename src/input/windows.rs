#![cfg(target_os = "windows")]

use crate::event::{ActiveWindowGeometry, AppEvent, InputEvent};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn spawn_windows_poller(tx: Sender<AppEvent>) {
    let tx_cursor = tx.clone();
    // Cursor position poller (33ms interval)
    thread::spawn(move || {
        loop {
            let mut point = POINT { x: 0, y: 0 };
            unsafe {
                if GetCursorPos(&mut point).is_ok() {
                    let _ = tx_cursor.send(AppEvent::Input(InputEvent::CursorPos {
                        x: point.x as f64,
                        y: point.y as f64,
                    }));
                }
            }
            thread::sleep(Duration::from_millis(33));
        }
    });

    let tx_win = tx.clone();
    // Active window geometry and details poller (150ms interval)
    thread::spawn(move || {
        let mut last_hwnd = HWND(core::ptr::null_mut());
        loop {
            unsafe {
                let hwnd = GetForegroundWindow();
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                if GetWindowRect(hwnd, &mut rect).is_ok() {
                    let width = rect.right - rect.left;
                    let height = rect.bottom - rect.top;

                    if width > 0 && height > 0 {
                        let mut title_buf = [0u16; 512];
                        let len = GetWindowTextW(hwnd, &mut title_buf);
                        let title = String::from_utf16_lossy(&title_buf[..len as usize]);

                        let mut class_buf = [0u16; 256];
                        let len_class = GetClassNameW(hwnd, &mut class_buf);
                        let class = String::from_utf16_lossy(&class_buf[..len_class as usize]);

                        if hwnd != last_hwnd {
                            last_hwnd = hwnd;
                            let _ = tx_win.send(AppEvent::Input(InputEvent::ActiveWindow {
                                class: class.clone(),
                                title: title.clone(),
                            }));
                        }

                        let style = WINDOW_STYLE(GetWindowLongW(hwnd, GWL_STYLE) as u32);
                        let is_resizable = (style & WS_THICKFRAME)
                            != windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0);

                        let geom = ActiveWindowGeometry {
                            address: "\0".to_string(),
                            x: rect.left,
                            y: rect.top,
                            width,
                            height,
                            class,
                            title,
                            floating: is_resizable,
                        };

                        let _ = tx_win.send(AppEvent::Input(InputEvent::ActiveWindowGeom(geom)));
                    } else {
                        let _ = tx_win.send(AppEvent::Input(InputEvent::ActiveWindowGeomNone));
                    }
                } else {
                    let _ = tx_win.send(AppEvent::Input(InputEvent::ActiveWindowGeomNone));
                }
            }
            thread::sleep(Duration::from_millis(150));
        }
    });
}
