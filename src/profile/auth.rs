use adw::prelude::*;

pub struct AuthCredentials {
    pub username: String,
    pub password: String,
}

pub fn show_auth_dialog(
    parent: &impl IsA<gtk::Window>,
    profile_name: &str,
    saved_username: Option<&str>,
    saved_password: Option<&str>,
    callback: impl FnOnce(Option<AuthCredentials>) + 'static,
) {
    let dialog = adw::MessageDialog::new(
        Some(parent),
        Some(&format!("Connect to {}", profile_name)),
        Some("Enter your OpenVPN credentials"),
    );
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("connect", "Connect");
    dialog.set_response_appearance("connect", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("connect"));
    dialog.set_close_response("cancel");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let username_entry = gtk::Entry::builder()
        .placeholder_text("Username")
        .build();
    if let Some(u) = saved_username {
        username_entry.set_text(u);
    }

    let password_entry = gtk::PasswordEntry::builder()
        .placeholder_text("Password")
        .show_peek_icon(true)
        .build();
    if let Some(p) = saved_password {
        password_entry.set_text(p);
    }

    content.append(&username_entry);
    content.append(&password_entry);

    dialog.set_extra_child(Some(&content));

    let u_ref = username_entry.clone();
    let p_ref = password_entry.clone();
    let callback = std::cell::Cell::new(Some(callback));

    dialog.connect_response(None, move |_, response| {
        if let Some(cb) = callback.take() {
            match response {
                "connect" => {
                    let username = u_ref.text().to_string();
                    let password = p_ref.text().to_string();
                    if username.is_empty() || password.is_empty() {
                        cb(None);
                        return;
                    }
                    cb(Some(AuthCredentials { username, password }));
                }
                _ => cb(None),
            }
        }
    });

    dialog.present();
}

pub fn show_save_credentials_dialog(
    parent: &impl IsA<gtk::Window>,
    profile_name: &str,
    username: &str,
    password: &str,
    callback: impl FnOnce(bool) + 'static,
) {
    let dialog = adw::MessageDialog::new(
        Some(parent),
        Some("Save credentials?"),
        Some(&format!("Save username and password for {} so you can connect with one click next time?", profile_name)),
    );
    dialog.add_response("no", "No");
    dialog.add_response("yes", "Save");
    dialog.set_response_appearance("yes", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("yes"));
    dialog.set_close_response("no");

    let _ = (username, password);
    let callback = std::cell::Cell::new(Some(callback));

    dialog.connect_response(None, move |_, response| {
        if let Some(cb) = callback.take() {
            cb(response == "yes");
        }
    });

    dialog.present();
}

pub fn show_import_auth_dialog(
    parent: &impl IsA<gtk::Window>,
    profile_name: &str,
    callback: impl FnOnce(Option<String>) + 'static,
) {
    let dialog = adw::MessageDialog::new(
        Some(parent),
        Some(&format!("Save credentials for {}", profile_name)),
        Some("Optionally save your username for this profile"),
    );
    dialog.add_response("skip", "Skip");
    dialog.add_response("save", "Save");
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("save"));
    dialog.set_close_response("skip");

    let username_entry = gtk::Entry::builder()
        .placeholder_text("Username")
        .margin_start(12)
        .margin_end(12)
        .build();

    dialog.set_extra_child(Some(&username_entry));

    let u_ref = username_entry.clone();
    let callback = std::cell::Cell::new(Some(callback));

    dialog.connect_response(None, move |_, response| {
        if let Some(cb) = callback.take() {
            match response {
                "save" => {
                    let username = u_ref.text().to_string();
                    if username.is_empty() {
                        cb(None);
                    } else {
                        cb(Some(username));
                    }
                }
                _ => cb(None),
            }
        }
    });

    dialog.present();
}
