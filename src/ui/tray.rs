use crate::event::{AppEvent, TrayAction};
use ksni::menu::StandardItem;
use ksni::{MenuItem, ToolTip, Tray};
use std::sync::mpsc::Sender;

pub struct PetTray {
    tx: Sender<AppEvent>,
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

pub fn spawn_tray(tx: Sender<AppEvent>) -> ksni::blocking::Handle<PetTray> {
    use ksni::blocking::TrayMethods;
    let tray = PetTray::new(tx);
    tray.spawn().expect("Failed to spawn system tray")
}
