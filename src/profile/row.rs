use adw::prelude::*;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::profile::auth::{show_auth_dialog, show_save_credentials_dialog, AuthCredentials};
use crate::profile::model::VpnProfile;
use crate::vpn::manager;
use crate::window::OpenvpnGuiWindow;

const MAX_SAMPLES: usize = 30;
const REFRESH_MS: u32 = 2000;

struct StatsState {
    last_bytes_in: u64,
    last_bytes_out: u64,
    download_speeds: Vec<f64>,
    upload_speeds: Vec<f64>,
    connected_at: Option<Instant>,
}

impl Default for StatsState {
    fn default() -> Self {
        Self {
            last_bytes_in: 0,
            last_bytes_out: 0,
            download_speeds: Vec::with_capacity(MAX_SAMPLES),
            upload_speeds: Vec::with_capacity(MAX_SAMPLES),
            connected_at: None,
        }
    }
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{:02}h {:02}m {:02}s", h, m, s)
    } else {
        format!("{:02}m {:02}s", m, s)
    }
}

fn format_speed(bps: f64) -> String {
    if bps < 1024.0 {
        format!("{:.0} B/s", bps)
    } else if bps < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{:.1} MB/s", bps / (1024.0 * 1024.0))
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn build_stats_section() -> (gtk::Box, gtk::Label, gtk::Label, gtk::Label, gtk::Label, gtk::DrawingArea, gtk::Label, gtk::Label, gtk::Label) {
    let stats_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    stats_box.set_margin_start(12);
    stats_box.set_margin_end(12);
    stats_box.set_margin_top(8);
    stats_box.set_margin_bottom(8);

    let speed_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let dl_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let dl_speed = gtk::Label::builder().label("0 B/s").halign(gtk::Align::Start).build();
    dl_speed.add_css_class("heading");
    let dl_cap = gtk::Label::builder().label("↓ Download").halign(gtk::Align::Start).build();
    dl_cap.add_css_class("caption");
    dl_cap.add_css_class("dim-label");
    dl_box.append(&dl_speed);
    dl_box.append(&dl_cap);

    let ul_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    ul_box.set_halign(gtk::Align::End);
    ul_box.set_hexpand(true);
    let ul_speed = gtk::Label::builder().label("0 B/s").halign(gtk::Align::End).build();
    ul_speed.add_css_class("heading");
    let ul_cap = gtk::Label::builder().label("↑ Upload").halign(gtk::Align::End).build();
    ul_cap.add_css_class("caption");
    ul_cap.add_css_class("dim-label");
    ul_box.append(&ul_speed);
    ul_box.append(&ul_cap);

    speed_row.append(&dl_box);
    speed_row.append(&ul_box);
    stats_box.append(&speed_row);

    let drawing_area = gtk::DrawingArea::builder()
        .height_request(60)
        .hexpand(true)
        .build();
    stats_box.append(&drawing_area);

    let bytes_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    bytes_row.set_margin_top(2);
    let bi_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let bytes_in_val = gtk::Label::builder().label("0 B").halign(gtk::Align::Start).build();
    bytes_in_val.add_css_class("caption-heading");
    let bi_lbl = gtk::Label::builder().label("BYTES IN").halign(gtk::Align::Start).build();
    bi_lbl.add_css_class("caption");
    bi_lbl.add_css_class("dim-label");
    bi_box.append(&bytes_in_val);
    bi_box.append(&bi_lbl);

    let bo_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    bo_box.set_halign(gtk::Align::End);
    bo_box.set_hexpand(true);
    let bytes_out_val = gtk::Label::builder().label("0 B").halign(gtk::Align::End).build();
    bytes_out_val.add_css_class("caption-heading");
    let bo_lbl = gtk::Label::builder().label("BYTES OUT").halign(gtk::Align::End).build();
    bo_lbl.add_css_class("caption");
    bo_lbl.add_css_class("dim-label");
    bo_box.append(&bytes_out_val);
    bo_box.append(&bo_lbl);

    bytes_row.append(&bi_box);
    bytes_row.append(&bo_box);
    stats_box.append(&bytes_row);

    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep.set_margin_top(4);
    sep.set_margin_bottom(4);
    stats_box.append(&sep);

    let ip_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    let vpn_ip_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let vpn_ip_val = gtk::Label::builder().label("—").halign(gtk::Align::Start).build();
    vpn_ip_val.add_css_class("caption-heading");
    let vpn_ip_lbl = gtk::Label::builder().label("VPN IP").halign(gtk::Align::Start).build();
    vpn_ip_lbl.add_css_class("caption");
    vpn_ip_lbl.add_css_class("dim-label");
    vpn_ip_box.append(&vpn_ip_val);
    vpn_ip_box.append(&vpn_ip_lbl);

    let srv_ip_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    srv_ip_box.set_halign(gtk::Align::End);
    srv_ip_box.set_hexpand(true);
    let srv_ip_val = gtk::Label::builder().label("—").halign(gtk::Align::End).build();
    srv_ip_val.add_css_class("caption-heading");
    let srv_ip_lbl = gtk::Label::builder().label("SERVER IP").halign(gtk::Align::End).build();
    srv_ip_lbl.add_css_class("caption");
    srv_ip_lbl.add_css_class("dim-label");
    srv_ip_box.append(&srv_ip_val);
    srv_ip_box.append(&srv_ip_lbl);

    ip_row.append(&vpn_ip_box);
    ip_row.append(&srv_ip_box);
    stats_box.append(&ip_row);

    let dur_lbl = gtk::Label::builder().label("—").halign(gtk::Align::Start).build();
    dur_lbl.add_css_class("caption");
    dur_lbl.add_css_class("dim-label");
    dur_lbl.set_margin_top(2);
    stats_box.append(&dur_lbl);

    (stats_box, dl_speed, ul_speed, bytes_in_val, bytes_out_val, drawing_area, vpn_ip_val, srv_ip_val, dur_lbl)
}

fn start_stats_timer(
    profile_name: String,
    dl_lbl: gtk::Label,
    ul_lbl: gtk::Label,
    bi_lbl: gtk::Label,
    bo_lbl: gtk::Label,
    da: gtk::DrawingArea,
    vpn_ip_lbl: gtk::Label,
    srv_ip_lbl: gtk::Label,
    dur_lbl: gtk::Label,
    state: Rc<RefCell<StatsState>>,
    active: Rc<Cell<bool>>,
) {
    let da_weak = da.downgrade();
    glib::timeout_add_local(Duration::from_millis(REFRESH_MS as u64), move || {
        if !active.get() {
            return glib::ControlFlow::Break;
        }
        let Some(da) = da_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };

        let pname = profile_name.clone();
        let da2 = da.clone();
        let dl2 = dl_lbl.clone();
        let ul2 = ul_lbl.clone();
        let bi2 = bi_lbl.clone();
        let bo2 = bo_lbl.clone();
        let vip = vpn_ip_lbl.clone();
        let sip = srv_ip_lbl.clone();
        let dur = dur_lbl.clone();
        let st = state.clone();

        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || manager::read_stats_and_info(&pname)).await;
            if let Ok(Some((bytes_in, bytes_out, vpn_ip, server_ip))) = result {
                let mut s = st.borrow_mut();
                if s.connected_at.is_none() {
                    s.connected_at = Some(Instant::now());
                }
                if s.last_bytes_in > 0 {
                    let dt = REFRESH_MS as f64 / 1000.0;
                    let dl_speed = bytes_in.saturating_sub(s.last_bytes_in) as f64 / dt;
                    let ul_speed = bytes_out.saturating_sub(s.last_bytes_out) as f64 / dt;
                    s.download_speeds.push(dl_speed);
                    s.upload_speeds.push(ul_speed);
                    if s.download_speeds.len() > MAX_SAMPLES {
                        s.download_speeds.remove(0);
                        s.upload_speeds.remove(0);
                    }
                    dl2.set_label(&format_speed(dl_speed));
                    ul2.set_label(&format_speed(ul_speed));
                }
                s.last_bytes_in = bytes_in;
                s.last_bytes_out = bytes_out;
                bi2.set_label(&format_bytes(bytes_in));
                bo2.set_label(&format_bytes(bytes_out));
                if !vpn_ip.is_empty() { vip.set_label(&vpn_ip); }
                if !server_ip.is_empty() { sip.set_label(&server_ip); }
                if let Some(t) = s.connected_at {
                    dur.set_label(&format!("Connected for {}", format_duration(t.elapsed().as_secs())));
                }
                da2.queue_draw();
            }
        });

        glib::ControlFlow::Continue
    });
}

