use adw::prelude::*;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::profile::auth::{show_auth_dialog, AuthCredentials};
use crate::profile::model::VpnProfile;
use crate::profile::storage;
use crate::vpn::manager;

const MAX_SAMPLES: usize = 30;
const REFRESH_MS: u32 = 2000;
const MAX_PROFILES: usize = 10;

struct PanelState {
    last_bytes_in: u64,
    last_bytes_out: u64,
    download_speeds: Vec<f64>,
    upload_speeds: Vec<f64>,
    total_bytes_in: u64,
    total_bytes_out: u64,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            last_bytes_in: 0,
            last_bytes_out: 0,
            download_speeds: Vec::with_capacity(MAX_SAMPLES),
            upload_speeds: Vec::with_capacity(MAX_SAMPLES),
            total_bytes_in: 0,
            total_bytes_out: 0,
        }
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

pub fn create_panel(app: &adw::Application) -> adw::Window {
    let profiles = storage::load_profiles();
    let connected_profile: Option<VpnProfile> = profiles
        .iter()
        .find(|p| manager::is_connected(&p.name))
        .cloned();

    let window = adw::Window::builder()
        .title("OpenVPN Connect")
        .default_width(320)
        .default_height(520)
        .resizable(false)
        .application(app)
        .build();

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new("OpenVPN Connect", ""))
        .build();
    toolbar.add_top_bar(&header);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let status_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    status_box.set_margin_top(12);
    status_box.set_margin_bottom(8);
    status_box.set_margin_start(16);
    status_box.set_margin_end(16);

    let status_label = gtk::Label::builder()
        .label(if connected_profile.is_some() { "CONNECTED" } else { "DISCONNECTED" })
        .halign(gtk::Align::Start)
        .xalign(0.0)
        .build();
    status_label.add_css_class("title-4");
    if connected_profile.is_some() {
        status_label.add_css_class("success");
    } else {
        status_label.add_css_class("dim-label");
    }

    let separator1 = gtk::Separator::new(gtk::Orientation::Horizontal);
    status_box.append(&status_label);
    status_box.append(&separator1);
    content.append(&status_box);

    let stats_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    stats_box.set_margin_start(16);
    stats_box.set_margin_end(16);
    stats_box.set_margin_top(8);
    stats_box.set_margin_bottom(8);

    let speed_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    let dl_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let dl_speed_label = gtk::Label::builder()
        .label("0 B/s")
        .halign(gtk::Align::Start)
        .build();
    dl_speed_label.add_css_class("heading");
    let dl_caption = gtk::Label::builder()
        .label("↓ Download")
        .halign(gtk::Align::Start)
        .build();
    dl_caption.add_css_class("caption");
    dl_caption.add_css_class("dim-label");
    dl_box.append(&dl_speed_label);
    dl_box.append(&dl_caption);

    let ul_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    ul_box.set_halign(gtk::Align::End);
    ul_box.set_hexpand(true);
    let ul_speed_label = gtk::Label::builder()
        .label("0 B/s")
        .halign(gtk::Align::End)
        .build();
    ul_speed_label.add_css_class("heading");
    let ul_caption = gtk::Label::builder()
        .label("↑ Upload")
        .halign(gtk::Align::End)
        .build();
    ul_caption.add_css_class("caption");
    ul_caption.add_css_class("dim-label");
    ul_box.append(&ul_speed_label);
    ul_box.append(&ul_caption);

    speed_row.append(&dl_box);
    speed_row.append(&ul_box);
    stats_box.append(&speed_row);

    let drawing_area = gtk::DrawingArea::builder()
        .height_request(80)
        .hexpand(true)
        .build();

    let state: Rc<RefCell<PanelState>> = Rc::new(RefCell::new(PanelState::default()));

    let state_draw = state.clone();
    drawing_area.set_draw_func(move |_area, cr, width, height| {
        let st = state_draw.borrow();
        draw_graph(cr, width as f64, height as f64, &st.download_speeds, &st.upload_speeds);
    });

    stats_box.append(&drawing_area);

    let bytes_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    bytes_row.set_margin_top(4);

    let bi_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let bytes_in_val = gtk::Label::builder()
        .label("0 B")
        .halign(gtk::Align::Start)
        .build();
    bytes_in_val.add_css_class("caption-heading");
    let bytes_in_lbl = gtk::Label::builder()
        .label("BYTES IN")
        .halign(gtk::Align::Start)
        .build();
    bytes_in_lbl.add_css_class("caption");
    bytes_in_lbl.add_css_class("dim-label");
    bi_box.append(&bytes_in_val);
    bi_box.append(&bytes_in_lbl);

    let bo_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    bo_box.set_halign(gtk::Align::End);
    bo_box.set_hexpand(true);
    let bytes_out_val = gtk::Label::builder()
        .label("0 B")
        .halign(gtk::Align::End)
        .build();
    bytes_out_val.add_css_class("caption-heading");
    let bytes_out_lbl = gtk::Label::builder()
        .label("BYTES OUT")
        .halign(gtk::Align::End)
        .build();
    bytes_out_lbl.add_css_class("caption");
    bytes_out_lbl.add_css_class("dim-label");
    bo_box.append(&bytes_out_val);
    bo_box.append(&bytes_out_lbl);

    bytes_row.append(&bi_box);
    bytes_row.append(&bo_box);
    stats_box.append(&bytes_row);

    if connected_profile.is_some() {
        content.append(&stats_box);
    }

    let separator2 = gtk::Separator::new(gtk::Orientation::Horizontal);
    separator2.set_margin_top(8);
    content.append(&separator2);

    let profiles_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .margin_start(8)
        .margin_end(8)
        .margin_top(8)
        .margin_bottom(8)
        .build();
    profiles_list.add_css_class("boxed-list");

    let status_lbl_ref = status_label.clone();
    let stats_box_ref = stats_box.clone();

    for profile in profiles.iter().take(MAX_PROFILES) {
        let row = adw::ActionRow::builder()
            .title(&profile.name)
            .subtitle("OpenVPN Profile")
            .build();

        let toggle = gtk::Switch::builder()
            .valign(gtk::Align::Center)
            .build();

        let is_connected = manager::is_connected(&profile.name);
        let skip_signal = Rc::new(Cell::new(false));

        if is_connected {
            skip_signal.set(true);
            toggle.set_active(true);
            skip_signal.set(false);
        }

        let pname = profile.name.clone();
        let cpath = profile.config_path.clone();
        let saved_u = profile.username.clone();
        let saved_p = profile.password.clone();
        let has_creds = saved_u.is_some() && saved_p.is_some();
        let win_weak = window.downgrade();
        let skip = skip_signal.clone();
        let sl = status_lbl_ref.clone();
        let sb = stats_box_ref.clone();

        toggle.connect_state_set(move |switch, active| {
            if skip.get() {
                return glib::Propagation::Proceed;
            }

            let Some(win) = win_weak.upgrade() else {
                return glib::Propagation::Proceed;
            };

            if active {
                let pname = pname.clone();
                let cpath = cpath.clone();
                let sw = switch.clone();
                let sk = skip.clone();
                let sl = sl.clone();
                let sb = sb.clone();

                let pname_for_dialog = pname.clone();
                let do_connect = move |creds: AuthCredentials| {
                    let sw2 = sw.clone();
                    let sk2 = sk.clone();
                    let sl2 = sl.clone();
                    let sb2 = sb.clone();
                    sl.set_label("CONNECTING...");
                    sl.remove_css_class("dim-label");
                    sl.remove_css_class("success");
                    sl.add_css_class("warning");

                    glib::spawn_future_local(async move {
                        let result = gio::spawn_blocking(move || {
                            manager::connect(&pname, &cpath, &creds.username, &creds.password)
                        })
                        .await;
                        match result {
                            Ok(Ok(_)) => {
                                sk2.set(true);
                                sw2.set_active(true);
                                sk2.set(false);
                                sl2.set_label("CONNECTED");
                                sl2.remove_css_class("warning");
                                sl2.remove_css_class("dim-label");
                                sl2.add_css_class("success");
                                sb2.set_visible(true);
                            }
                            Ok(Err(e)) => {
                                log::error!("Panel connect failed: {}", e);
                                sk2.set(true);
                                sw2.set_active(false);
                                sk2.set(false);
                                sl2.set_label("DISCONNECTED");
                                sl2.remove_css_class("warning");
                                sl2.remove_css_class("success");
                                sl2.add_css_class("dim-label");
                            }
                            Err(e) => {
                                log::error!("Panel thread error: {:?}", e);
                                sk2.set(true);
                                sw2.set_active(false);
                                sk2.set(false);
                            }
                        }
                    });
                };

                if has_creds {
                    let creds = AuthCredentials {
                        username: saved_u.clone().unwrap_or_default(),
                        password: saved_p.clone().unwrap_or_default(),
                    };
                    do_connect(creds);
                } else {
                    let gtk_win: gtk::Window = win.clone().upcast();
                    let pname_d = pname_for_dialog.clone();
                    let su = saved_u.clone();
                    let sp = saved_p.clone();
                    let sw_cancel = switch.clone();
                    let sk_cancel = skip.clone();
                    show_auth_dialog(&gtk_win, &pname_d, su.as_deref(), sp.as_deref(), move |creds| {
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
                let pname = pname.clone();
                let sw = switch.clone();
                let sk = skip.clone();
                let sl = sl.clone();
                let sb = sb.clone();

                glib::spawn_future_local(async move {
                    let pname2 = pname.clone();
                    let result = gio::spawn_blocking(move || {
                        manager::disconnect(&pname2)
                    })
                    .await;
                    match result {
                        Ok(Ok(_)) => {
                            sk.set(true);
                            sw.set_active(false);
                            sk.set(false);
                            sl.set_label("DISCONNECTED");
                            sl.remove_css_class("success");
                            sl.remove_css_class("warning");
                            sl.add_css_class("dim-label");
                            sb.set_visible(false);
                        }
                        Ok(Err(e)) => {
                            log::error!("Panel disconnect failed: {}", e);
                            sk.set(true);
                            sw.set_active(true);
                            sk.set(false);
                        }
                        Err(e) => {
                            log::error!("Panel thread error: {:?}", e);
                        }
                    }
                });
            }

            glib::Propagation::Proceed
        });

        row.add_prefix(&toggle);
        profiles_list.append(&row);
    }

    content.append(&profiles_list);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.set_margin_top(4);
    footer.set_margin_bottom(12);
    footer.set_halign(gtk::Align::Center);

    let open_app_btn = gtk::Button::builder()
        .label("Open Full App")
        .build();
    open_app_btn.add_css_class("pill");
    open_app_btn.add_css_class("suggested-action");

    let win_ref = window.downgrade();
    open_app_btn.connect_clicked(move |_| {
        if let Some(win) = win_ref.upgrade() {
            if let Some(app) = win.application() {
                app.activate();
            }
            win.close();
        }
    });

    footer.append(&open_app_btn);
    content.append(&footer);

    scroll.set_child(Some(&content));
    toolbar.set_content(Some(&scroll));
    window.set_content(Some(&toolbar));

    if let Some(ref cp) = connected_profile {
        let cp_name = cp.name.clone();
        let da = drawing_area.clone();
        let dl_lbl = dl_speed_label.clone();
        let ul_lbl = ul_speed_label.clone();
        let bi_lbl = bytes_in_val.clone();
        let bo_lbl = bytes_out_val.clone();
        let state_timer = state.clone();

        let da_weak = da.downgrade();
        glib::timeout_add_local(Duration::from_millis(REFRESH_MS as u64), move || {
            let Some(da) = da_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };

            let cp_name2 = cp_name.clone();
            let da2 = da.clone();
            let dl2 = dl_lbl.clone();
            let ul2 = ul_lbl.clone();
            let bi2 = bi_lbl.clone();
            let bo2 = bo_lbl.clone();
            let st = state_timer.clone();

            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(move || {
                    manager::read_stats(&cp_name2)
                })
                .await;

                if let Ok(Some((bytes_in, bytes_out))) = result {
                    let mut s = st.borrow_mut();
                    if s.last_bytes_in > 0 {
                        let dl_speed = (bytes_in.saturating_sub(s.last_bytes_in)) as f64 / (REFRESH_MS as f64 / 1000.0);
                        let ul_speed = (bytes_out.saturating_sub(s.last_bytes_out)) as f64 / (REFRESH_MS as f64 / 1000.0);

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
                    s.total_bytes_in = bytes_in;
                    s.total_bytes_out = bytes_out;

                    bi2.set_label(&format_bytes(bytes_in));
                    bo2.set_label(&format_bytes(bytes_out));

                    da2.queue_draw();
                }
            });

            glib::ControlFlow::Continue
        });
    }

    window
}

