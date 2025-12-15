use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::session::{Room, RoomCategory};

mod imp {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::RoomCategoryFilter)]
    pub struct RoomCategoryFilter {
        /// The expression to watch.
        ///
        /// This expression must return a [`RoomCategory`].
        #[property(get, set = Self::set_expression, explicit_notify, nullable)]
        expression: RefCell<Option<gtk::Expression>>,
        /// The room category to filter.
        #[property(get, set = Self::set_room_category, explicit_notify, builder(RoomCategory::default()))]
        room_category: Cell<RoomCategory>,
        /// The current space being viewed (set by the section).
        pub(in crate::session::sidebar_data::section) current_space: RefCell<Option<Room>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RoomCategoryFilter {
        const NAME: &'static str = "RoomCategoryFilter";
        type Type = super::RoomCategoryFilter;
        type ParentType = gtk::Filter;
    }

    #[glib::derived_properties]
    impl ObjectImpl for RoomCategoryFilter {}

    impl FilterImpl for RoomCategoryFilter {
        fn strictness(&self) -> gtk::FilterMatch {
            if self.expression.borrow().is_none() {
                return gtk::FilterMatch::None;
            }

            gtk::FilterMatch::Some
        }

        fn match_(&self, item: &glib::Object) -> bool {
            let room_category = self.room_category.get();

            // First check if the category matches
            let category_matches = self
                .expression
                .borrow()
                .as_ref()
                .and_then(|e| e.evaluate(Some(item)))
                .map(|v| {
                    v.get::<RoomCategory>()
                        .expect("expression returns a room category")
                })
                .is_some_and(|item_room_category| item_room_category == room_category);

            if !category_matches {
                return false;
            }

            // Apply space-based filtering
            let Some(room) = item.downcast_ref::<Room>() else {
                return true;
            };

            let current_space = self.current_space.borrow();

            if let Some(space) = current_space.as_ref() {
                // Inside a space: show only children of this space
                room.is_in_space(space.room_id())
            } else {
                // At root level: only show orphaned items (not in any space)
                room.is_orphaned()
            }
        }
    }

    impl RoomCategoryFilter {
        /// Set the expression to watch.
        ///
        /// This expression must return a [`RoomCategory`].
        fn set_expression(&self, expression: Option<gtk::Expression>) {
            let prev_expression = self.expression.borrow().clone();

            if prev_expression.is_none() && expression.is_none() {
                return;
            }
            let obj = self.obj();

            let change = if prev_expression.is_none() {
                Some(gtk::FilterChange::LessStrict)
            } else if expression.is_none() {
                Some(gtk::FilterChange::MoreStrict)
            } else {
                Some(gtk::FilterChange::Different)
            };

            self.expression.replace(expression);
            if let Some(change) = change {
                obj.changed(change);
            }
            obj.notify_expression();
        }

        /// Set the room category to filter.
        fn set_room_category(&self, category: RoomCategory) {
            let prev_category = self.room_category.get();

            if prev_category == category {
                return;
            }
            let obj = self.obj();

            let change = if self.expression.borrow().is_none() {
                None
            } else {
                Some(gtk::FilterChange::Different)
            };

            self.room_category.set(category);
            if let Some(change) = change {
                obj.changed(change);
            }
            obj.notify_room_category();
        }
    }
}

glib::wrapper! {
    /// A `GtkFilter` to filter by [`RoomCategory`].
    pub struct RoomCategoryFilter(ObjectSubclass<imp::RoomCategoryFilter>)
        @extends gtk::Filter;
}

impl RoomCategoryFilter {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl Default for RoomCategoryFilter {
    fn default() -> Self {
        Self::new()
    }
}