fn draw_graph(cr: &gtk::cairo::Context, width: f64, height: f64, downloads: &[f64], uploads: &[f64]) {
    cr.set_source_rgba(0.15, 0.15, 0.15, 0.4);
    let _ = cr.paint();
    if downloads.is_empty() {
        return;
    }
    let max_val = downloads.iter().chain(uploads.iter()).cloned().fold(1.0_f64, f64::max);
    let n = downloads.len();
    let step = if n > 1 { width / (n - 1) as f64 } else { width };

    cr.set_source_rgba(0.2, 0.78, 0.4, 0.25);
    cr.move_to(0.0, height);
    for (i, &val) in downloads.iter().enumerate() {
        cr.line_to(i as f64 * step, height - (val / max_val * height * 0.85));
    }
    cr.line_to((n - 1) as f64 * step, height);
    cr.close_path();
    let _ = cr.fill();

    cr.set_source_rgba(0.2, 0.78, 0.4, 0.9);
    cr.set_line_width(2.0);
    for (i, &val) in downloads.iter().enumerate() {
        let x = i as f64 * step;
        let y = height - (val / max_val * height * 0.85);
        if i == 0 { cr.move_to(x, y); } else { cr.line_to(x, y); }
    }
    let _ = cr.stroke();

    cr.set_source_rgba(0.93, 0.5, 0.14, 0.25);
    cr.move_to(0.0, height);
    for (i, &val) in uploads.iter().enumerate() {
        cr.line_to(i as f64 * step, height - (val / max_val * height * 0.85));
    }
    cr.line_to((n - 1) as f64 * step, height);
    cr.close_path();
    let _ = cr.fill();

    cr.set_source_rgba(0.93, 0.5, 0.14, 0.9);
    cr.set_line_width(2.0);
    for (i, &val) in uploads.iter().enumerate() {
        let x = i as f64 * step;
        let y = height - (val / max_val * height * 0.85);
        if i == 0 { cr.move_to(x, y); } else { cr.line_to(x, y); }
    }
    let _ = cr.stroke();
}

