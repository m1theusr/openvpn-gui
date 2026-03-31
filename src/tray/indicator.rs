use ksni::blocking::TrayMethods;
use std::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ShowWindow,
    ShowPanel,
    Quit,
}

#[derive(Debug)]
struct VpnTray {
    connected: bool,
    tx: mpsc::Sender<TrayCommand>,
}

impl ksni::Tray for VpnTray {
    fn id(&self) -> String {
        "openvpn-gui".into()
    }

    fn title(&self) -> String {
        "OpenVPN GUI".into()
    }

    fn icon_name(&self) -> String {
        "openvpn-gui".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
            title: "OpenVPN GUI".into(),
            description: if self.connected {
                "VPN Connected"
            } else {
                "VPN Disconnected"
            }
            .into(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayCommand::ShowPanel);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            ksni::menu::StandardItem {
                label: "Status Panel".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.tx.send(TrayCommand::ShowPanel);
                }),
                ..Default::default()
            }
            .into(),
            ksni::menu::StandardItem {
                label: "Open Window".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.tx.send(TrayCommand::ShowWindow);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            ksni::menu::StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.tx.send(TrayCommand::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[allow(dead_code)]
pub struct TrayHandle {
    handle: ksni::blocking::Handle<VpnTray>,
}

#[allow(dead_code)]
impl TrayHandle {
    pub fn set_connected(&self, connected: bool) {
        self.handle.update(move |tray| {
            tray.connected = connected;
        });
    }
}

pub fn spawn_tray() -> Option<(TrayHandle, mpsc::Receiver<TrayCommand>)> {
    let (tx, rx) = mpsc::channel();
    let tray = VpnTray {
        connected: false,
        tx,
    };
    match tray.assume_sni_available(true).spawn() {
        Ok(handle) => Some((TrayHandle { handle }, rx)),
        Err(e) => {
            log::warn!("Failed to spawn tray icon (install AppIndicator extension?): {}", e);
            None
        }
    }
}
