use adw::prelude::*;
use gtk::gio;
use std::fs;
use std::path::Path;

use crate::profile::auth::show_import_auth_dialog;
use crate::profile::model::VpnProfile;
use crate::profile::storage::profiles_config_dir;
use crate::window::OpenvpnGuiWindow;

pub fn import_profile_dialog(window: &OpenvpnGuiWindow) {
    let filter = gtk::FileFilter::new();
    filter.add_pattern("*.ovpn");
    filter.add_pattern("*.conf");
    filter.set_name(Some("OpenVPN config (*.ovpn, *.conf)"));

    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);

    let dialog = gtk::FileDialog::builder()
        .title("Import OpenVPN Profile")
        .modal(true)
        .filters(&filters)
        .build();

    let win = window.clone();
    dialog.open(Some(window), gio::Cancellable::NONE, move |result| {
        if let Ok(file) = result {
            if let Some(path) = file.path() {
                handle_import(&win, &path);
            }
        }
    });
}

fn handle_import(window: &OpenvpnGuiWindow, source_path: &Path) {
    let file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.ovpn");

    let profile_name = file_name
        .trim_end_matches(".ovpn")
        .trim_end_matches(".conf")
        .to_string();

    let dest_dir = profiles_config_dir();
    let dest_path = dest_dir.join(file_name);

    if let Err(e) = fs::copy(source_path, &dest_path) {
        log::error!("Failed to copy config file: {}", e);
        window.add_toast(&format!("Failed to import: {}", e));
        return;
    }

    let config_path = dest_path.to_string_lossy().to_string();
    let name = profile_name.clone();
    let win = window.clone();

    let gtk_window: gtk::Window = window.clone().upcast();
    show_import_auth_dialog(&gtk_window, &profile_name, move |username| {
        let profile = VpnProfile::new(name.clone(), config_path, username);
        win.add_profile(profile);
        win.add_toast(&format!("Imported: {}", name));
    });
}
