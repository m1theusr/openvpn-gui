use adw::prelude::*;
use gtk::{gio, glib};
use std::cell::Cell;
use std::rc::Rc;

use crate::profile::auth::{show_auth_dialog, show_save_credentials_dialog, AuthCredentials};
use crate::profile::model::VpnProfile;
use crate::window::OpenvpnGuiWindow;

pub fn create_profile_row(profile: &VpnProfile, window: &OpenvpnGuiWindow) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(&profile.name)
        .subtitle("OpenVPN Profile")
        .build();

    let toggle = gtk::Switch::builder()
        .valign(gtk::Align::Center)
        .tooltip_text("Toggle VPN connection")
        .build();
    toggle.update_property(&[
        gtk::accessible::Property::Label("Toggle VPN connection"),
    ]);

    let delete_button = gtk::Button::builder()
        .icon_name("edit-delete-symbolic")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .tooltip_text("Remove profile")
        .build();
    delete_button.update_property(&[
        gtk::accessible::Property::Label("Remove VPN profile"),
    ]);

    let is_already_connected = crate::vpn::manager::is_connected(&profile.name);
    let skip_signal = Rc::new(Cell::new(false));

    if is_already_connected {
        skip_signal.set(true);
        toggle.set_active(true);
        skip_signal.set(false);
    }

    let profile_name = profile.name.clone();
    let config_path = profile.config_path.clone();
    let saved_username = profile.username.clone();
    let saved_password = profile.password.clone();
    let has_saved_creds = saved_username.is_some() && saved_password.is_some();
    let weak_window = window.downgrade();
    let skip = skip_signal.clone();

    toggle.connect_state_set(move |switch, active| {
        if skip.get() {
            return glib::Propagation::Proceed;
        }

        let Some(window) = weak_window.upgrade() else {
            return glib::Propagation::Proceed;
        };

        if active {
            let gtk_win: gtk::Window = window.clone().upcast();
            let pname = profile_name.clone();
            let cpath = config_path.clone();
            let saved_u = saved_username.clone();
            let saved_p = saved_password.clone();
            let win_ref = window.clone();
            let sw = switch.clone();
            let sk = skip.clone();
            let has_creds = has_saved_creds;

            let pname_for_dialog = pname.clone();
            let do_connect = move |creds: AuthCredentials| {
                let pname2 = pname.clone();
                let username = creds.username.clone();
                let password = creds.password.clone();
                let sw2 = sw.clone();
                let sk2 = sk.clone();
                let win2 = win_ref.clone();
                let had_saved = has_creds;
                let u_save = creds.username.clone();
                let p_save = creds.password.clone();
                let pname_save = pname.clone();

                win_ref.update_status_banner("CONNECTING...", "");

                glib::spawn_future_local(async move {
                    let result = gio::spawn_blocking(move || {
                        crate::vpn::manager::connect(&pname, &cpath, &username, &password)
                    })
                    .await;
                    match result {
                        Ok(Ok(_)) => {
                            sk2.set(true);
                            sw2.set_active(true);
                            sk2.set(false);
                            win2.update_status_banner("CONNECTED", "");
                            win2.add_toast(&format!("Connected to {}", pname2));

                            if !had_saved {
                                let gtk_win2: gtk::Window = win2.clone().upcast();
                                let win3 = win2.clone();
                                let pn = pname_save.clone();
                                let u = u_save.clone();
                                let p = p_save.clone();
                                let pn2 = pn.clone();
                                let u2 = u.clone();
                                let p2 = p.clone();
                                show_save_credentials_dialog(&gtk_win2, &pn, &u, &p, move |save| {
                                    if save {
                                        win3.save_profile_credentials(&pn2, &u2, &p2);
                                        win3.add_toast("Credentials saved");
                                    }
                                });
                            }
                        }
                        Ok(Err(e)) => {
                            log::error!("Connection failed: {}", e);
                            sk2.set(true);
                            sw2.set_active(false);
                            sk2.set(false);
                            win2.update_status_banner("DISCONNECTED", "");
                            win2.add_toast(&format!("Failed: {}", e));
                        }
                        Err(e) => {
                            log::error!("Thread error: {:?}", e);
                            sk2.set(true);
                            sw2.set_active(false);
                            sk2.set(false);
                            win2.update_status_banner("DISCONNECTED", "");
                        }
                    }
                });
            };

            if has_creds {
                let creds = AuthCredentials {
                    username: saved_u.unwrap_or_default(),
                    password: saved_p.unwrap_or_default(),
                };
                do_connect(creds);
            } else {
                let sw_cancel = switch.clone();
                let sk_cancel = skip.clone();
                show_auth_dialog(&gtk_win, &pname_for_dialog, saved_u.as_deref(), saved_p.as_deref(), move |creds| {
                    match creds {
                        Some(c) => do_connect(c),
                        None => {
                            sk_cancel.set(true);
                            sw_cancel.set_active(false);
                            sk_cancel.set(false);
                        }
                    }
                });
            }
        } else {
            let pname = profile_name.clone();
            let sw = switch.clone();
            let sk = skip.clone();
            let win_ref = window.clone();

            glib::spawn_future_local(async move {
                let pname2 = pname.clone();
                let result = gio::spawn_blocking(move || {
                    crate::vpn::manager::disconnect(&pname2)
                })
                .await;
                match result {
                    Ok(Ok(_)) => {
                        sk.set(true);
                        sw.set_active(false);
                        sk.set(false);
                        win_ref.update_status_banner("DISCONNECTED", "");
                        win_ref.add_toast(&format!("Disconnected from {}", pname));
                    }
                    Ok(Err(e)) => {
                        log::error!("Disconnect failed: {}", e);
                        sk.set(true);
                        sw.set_active(true);
                        sk.set(false);
                        win_ref.add_toast(&format!("Disconnect failed: {}", e));
                    }
                    Err(e) => {
                        log::error!("Thread error: {:?}", e);
                    }
                }
            });
        }

        glib::Propagation::Proceed
    });

    let profile_name_del = profile.name.clone();
    let weak_window_del = window.downgrade();
    delete_button.connect_clicked(move |_| {
        let Some(window) = weak_window_del.upgrade() else { return };
        window.remove_profile(&profile_name_del);
    });

    row.add_prefix(&toggle);
    row.add_suffix(&delete_button);

    row
}
