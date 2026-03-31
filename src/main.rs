mod app;
mod panel;
mod profile;
mod tray;
mod vpn;
mod window;

use gtk::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use app::OpenvpnGuiApp;
use tray::indicator::{spawn_tray, TrayCommand};

fn install_icon() {
    let icon_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("icons/hicolor/scalable/apps");
    std::fs::create_dir_all(&icon_dir).ok();
    let dest = icon_dir.join("openvpn-gui.svg");
    if !dest.exists() {
        let bytes = gio::resources_lookup_data(
            "/com/github/m1theusr/OpenVPNGUI/openvpn-logo.svg",
            gio::ResourceLookupFlags::NONE,
        );
        if let Ok(data) = bytes {
            std::fs::write(&dest, data.as_ref()).ok();
        }
    }
}

fn main() -> glib::ExitCode {
    env_logger::init();

    gio::resources_register_include!("openvpn-gui.gresource")
        .expect("Failed to register resources.");

    adw::init().expect("Failed to initialize libadwaita.");

    install_icon();

    let tray_result = spawn_tray();

    let app = OpenvpnGuiApp::new();

    if let Some((_tray_handle, rx)) = tray_result {
        let app_weak = app.downgrade();
        let panel_win: Rc<RefCell<Option<adw::Window>>> = Rc::new(RefCell::new(None));
        glib::timeout_add_local(Duration::from_millis(200), move || {
            while let Ok(cmd) = rx.try_recv() {
                let Some(app) = app_weak.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                match cmd {
                    TrayCommand::ShowWindow => {
                        app.activate();
                    }
                    TrayCommand::ShowPanel => {
                        let mut pw = panel_win.borrow_mut();
                        if let Some(ref win) = *pw {
                            if win.is_visible() {
                                win.present();
                                continue;
                            }
                        }
                        let adw_app: adw::Application = app.clone().upcast();
                        let w = panel::create_panel(&adw_app);
                        w.present();
                        *pw = Some(w);
                    }
                    TrayCommand::Quit => {
                        app.quit();
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }

    app.run()
}
