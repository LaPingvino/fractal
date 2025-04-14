use adw::{prelude::*, subclass::prelude::*};
use gtk::{glib, CompositeTemplate};

#[derive(Default)]
pub(crate) enum MessageIcon {
    #[default]
    Info,
    Warning,
}

impl MessageIcon {
    fn icon_name(self) -> &'static str {
        match self {
            MessageIcon::Info => "info-symbolic",
            MessageIcon::Warning => "warning-symbolic",
        }
    }
}

mod imp {
    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(
        resource = "/org/gnome/Fractal/ui/session/view/content/room_history/message_row/info.ui"
    )]
    pub struct MessageInfo {
        #[template_child]
        icon: TemplateChild<gtk::Image>,
        #[template_child]
        description: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MessageInfo {
        const NAME: &'static str = "ContentMessageInfo";
        type Type = super::MessageInfo;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MessageInfo {}
    impl WidgetImpl for MessageInfo {}
    impl BinImpl for MessageInfo {}

    impl MessageInfo {
        pub(super) fn set_icon(&self, icon: MessageIcon) {
            self.icon.set_icon_name(Some(icon.icon_name()));
        }

        pub(super) fn set_text(&self, text: String) {
            self.description.set_text(&text);
        }
    }
}

glib::wrapper! {
    /// A widget presenting an informative event.
    pub struct MessageInfo(ObjectSubclass<imp::MessageInfo>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl MessageInfo {
    pub fn new() -> Self {
        let obj: Self = glib::Object::new();
        obj.imp().set_icon(Some(MessageIcon::Info));
        obj.imp().set_text("");
        obj
    }
}