fn draw_graph(cr: &gtk::cairo::Context, width: f64, height: f64, downloads: &[f64], uploads: &[f64]) {
    cr.set_source_rgba(0.15, 0.15, 0.15, 0.4);
    let _ = cr.paint();

    if downloads.is_empty() {
        return;
    }

    let max_val = downloads
        .iter()
        .chain(uploads.iter())
        .cloned()
        .fold(1.0_f64, f64::max);

    let n = downloads.len();
    let step = if n > 1 { width / (n - 1) as f64 } else { width };

    cr.set_source_rgba(0.2, 0.78, 0.4, 0.25);
    cr.move_to(0.0, height);
    for (i, &val) in downloads.iter().enumerate() {
        let x = i as f64 * step;
        let y = height - (val / max_val * height * 0.85);
        cr.line_to(x, y);
    }
    cr.line_to((n - 1) as f64 * step, height);
    cr.close_path();
    let _ = cr.fill();

    cr.set_source_rgba(0.2, 0.78, 0.4, 0.9);
    cr.set_line_width(2.0);
    for (i, &val) in downloads.iter().enumerate() {
        let x = i as f64 * step;
        let y = height - (val / max_val * height * 0.85);
        if i == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    let _ = cr.stroke();

    cr.set_source_rgba(0.93, 0.5, 0.14, 0.25);
    cr.move_to(0.0, height);
    for (i, &val) in uploads.iter().enumerate() {
        let x = i as f64 * step;
        let y = height - (val / max_val * height * 0.85);
        cr.line_to(x, y);
    }
    cr.line_to((n - 1) as f64 * step, height);
    cr.close_path();
    let _ = cr.fill();

    cr.set_source_rgba(0.93, 0.5, 0.14, 0.9);
    cr.set_line_width(2.0);
    for (i, &val) in uploads.iter().enumerate() {
        let x = i as f64 * step;
        let y = height - (val / max_val * height * 0.85);
        if i == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    let _ = cr.stroke();
}
