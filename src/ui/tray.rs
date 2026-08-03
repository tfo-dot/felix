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
use ksni::{MenuItem, ToolTip, Tray, menu::StandardItem};

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
use std::sync::Mutex;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

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
struct SendHwnd(HWND);
unsafe impl Send for SendHwnd {}
unsafe impl Sync for SendHwnd {}

#[cfg(target_os = "windows")]
static TRAY_HWND: Mutex<SendHwnd> = Mutex::new(SendHwnd(HWND(core::ptr::null_mut())));

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

            let guard = TRAY_HWND.lock().unwrap();
            let hwnd = guard.0;
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(hwnd),
                    windows::Win32::UI::WindowsAndMessaging::WM_USER + 2,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(0),
                );
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
    use windows::Win32::Foundation::HINSTANCE;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::PCWSTR;

    *TRAY_TX.lock().unwrap() = Some(tx.clone());

    let (tx_hwnd, rx_hwnd) = std::sync::mpsc::channel::<SendHwnd>();

    thread::spawn(move || unsafe {
        let hinstance: Option<HINSTANCE> = GetModuleHandleW(None).map(|h| HINSTANCE(h.0)).ok();

        let class_name: Vec<u16> = "FelixTrayClass\0".encode_utf16().collect();

        let wnd_class = WNDCLASSW {
            style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
            lpfnWndProc: Some(tray_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance.unwrap(),
            hIcon: windows::Win32::UI::WindowsAndMessaging::HICON(core::ptr::null_mut()),
            hCursor: windows::Win32::UI::WindowsAndMessaging::HCURSOR(core::ptr::null_mut()),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(core::ptr::null_mut()),
            lpszMenuName: PCWSTR::from_raw(std::ptr::null()),
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
        };

        RegisterClassW(&wnd_class);

        let hwnd = CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(0),
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::from_raw(std::ptr::null()),
            windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            None,
            None,
            hinstance,
            Some(std::ptr::null()),
        )
        .expect("Failed to create window");

        *TRAY_HWND.lock().unwrap() = SendHwnd(hwnd);

        let nid = get_notify_icon_data(hwnd);
        windows::Win32::UI::Shell::Shell_NotifyIconW(windows::Win32::UI::Shell::NIM_ADD, &nid);

        let _ = tx_hwnd.send(SendHwnd(hwnd));

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, Some(hwnd), 0, 0).0 > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });

    let hwnd = rx_hwnd.recv().unwrap().0;
    TrayHandle { hwnd, tx }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn spawn_tray(_tx: Sender<AppEvent>) -> TrayHandle {
    TrayHandle {}
}

#[cfg(target_os = "windows")]
unsafe fn get_notify_icon_data(hwnd: HWND) -> windows::Win32::UI::Shell::NOTIFYICONDATAW {
    use windows::Win32::UI::Shell::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_USER + 1;
    nid.hIcon = unsafe { LoadIconW(None, IDI_APPLICATION).expect("Shouldnt be empty") };

    let title = "Felix Desktop Pet";
    let wtitle: Vec<u16> = title.encode_utf16().collect();
    let copy_len = std::cmp::min(wtitle.len(), nid.szTip.len() - 1);
    nid.szTip[..copy_len].copy_from_slice(&wtitle[..copy_len]);
    nid.szTip[copy_len] = 0;

    nid
}

#[cfg(target_os = "windows")]
unsafe fn update_tray_tooltip(hwnd: HWND) {
    use windows::Win32::UI::Shell::*;
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
unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::Shell::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::PCWSTR;

    match msg {
        1025 => {
            let event = lparam.0 as u32;
            if event == WM_RBUTTONUP {
                let mut pos = POINT { x: 0, y: 0 };
                unsafe { GetCursorPos(&mut pos) };

                let menu = unsafe { CreatePopupMenu() }.expect("Should be valid");
                let add_menu_item = |id: usize, text: &str| {
                    let wtext: Vec<u16> = text.encode_utf16().collect();
                    unsafe { AppendMenuW(menu, MF_STRING, id, PCWSTR::from_raw(wtext.as_ptr())) };
                };

                add_menu_item(1, "Toggle Pomodoro Pause\0");
                add_menu_item(2, "Reset Pomodoro\0");
                add_menu_item(3, "Reload Config\0");
                add_menu_item(4, "Toggle visibility\0");
                unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::from_raw(std::ptr::null())) };
                add_menu_item(5, "Quit\0");

                unsafe { SetForegroundWindow(hwnd) };
                let selected = unsafe {
                    TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD,
                        pos.x,
                        pos.y,
                        Some(0),
                        hwnd,
                        Some(std::ptr::null()),
                    )
                }
                .0;
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
            windows::Win32::Foundation::LRESULT(0)
        }
        1026 => {
            unsafe { update_tray_tooltip(hwnd) };
            windows::Win32::Foundation::LRESULT(0)
        }
        WM_DESTROY => {
            let nid = unsafe { get_notify_icon_data(hwnd) };
            unsafe { Shell_NotifyIconW(NIM_DELETE, &nid) };
            unsafe { PostQuitMessage(0) };
            windows::Win32::Foundation::LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
