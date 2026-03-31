use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::window::OpenvpnGuiWindow;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct OpenvpnGuiApp;

    #[glib::object_subclass]
    impl ObjectSubclass for OpenvpnGuiApp {
        const NAME: &'static str = "OpenvpnGuiApp";
        type Type = super::OpenvpnGuiApp;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for OpenvpnGuiApp {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
        }
    }

    impl ApplicationImpl for OpenvpnGuiApp {
        fn activate(&self) {
            let app = self.obj();
            if let Some(window) = app.active_window() {
                window.set_visible(true);
                window.present();
                return;
            }
            let window = OpenvpnGuiWindow::new(&app.clone().upcast::<adw::Application>());
            window.present();
        }
    }

    impl GtkApplicationImpl for OpenvpnGuiApp {}
    impl AdwApplicationImpl for OpenvpnGuiApp {}
}

glib::wrapper! {
    pub struct OpenvpnGuiApp(ObjectSubclass<imp::OpenvpnGuiApp>)
        @extends adw::Application, gtk::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl OpenvpnGuiApp {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", "com.github.m1theusr.OpenVPNGUI")
            .property("flags", gio::ApplicationFlags::FLAGS_NONE)
            .build()
    }

    fn setup_actions(&self) {
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| {
                app.show_about();
            })
            .build();

        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| {
                app.quit();
            })
            .build();

        self.add_action_entries([about_action, quit_action]);
        self.set_accels_for_action("app.quit", &["<primary>q"]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutWindow::builder()
            .transient_for(&window)
            .application_name("OpenVPN GUI")
            .application_icon("openvpn-gui")
            .developer_name("m1theusr")
            .version("0.1.0")
            .developers(vec!["m1theusr"])
            .copyright("© 2026 m1theusr")
            .license_type(gtk::License::Gpl30)
            .website("https://github.com/m1theusr/openvpn-gui")
            .build();
        about.present();
    }
}
