use crate::event::{AppEvent, TrayAction};
use std::sync::mpsc::Sender;

pub struct PetTray {
    pub tx: Sender<AppEvent>,
    pub status_text: String,
}

impl PetTray {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        Self {
            tx,
            status_text: "Felix Desktop Pet".to_string(),
        }
    }
}

#[cfg(target_os = "linux")]
use ksni::{menu::StandardItem, MenuItem, ToolTip, Tray};

#[cfg(target_os = "linux")]
impl Tray for PetTray {
    fn id(&self) -> String {
        "felix".to_string()
    }

    fn icon_name(&self) -> String {
        "face-smile-symbolic".into()
    }

    fn title(&self) -> String {
        "Felix".into()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "Felix".to_string(),
            description: self.status_text.clone(),
            icon_name: "face-smile-symbolic".to_string(),
            icon_pixmap: Vec::new(),
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let tx1 = self.tx.clone();
        let tx2 = self.tx.clone();
        let tx3 = self.tx.clone();
        let tx4 = self.tx.clone();
        let tx5 = self.tx.clone();

        vec![
            StandardItem {
                label: "Toggle Pomodoro Pause".into(),
                activate: Box::new(move |_| {
                    let _ = tx1.send(AppEvent::Tray(TrayAction::TogglePause));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Reset Pomodoro".into(),
                activate: Box::new(move |_| {
                    let _ = tx2.send(AppEvent::Tray(TrayAction::ResetTimer));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Reload Config".into(),
                activate: Box::new(move |_| {
                    let _ = tx3.send(AppEvent::Tray(TrayAction::ReloadConfig));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Toggle visibility".into(),
                activate: Box::new(move |_| {
                    let _ = tx4.send(AppEvent::Tray(TrayAction::TogglePetVisibility));
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(move |_| {
                    let _ = tx5.send(AppEvent::Tray(TrayAction::Quit));
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use std::sync::Mutex;

pub struct TrayHandle {
    #[cfg(target_os = "linux")]
    handle: ksni::blocking::Handle<PetTray>,
    #[cfg(target_os = "windows")]
    hwnd: HWND,
    #[cfg(target_os = "windows")]
    tx: Sender<AppEvent>,
}

#[cfg(target_os = "windows")]
static STATUS_TEXT: Mutex<String> = Mutex::new(String::new());

#[cfg(target_os = "windows")]
static TRAY_HWND: Mutex<HWND> = Mutex::new(0);

#[cfg(target_os = "windows")]
static TRAY_TX: Mutex<Option<Sender<AppEvent>>> = Mutex::new(None);

impl TrayHandle {
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut PetTray) + Send + 'static,
    {
        #[cfg(target_os = "linux")]
        self.handle.update(f);
        
        #[cfg(target_os = "windows")]
        {
            let mut tray = PetTray {
                tx: self.tx.clone(),
                status_text: STATUS_TEXT.lock().unwrap().clone(),
            };
            f(&mut tray);
            *STATUS_TEXT.lock().unwrap() = tray.status_text.clone();
            
            let hwnd = *TRAY_HWND.lock().unwrap();
            if hwnd != 0 {
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
                        hwnd,
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_USER + 2,
                        0,
                        0,
                    );
                }
            }
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = f;
        }
    }
}

#[cfg(target_os = "linux")]
pub fn spawn_tray(tx: Sender<AppEvent>) -> TrayHandle {
    use ksni::blocking::TrayMethods;
    let tray = PetTray::new(tx);
    let handle = tray.spawn().expect("Failed to spawn system tray");
    TrayHandle { handle }
}

#[cfg(target_os = "windows")]
pub fn spawn_tray(tx: Sender<AppEvent>) -> TrayHandle {
    use std::thread;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

    *TRAY_TX.lock().unwrap() = Some(tx.clone());

    let (tx_hwnd, rx_hwnd) = std::sync::mpsc::channel::<HWND>();

    thread::spawn(move || unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let class_name: Vec<u16> = "FelixTrayClass\0".encode_utf16().collect();

        let wnd_class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(tray_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        RegisterClassW(&wnd_class);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            std::ptr::null(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            hinstance,
            std::ptr::null(),
        );

        if hwnd == 0 {
            log::error!("Failed to create helper window for Win32 system tray");
            return;
        }

        *TRAY_HWND.lock().unwrap() = hwnd;

        let nid = get_notify_icon_data(hwnd);
        windows_sys::Win32::UI::Shell::Shell_NotifyIconW(windows_sys::Win32::UI::Shell::NIM_ADD, &nid);

        let _ = tx_hwnd.send(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });

    let hwnd = rx_hwnd.recv().unwrap_or(0);
    TrayHandle { hwnd, tx }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn spawn_tray(_tx: Sender<AppEvent>) -> TrayHandle {
    TrayHandle {}
}

#[cfg(target_os = "windows")]
unsafe fn get_notify_icon_data(hwnd: HWND) -> windows_sys::Win32::UI::Shell::NOTIFYICONDATAW {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use windows_sys::Win32::UI::Shell::*;
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_USER + 1;
    nid.hIcon = unsafe { LoadIconW(0, IDI_APPLICATION) };

    let title = "Felix Desktop Pet";
    let wtitle: Vec<u16> = title.encode_utf16().collect();
    let copy_len = std::cmp::min(wtitle.len(), nid.szTip.len() - 1);
    nid.szTip[..copy_len].copy_from_slice(&wtitle[..copy_len]);
    nid.szTip[copy_len] = 0;

    nid
}

#[cfg(target_os = "windows")]
unsafe fn update_tray_tooltip(hwnd: HWND) {
    use windows_sys::Win32::UI::Shell::*;
    let mut nid = unsafe { get_notify_icon_data(hwnd) };
    let status = STATUS_TEXT.lock().unwrap().clone();
    if status.is_empty() {
        return;
    }
    let wstatus: Vec<u16> = status.encode_utf16().collect();
    let copy_len = std::cmp::min(wstatus.len(), nid.szTip.len() - 1);
    nid.szTip[..copy_len].copy_from_slice(&wstatus[..copy_len]);
    nid.szTip[copy_len] = 0;
    nid.uFlags |= NIF_TIP;
    unsafe { Shell_NotifyIconW(NIM_MODIFY, &nid) };
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn tray_wnd_proc(hwnd: HWND, msg: u32, wparam: windows_sys::Win32::Foundation::WPARAM, lparam: windows_sys::Win32::Foundation::LPARAM) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use windows_sys::Win32::UI::Shell::*;
    use windows_sys::Win32::Foundation::POINT;
    match msg {
        1025 => {
            let event = lparam as u32;
            if event == WM_RBUTTONUP {
                let mut pos = POINT { x: 0, y: 0 };
                unsafe { GetCursorPos(&mut pos) };

                let menu = unsafe { CreatePopupMenu() };
                let add_menu_item = |id: usize, text: &str| {
                    let wtext: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
                    unsafe { AppendMenuW(menu, MF_STRING, id, wtext.as_ptr()) };
                };

                add_menu_item(1, "Toggle Pomodoro Pause");
                add_menu_item(2, "Reset Pomodoro");
                add_menu_item(3, "Reload Config");
                add_menu_item(4, "Toggle visibility");
                unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null()) };
                add_menu_item(5, "Quit");

                unsafe { SetForegroundWindow(hwnd) };
                let selected = unsafe { TrackPopupMenu(menu, TPM_RETURNCMD, pos.x, pos.y, 0, hwnd, std::ptr::null()) };
                unsafe { DestroyMenu(menu) };

                if selected > 0 {
                    if let Some(ref tx) = *TRAY_TX.lock().unwrap() {
                        let action = match selected {
                            1 => Some(TrayAction::TogglePause),
                            2 => Some(TrayAction::ResetTimer),
                            3 => Some(TrayAction::ReloadConfig),
                            4 => Some(TrayAction::TogglePetVisibility),
                            5 => Some(TrayAction::Quit),
                            _ => None,
                        };
                        if let Some(act) = action {
                            let _ = tx.send(AppEvent::Tray(act));
                        }
                    }
                }
            } else if event == WM_LBUTTONDBLCLK {
                if let Some(ref tx) = *TRAY_TX.lock().unwrap() {
                    let _ = tx.send(AppEvent::Tray(TrayAction::TogglePetVisibility));
                }
            }
            0
        }
        1026 => {
            unsafe { update_tray_tooltip(hwnd) };
            0
        }
        WM_DESTROY => {
            let nid = unsafe { get_notify_icon_data(hwnd) };
            unsafe { Shell_NotifyIconW(NIM_DELETE, &nid) };
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
