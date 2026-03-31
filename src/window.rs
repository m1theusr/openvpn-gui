use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate};

use crate::profile::import::import_profile_dialog;
use crate::profile::model::VpnProfile;
use crate::profile::row::create_profile_row;
use crate::profile::storage;

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/m1theusr/OpenVPNGUI/window.ui")]
    pub struct OpenvpnGuiWindow {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub import_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub status_banner: TemplateChild<gtk::Box>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub profiles_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub content_box: TemplateChild<gtk::Box>,
        pub profiles: RefCell<Vec<VpnProfile>>,
        pub row_count: RefCell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for OpenvpnGuiWindow {
        const NAME: &'static str = "OpenvpnGuiWindow";
        type Type = super::OpenvpnGuiWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for OpenvpnGuiWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_import_button();
            obj.load_profiles();
        }
    }

    impl WidgetImpl for OpenvpnGuiWindow {}
    impl WindowImpl for OpenvpnGuiWindow {
        fn close_request(&self) -> glib::Propagation {
            let window = self.obj().clone();

            let gtk_window: gtk::Window = window.clone().upcast();
            let dialog = adw::MessageDialog::new(
                Some(&gtk_window),
                Some("Close Application"),
                Some("What would you like to do?"),
            );
            dialog.add_response("minimize", "Minimize to Tray");
            dialog.add_response("quit", "Quit");
            dialog.set_response_appearance("quit", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("minimize"));
            dialog.set_close_response("minimize");

            let win = window.clone();
            dialog.connect_response(None, move |_, response| {
                match response {
                    "minimize" => {
                        win.set_visible(false);
                    }
                    "quit" => {
                        if let Some(app) = win.application() {
                            app.quit();
                        }
                    }
                    _ => {}
                }
            });
            dialog.present();

            glib::Propagation::Stop
        }
    }
    impl ApplicationWindowImpl for OpenvpnGuiWindow {}
    impl AdwApplicationWindowImpl for OpenvpnGuiWindow {}
}

glib::wrapper! {
    pub struct OpenvpnGuiWindow(ObjectSubclass<imp::OpenvpnGuiWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl OpenvpnGuiWindow {
    pub fn new(app: &adw::Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    fn setup_import_button(&self) {
        let window = self.clone();
        self.imp().import_button.connect_clicked(move |_| {
            import_profile_dialog(&window);
        });
    }

    pub fn load_profiles(&self) {
        let profiles = storage::load_profiles();
        let imp = self.imp();

        let pbox = &*imp.profiles_box;
        while let Some(child) = pbox.first_child() {
            pbox.remove(&child);
        }
        *imp.row_count.borrow_mut() = 0;

        if profiles.is_empty() {
            imp.status_page.set_visible(true);
            imp.profiles_box.set_visible(false);
            imp.status_banner.set_visible(false);
        } else {
            imp.status_page.set_visible(false);
            imp.profiles_box.set_visible(true);
            imp.status_banner.set_visible(true);

            let has_connected = profiles.iter().any(|p| crate::vpn::manager::is_connected(&p.name));
            if has_connected {
                imp.status_label.set_label("CONNECTED");
                imp.status_label.remove_css_class("dim-label");
                imp.status_label.add_css_class("success");
            }

            for profile in &profiles {
                let section = create_profile_row(profile, self);
                imp.profiles_box.append(&section);
                *imp.row_count.borrow_mut() += 1;
            }
        }

        *imp.profiles.borrow_mut() = profiles;
    }

    pub fn add_profile(&self, profile: VpnProfile) {
        let imp = self.imp();
        imp.status_page.set_visible(false);
        imp.profiles_box.set_visible(true);
        imp.status_banner.set_visible(true);

        let section = create_profile_row(&profile, self);
        imp.profiles_box.append(&section);
        *imp.row_count.borrow_mut() += 1;
        imp.profiles.borrow_mut().push(profile);

        storage::save_profiles(&imp.profiles.borrow());
    }

    pub fn remove_profile(&self, name: &str) {
        {
            let imp = self.imp();
            imp.profiles.borrow_mut().retain(|p| p.name != name);
            storage::save_profiles(&imp.profiles.borrow());
        }
        self.load_profiles();
    }

    pub fn add_toast(&self, message: &str) {
        let toast = adw::Toast::new(message);
        toast.set_timeout(3);
        self.imp().toast_overlay.add_toast(toast);
    }

    pub fn update_status_banner(&self, text: &str, _icon_name: &str) {
        let imp = self.imp();
        imp.status_label.set_label(text);
        imp.status_label.remove_css_class("dim-label");
        imp.status_label.remove_css_class("success");
        imp.status_label.remove_css_class("warning");
        if text.contains("CONNECTED") && !text.contains("DIS") {
            imp.status_label.add_css_class("success");
        } else if text.contains("CONNECTING") {
            imp.status_label.add_css_class("warning");
        } else {
            imp.status_label.add_css_class("dim-label");
        }
    }

    pub fn save_profile_credentials(&self, name: &str, username: &str, password: &str) {
        let imp = self.imp();
        let mut profiles = imp.profiles.borrow_mut();
        if let Some(profile) = profiles.iter_mut().find(|p| p.name == name) {
            profile.username = Some(username.to_string());
            profile.password = Some(password.to_string());
        }
        storage::save_profiles(&profiles);
    }
}
