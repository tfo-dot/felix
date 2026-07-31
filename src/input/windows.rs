#![cfg(target_os = "windows")]

use crate::event::{ActiveWindowGeometry, AppEvent, InputEvent};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub fn spawn_windows_poller(tx: Sender<AppEvent>) {
    let tx_cursor = tx.clone();
    // Cursor position poller (33ms interval)
    thread::spawn(move || {
        loop {
            let mut point = POINT { x: 0, y: 0 };
            unsafe {
                if GetCursorPos(&mut point) != 0 {
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
        let mut last_hwnd = 0;
        loop {
            unsafe {
                let hwnd = GetForegroundWindow();
                if hwnd != 0 {
                    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    if GetWindowRect(hwnd, &mut rect) != 0 {
                        let width = rect.right - rect.left;
                        let height = rect.bottom - rect.top;
                        
                        if width > 0 && height > 0 {
                            let mut title_buf = [0u16; 512];
                            let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
                            let title = String::from_utf16_lossy(&title_buf[..len as usize]);

                            let mut class_buf = [0u16; 256];
                            let len_class = GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32);
                            let class = String::from_utf16_lossy(&class_buf[..len_class as usize]);

                            if hwnd != last_hwnd {
                                last_hwnd = hwnd;
                                let _ = tx_win.send(AppEvent::Input(InputEvent::ActiveWindow {
                                    class: class.clone(),
                                    title: title.clone(),
                                }));
                            }

                            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
                            let is_resizable = (style & WS_THICKFRAME) != 0;

                            let geom = ActiveWindowGeometry {
                                address: format!("{:x}", hwnd),
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
                } else {
                    let _ = tx_win.send(AppEvent::Input(InputEvent::ActiveWindowGeomNone));
                }
            }
            thread::sleep(Duration::from_millis(150));
        }
    });
}