pub fn create_profile_row(profile: &VpnProfile, window: &OpenvpnGuiWindow) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .build();
    list.add_css_class("boxed-list");

    let row = adw::ActionRow::builder()
        .title(&profile.name)
        .subtitle("OpenVPN Profile")
        .build();

    let toggle = gtk::Switch::builder()
        .valign(gtk::Align::Center)
        .tooltip_text("Toggle VPN connection")
        .build();
    toggle.update_property(&[gtk::accessible::Property::Label("Toggle VPN connection")]);

    let delete_button = gtk::Button::builder()
        .icon_name("edit-delete-symbolic")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .tooltip_text("Remove profile")
        .build();
    delete_button.update_property(&[gtk::accessible::Property::Label("Remove VPN profile")]);

    row.add_prefix(&toggle);
    row.add_suffix(&delete_button);
    list.append(&row);
    section.append(&list);

    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .transition_duration(200)
        .reveal_child(false)
        .build();

    let (stats_box, dl_lbl, ul_lbl, bi_lbl, bo_lbl, drawing_area, vpn_ip_lbl, srv_ip_lbl, dur_lbl) = build_stats_section();
    let stats_state: Rc<RefCell<StatsState>> = Rc::new(RefCell::new(StatsState::default()));
    let stats_active: Rc<Cell<bool>> = Rc::new(Cell::new(false));

    let state_draw = stats_state.clone();
    drawing_area.set_draw_func(move |_area, cr, w, h| {
        let st = state_draw.borrow();
        draw_graph(cr, w as f64, h as f64, &st.download_speeds, &st.upload_speeds);
    });

    revealer.set_child(Some(&stats_box));
    section.append(&revealer);

    let is_already_connected = manager::is_connected(&profile.name);
    let skip_signal = Rc::new(Cell::new(false));

    if is_already_connected {
        skip_signal.set(true);
        toggle.set_active(true);
        skip_signal.set(false);
        revealer.set_reveal_child(true);
        stats_active.set(true);
        start_stats_timer(
            profile.name.clone(),
            dl_lbl.clone(), ul_lbl.clone(), bi_lbl.clone(), bo_lbl.clone(),
            drawing_area.clone(),
            vpn_ip_lbl.clone(), srv_ip_lbl.clone(), dur_lbl.clone(),
            stats_state.clone(), stats_active.clone(),
        );
    }

    let profile_name = profile.name.clone();
    let config_path = profile.config_path.clone();
    let saved_username = profile.username.clone();
    let saved_password = profile.password.clone();
    let has_saved_creds = saved_username.is_some() && saved_password.is_some();
    let weak_window = window.downgrade();
    let skip = skip_signal.clone();
    let rev = revealer.clone();
    let sa = stats_active.clone();
    let ss = stats_state.clone();
    let dl_c = dl_lbl.clone();
    let ul_c = ul_lbl.clone();
    let bi_c = bi_lbl.clone();
    let bo_c = bo_lbl.clone();
    let da_c = drawing_area.clone();
    let vip_c = vpn_ip_lbl.clone();
    let sip_c = srv_ip_lbl.clone();
    let dur_c = dur_lbl.clone();

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
            let rev2 = rev.clone();
            let sa2 = sa.clone();
            let ss2 = ss.clone();
            let dl2 = dl_c.clone();
            let ul2 = ul_c.clone();
            let bi2 = bi_c.clone();
            let bo2 = bo_c.clone();
            let da2 = da_c.clone();
            let vip2 = vip_c.clone();
            let sip2 = sip_c.clone();
            let dur2 = dur_c.clone();

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
                let rev3 = rev2.clone();
                let sa3 = sa2.clone();
                let ss3 = ss2.clone();
                let dl3 = dl2.clone();
                let ul3 = ul2.clone();
                let bi3 = bi2.clone();
                let bo3 = bo2.clone();
                let da3 = da2.clone();
                let vip3 = vip2.clone();
                let sip3 = sip2.clone();
                let dur3 = dur2.clone();

                win_ref.update_status_banner("CONNECTING...", "");

                glib::spawn_future_local(async move {
                    let result = gio::spawn_blocking(move || {
                        manager::connect(&pname, &cpath, &username, &password)
                    }).await;
                    match result {
                        Ok(Ok(_)) => {
                            sk2.set(true);
                            sw2.set_active(true);
                            sk2.set(false);
                            win2.update_status_banner("CONNECTED", "");
                            win2.add_toast(&format!("Connected to {}", pname2));

                            rev3.set_reveal_child(true);
                            sa3.set(true);
                            *ss3.borrow_mut() = StatsState::default();
                            start_stats_timer(pname2.clone(), dl3, ul3, bi3, bo3, da3, vip3, sip3, dur3, ss3, sa3);

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
            let rev2 = rev.clone();
            let sa2 = sa.clone();

            glib::spawn_future_local(async move {
                let pname2 = pname.clone();
                let result = gio::spawn_blocking(move || manager::disconnect(&pname2)).await;
                match result {
                    Ok(Ok(_)) => {
                        sk.set(true);
                        sw.set_active(false);
                        sk.set(false);
                        win_ref.update_status_banner("DISCONNECTED", "");
                        win_ref.add_toast(&format!("Disconnected from {}", pname));
                        sa2.set(false);
                        rev2.set_reveal_child(false);
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
    delete_button.connect_clicked(move |btn| {
        let Some(window) = weak_window_del.upgrade() else { return };
        let gtk_win: gtk::Window = window.clone().upcast();
        let pname = profile_name_del.clone();
        let win_ref = window.clone();
        let dialog = adw::MessageDialog::new(
            Some(&gtk_win),
            Some("Remove Profile"),
            Some(&format!("Remove \"{}\"? This cannot be undone.", pname)),
        );
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("remove", "Remove");
        dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let _ = btn;
        dialog.connect_response(None, move |_, response| {
            if response == "remove" {
                win_ref.remove_profile(&pname);
            }
        });
        dialog.present();
    });

    section
}
