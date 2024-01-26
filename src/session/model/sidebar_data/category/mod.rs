use gtk::{
    gio, glib,
    glib::{clone, closure},
    prelude::*,
    subclass::prelude::*,
};

mod category_filter;
mod category_type;

use self::category_filter::CategoryFilter;
pub use self::category_type::CategoryType;
use super::{SidebarItem, SidebarItemExt, SidebarItemImpl};
use crate::{
    session::model::{Room, RoomList, RoomType},
    utils::ExpressionListModel,
};

#[derive(Debug, Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "CategorySortCriteria")]
pub enum CategorySortCriteria {
    #[default]
    LastMessage = 0,
    Name = 1,
}

impl TryFrom<&str> for CategorySortCriteria {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "last-message" => Ok(CategorySortCriteria::LastMessage),
            "name" => Ok(CategorySortCriteria::Name),
            _ => Err("Unexpected CategorySortCriteria"),
        }
    }
}

mod imp {
    use std::{
        cell::{Cell, OnceCell},
        marker::PhantomData,
    };

    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Category)]
    pub struct Category {
        /// The filter list model of this category.
        #[property(get, set = Self::set_model, construct_only)]
        pub model: OnceCell<gio::ListModel>,
        /// The filter of this category.
        pub filter: CategoryFilter,
        pub sort_model: gtk::SortListModel,
        pub filter_model: gtk::FilterListModel,
        /// The type of this category.
        #[property(get = Self::category_type, set = Self::set_category_type, construct_only, builder(CategoryType::default()))]
        pub category_type: PhantomData<CategoryType>,
        #[property(get, set = Self::set_sort_criteria, builder(CategorySortCriteria::default()))]
        pub sort_criteria: Cell<CategorySortCriteria>,
        /// Whether this category is empty.
        #[property(get)]
        pub empty: Cell<bool>,
        /// The display name of this category.
        #[property(get = Self::display_name)]
        pub display_name: PhantomData<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Category {
        const NAME: &'static str = "Category";
        type Type = super::Category;
        type ParentType = SidebarItem;
        type Interfaces = (gio::ListModel,);
    }

    #[glib::derived_properties]
    impl ObjectImpl for Category {}

    impl ListModelImpl for Category {
        fn item_type(&self) -> glib::Type {
            SidebarItem::static_type()
        }

        fn n_items(&self) -> u32 {
            self.model.get().unwrap().n_items()
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.model.get().unwrap().item(position)
        }
    }

    impl SidebarItemImpl for Category {
        fn update_visibility(&self, for_category: CategoryType) {
            let obj = self.obj();

            let visible = if !obj.empty() {
                true
            } else {
                let room_types =
                    RoomType::try_from(for_category)
                        .ok()
                        .and_then(|source_room_type| {
                            RoomType::try_from(obj.category_type())
                                .ok()
                                .map(|target_room_type| (source_room_type, target_room_type))
                        });

                room_types.is_some_and(|(source_room_type, target_room_type)| {
                    source_room_type.can_change_to(target_room_type)
                })
            };

            obj.set_visible(visible)
        }
    }

    impl Category {
        /// Set the filter list model of this category.
        fn set_model(&self, model: gio::ListModel) {
            let obj = self.obj();

            // Special case room lists so that they are sorted and in the right category
            let model = if model.is::<RoomList>() {
                let room_category_type = Room::this_expression("category")
                    .chain_closure::<CategoryType>(closure!(
                        |_: Option<glib::Object>, room_type: RoomType| {
                            CategoryType::from(room_type)
                        }
                    ));
                self.filter
                    .set_expression(Some(room_category_type.clone().upcast()));

                let category_type_expr_model = ExpressionListModel::new();
                category_type_expr_model.set_expressions(vec![room_category_type.upcast()]);
                category_type_expr_model.set_model(Some(model));

                self.filter_model.set_model(Some(&category_type_expr_model));
                self.filter_model.set_filter(Some(&self.filter));

                // The rest will be set in update_sorter()

                self.sort_model.clone().upcast()
            } else {
                model
            };

            model.connect_items_changed(clone!(@weak obj => move |model, pos, removed, added| {
                obj.items_changed(pos, removed, added);
                obj.imp().set_empty(model.n_items() == 0);
            }));

            self.set_empty(model.n_items() == 0);
            self.model.set(model).unwrap();
            self.update_sorter();
        }

        fn update_sorter(&self) {
            match self.sort_criteria.get() {
                CategorySortCriteria::LastMessage => {
                    let room_latest_activity = Room::this_expression("latest-activity");
                    let sorter = gtk::NumericSorter::builder()
                        .expression(&room_latest_activity)
                        .sort_order(gtk::SortType::Descending)
                        .build();

                    let latest_activity_expr_model = ExpressionListModel::new();
                    latest_activity_expr_model.set_expressions(vec![room_latest_activity.upcast()]);
                    latest_activity_expr_model.set_model(Some(self.filter_model.clone().upcast()));
                    self.sort_model.set_model(Some(&latest_activity_expr_model));
                    self.sort_model.set_sorter(Some(&sorter));
                }
                CategorySortCriteria::Name => {
                    let room_display_name = Room::this_expression("display-name");
                    let sorter = gtk::StringSorter::builder()
                        .expression(&room_display_name)
                        .build();

                    let display_name_expr_model = ExpressionListModel::new();
                    display_name_expr_model.set_expressions(vec![room_display_name.upcast()]);
                    display_name_expr_model.set_model(Some(self.filter_model.clone().upcast()));
                    self.sort_model.set_model(Some(&display_name_expr_model));
                    self.sort_model.set_sorter(Some(&sorter));
                }
            }
        }

        /// The type of this category.
        fn category_type(&self) -> CategoryType {
            self.filter.category_type()
        }

        /// Set the type of this category.
        fn set_category_type(&self, type_: CategoryType) {
            self.filter.set_category_type(type_);
        }

        fn set_sort_criteria(&self, sort_criteria: CategorySortCriteria) {
            self.sort_criteria.set(sort_criteria);
            self.obj().notify_sort_criteria();
            self.update_sorter();
        }

        /// Set whether this category is empty.
        fn set_empty(&self, empty: bool) {
            if empty == self.empty.get() {
                return;
            }

            self.empty.set(empty);
            self.obj().notify_empty();
        }

        /// The display name of this category.
        fn display_name(&self) -> String {
            self.category_type().to_string()
        }
    }
}

glib::wrapper! {
    /// A list of Items in the same category, implementing ListModel.
    ///
    /// This struct is used in ItemList for the sidebar.
    pub struct Category(ObjectSubclass<imp::Category>)
        @extends SidebarItem,
        @implements gio::ListModel;
}

impl Category {
    pub fn new(category_type: CategoryType, model: &impl IsA<gio::ListModel>) -> Self {
        glib::Object::builder()
            .property("category-type", category_type)
            .property("model", model)
            .build()
    }
}
