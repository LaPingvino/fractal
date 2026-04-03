use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::{glib, glib::clone};

use super::{Explore, Invite, InviteRequest, RoomHistory};
use crate::{
    components::PillSourceExt,
    identity_verification_view::IdentityVerificationView,
    session::{
        IdentityVerification, Room, RoomCategory, Session, SidebarIconItem, SidebarIconItemType,
    },
    spawn,
    utils::BoundObject,
};

/// A page of the content stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentPage {
    /// The placeholder page when no content is presented.
    Empty,
    /// The history of the selected room.
    RoomHistory,
    /// The selected invite request.
    InviteRequest,
    /// The selected room invite.
    Invite,
    /// The explore page.
    Explore,
    /// The selected identity verification.
    Verification,
    /// The space details and management page.
    SpaceDetails,
}

impl ContentPage {
    /// The name of this page.
    const fn name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::RoomHistory => "room-history",
            Self::InviteRequest => "invite-request",
            Self::Invite => "invite",
            Self::Explore => "explore",
            Self::Verification => "verification",
            Self::SpaceDetails => "space-details",
        }
    }

    /// Get the page matching the given name.
    ///
    /// Panics if the name does not match any of the variants.
    fn from_name(name: &str) -> Self {
        match name {
            "empty" => Self::Empty,
            "room-history" => Self::RoomHistory,
            "invite-request" => Self::InviteRequest,
            "invite" => Self::Invite,
            "explore" => Self::Explore,
            "verification" => Self::Verification,
            "space-details" => Self::SpaceDetails,
            _ => panic!("Unknown ContentPage: {name}"),
        }
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/org/gnome/Fractal/ui/session_view/content.ui")]
    #[properties(wrapper_type = super::Content)]
    pub struct Content {
        #[template_child]
        stack: TemplateChild<gtk::Stack>,
        #[template_child]
        room_history: TemplateChild<RoomHistory>,
        #[template_child]
        invite_request: TemplateChild<InviteRequest>,
        #[template_child]
        invite: TemplateChild<Invite>,
        #[template_child]
        explore: TemplateChild<Explore>,
        #[template_child]
        empty_page: TemplateChild<adw::ToolbarView>,
        #[template_child]
        empty_page_header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        verification_page: TemplateChild<adw::ToolbarView>,
        #[template_child]
        verification_page_header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        identity_verification_widget: TemplateChild<IdentityVerificationView>,
        #[template_child]
        space_details_page: TemplateChild<adw::ToolbarView>,
        #[template_child]
        space_details_header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        space_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        child_rooms_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        child_rooms_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        suggested_rooms_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        suggested_rooms_group: TemplateChild<adw::PreferencesGroup>,
        /// The current session.
        #[property(get, set = Self::set_session, explicit_notify, nullable)]
        session: glib::WeakRef<Session>,
        /// Whether this is the only visible view, i.e. there is no sidebar.
        #[property(get, set)]
        only_view: Cell<bool>,
        item_binding: RefCell<Option<glib::Binding>>,
        /// The item currently displayed.
        #[property(get, set = Self::set_item, explicit_notify, nullable)]
        item: BoundObject<glib::Object>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Content {
        const NAME: &'static str = "Content";
        type Type = super::Content;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.set_accessible_role(gtk::AccessibleRole::Group);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Content {
        fn constructed(&self) {
            self.parent_constructed();

            self.stack.connect_visible_child_notify(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_| {
                    if imp.visible_page() != ContentPage::Verification {
                        imp.identity_verification_widget
                            .set_verification(None::<IdentityVerification>);
                    }
                }
            ));
        }

        fn dispose(&self) {
            if let Some(binding) = self.item_binding.take() {
                binding.unbind();
            }
        }
    }

    impl WidgetImpl for Content {}

    impl NavigationPageImpl for Content {
        fn hidden(&self) {
            self.obj().set_item(None::<glib::Object>);
        }
    }

    impl Content {
        /// The visible page of the content.
        pub(super) fn visible_page(&self) -> ContentPage {
            ContentPage::from_name(
                &self
                    .stack
                    .visible_child_name()
                    .expect("Content stack should always have a visible child name"),
            )
        }

        /// Set the visible page of the content.
        fn set_visible_page(&self, page: ContentPage) {
            if self.visible_page() == page {
                return;
            }

            self.stack.set_visible_child_name(page.name());
        }

        /// Set the current session.
        fn set_session(&self, session: Option<&Session>) {
            if session == self.session.upgrade().as_ref() {
                return;
            }
            let obj = self.obj();

            if let Some(binding) = self.item_binding.take() {
                binding.unbind();
            }

            if let Some(session) = session {
                let item_binding = session
                    .sidebar_list_model()
                    .selection_model()
                    .bind_property("selected-item", &*obj, "item")
                    .sync_create()
                    .bidirectional()
                    .build();

                self.item_binding.replace(Some(item_binding));
            }

            self.session.set(session);
            obj.notify_session();
        }

        /// Set the item currently displayed.
        fn set_item(&self, item: Option<glib::Object>) {
            if self.item.obj() == item {
                return;
            }

            self.item.disconnect_signals();

            if let Some(item) = item {
                let handler = if let Some(room) = item.downcast_ref::<Room>() {
                    let category_handler = room.connect_category_notify(clone!(
                        #[weak(rename_to = imp)]
                        self,
                        move |_| {
                            imp.update_visible_child();
                        }
                    ));

                    Some(category_handler)
                } else if let Some(verification) = item.downcast_ref::<IdentityVerification>() {
                    let dismiss_handler = verification.connect_dismiss(clone!(
                        #[weak(rename_to = imp)]
                        self,
                        move |_| {
                            imp.set_item(None);
                        }
                    ));

                    Some(dismiss_handler)
                } else {
                    None
                };

                self.item.set(item, handler.into_iter().collect());
            }

            self.update_visible_child();
            self.obj().notify_item();

            if let Some(page) = self.stack.visible_child() {
                page.grab_focus();
            }
        }

        /// Update the visible child according to the current item.
        fn update_visible_child(&self) {
            let Some(item) = self.item.obj() else {
                self.set_visible_page(ContentPage::Empty);
                return;
            };

            if let Some(room) = item.downcast_ref::<Room>() {
                match room.category() {
                    RoomCategory::Knocked => {
                        self.invite_request.set_room(Some(room.clone()));
                        self.set_visible_page(ContentPage::InviteRequest);
                    }
                    RoomCategory::Invited => {
                        self.invite.set_room(Some(room.clone()));
                        self.set_visible_page(ContentPage::Invite);
                    }
                    RoomCategory::Space => {
                        self.update_space_details(room);
                        self.set_visible_page(ContentPage::SpaceDetails);
                    }
                    _ => {
                        self.room_history.set_timeline(Some(room.live_timeline()));
                        self.set_visible_page(ContentPage::RoomHistory);
                    }
                }
            } else if item
                .downcast_ref::<SidebarIconItem>()
                .is_some_and(|i| i.item_type() == SidebarIconItemType::Explore)
            {
                self.set_visible_page(ContentPage::Explore);
            } else if let Some(verification) = item.downcast_ref::<IdentityVerification>() {
                self.identity_verification_widget
                    .set_verification(Some(verification.clone()));
                self.set_visible_page(ContentPage::Verification);
            }
        }

        /// Update the space details page with information about the given space.
        fn update_space_details(&self, space: &Room) {
            // Set the space name
            self.space_name_label.set_text(&space.display_name());

            // Clear existing lists
            while let Some(child) = self.child_rooms_list.first_child() {
                self.child_rooms_list.remove(&child);
            }
            while let Some(child) = self.suggested_rooms_list.first_child() {
                self.suggested_rooms_list.remove(&child);
            }

            // Get child rooms that user has joined
            let Some(session) = space.session() else {
                return;
            };
            let room_list = session.room_list();

            let mut child_count = 0;
            for child_id in space.child_rooms().iter() {
                if let Some(child_room) = room_list.get(child_id) {
                    let row = adw::ActionRow::builder()
                        .title(child_room.display_name())
                        .activatable(true)
                        .build();

                    // Add an icon based on room type
                    let icon = if child_room.is_space() {
                        gtk::Image::from_icon_name("folder-symbolic")
                    } else if child_room.is_direct() {
                        gtk::Image::from_icon_name("person-symbolic")
                    } else {
                        gtk::Image::from_icon_name("chat-symbolic")
                    };
                    icon.add_css_class("dim-label");
                    row.add_prefix(&icon);

                    // Navigate to the room when activated
                    row.connect_activated(clone!(
                        #[weak(rename_to = imp)]
                        self,
                        #[strong]
                        child_room,
                        move |_| {
                            if let Some(session_view) = imp
                                .obj()
                                .ancestor(crate::session_view::SessionView::static_type())
                                .and_downcast::<crate::session_view::SessionView>()
                            {
                                session_view.select_room(child_room.clone());
                            }
                        }
                    ));

                    // Add a navigation arrow for accessibility
                    let arrow = gtk::Image::from_icon_name("go-next-symbolic");
                    arrow.add_css_class("dim-label");
                    row.add_suffix(&arrow);

                    self.child_rooms_list.append(&row);
                    child_count += 1;
                }
            }

            // Hide the group if no child rooms
            self.child_rooms_group.set_visible(child_count > 0);

            // Get suggested (unjoined) rooms
            let suggested = space.suggested_rooms();
            let suggested_count = suggested.len();

            for chunk in suggested {
                let room_id = chunk.summary.room_id.clone();
                let room_name = chunk
                    .summary
                    .name
                    .unwrap_or_else(|| chunk.summary.room_id.to_string());
                let row = adw::ActionRow::builder()
                    .title(&room_name)
                    .subtitle(&chunk.summary.topic.unwrap_or_default())
                    .build();

                // Add a room type icon as prefix
                let icon = gtk::Image::from_icon_name("chat-symbolic");
                icon.add_css_class("dim-label");
                row.add_prefix(&icon);

                // Add a Join button as suffix
                let join_button = gtk::Button::builder()
                    .label(&gettext("Join"))
                    .valign(gtk::Align::Center)
                    .build();
                join_button.add_css_class("suggested-action");
                join_button.add_css_class("pill");

                join_button.connect_clicked(clone!(
                    #[weak(rename_to = imp)]
                    self,
                    #[strong]
                    room_id,
                    move |button| {
                        button.set_sensitive(false);
                        if let Some(session) = imp.session.upgrade() {
                            spawn!(clone!(
                                #[weak]
                                session,
                                #[weak]
                                button,
                                #[strong]
                                room_id,
                                async move {
                                    let room_list = session.room_list();
                                    match room_list
                                        .join_by_id_or_alias(room_id.into(), vec![])
                                        .await
                                    {
                                        Ok(_) => {
                                            button.set_label(&gettext("Joined"));
                                        }
                                        Err(error) => {
                                            tracing::error!("Failed to join room: {error}");
                                            button.set_sensitive(true);
                                        }
                                    }
                                }
                            ));
                        }
                    }
                ));

                row.add_suffix(&join_button);

                self.suggested_rooms_list.append(&row);
            }

            // Hide the group if no suggested rooms
            self.suggested_rooms_group.set_visible(suggested_count > 0);
        }

        /// Handle a paste action.
        pub(super) fn handle_paste_action(&self) {
            if self.visible_page() == ContentPage::RoomHistory {
                self.room_history.handle_paste_action();
            }
        }

        /// All the header bars of the children of the content.
        pub(super) fn header_bars(&self) -> [&adw::HeaderBar; 7] {
            [
                &self.empty_page_header_bar,
                self.room_history.header_bar(),
                self.invite_request.header_bar(),
                self.invite.header_bar(),
                self.explore.header_bar(),
                &self.verification_page_header_bar,
                &self.space_details_header_bar,
            ]
        }
    }
}

glib::wrapper! {
    /// A view displaying the selected content in the sidebar.
    pub struct Content(ObjectSubclass<imp::Content>)
        @extends gtk::Widget, adw::NavigationPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Content {
    pub fn new(session: &Session) -> Self {
        glib::Object::builder().property("session", session).build()
    }

    /// Handle a paste action.
    pub(crate) fn handle_paste_action(&self) {
        self.imp().handle_paste_action();
    }

    /// All the header bars of the children of the content.
    pub(crate) fn header_bars(&self) -> [&adw::HeaderBar; 7] {
        self.imp().header_bars()
    }
}
