use iced::{
    Alignment, Length,
    widget::{self, column, row},
};
use ql_mod_manager::store::{Category, StoreBackendType};

use crate::{
    icons,
    menu_renderer::{Column, Element, barthin, mods::mods_store::color_from_backend, tsubtitle},
    state::{
        InstallModsMessage, MenuModsDownload, ModCategoryState, ModOperation, ModsDownloadSearch,
    },
    stylesheet::{
        color::Color,
        styles::{LauncherTheme, lerp_color},
    },
};

impl MenuModsDownload {
    pub(super) fn get_side_panel(&self, tick_timer: usize) -> Element<'_> {
        let s = |t: &LauncherTheme, s| {
            let mut style = t.style_scrollable_flat_extra_dark(s);
            style.container.background = Some(iced::Background::Color(lerp_color(
                style
                    .container
                    .background
                    .map(|n| match n {
                        iced::Background::Color(color) => color,
                        iced::Background::Gradient(_) => unreachable!(),
                    })
                    .unwrap_or_default(),
                color_from_backend(self.search.backend),
                0.07,
            )));
            style
        };

        widget::responsive(move |size| {
            column![
                widget::scrollable(self.search.view_categories(tick_timer))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(s),
            ]
            .push_maybe(
                self.get_ongoing_operations(size.height)
                    .map(|n| column![widget::horizontal_rule(1).style(barthin), n]),
            )
            .into()
        })
        .into()
    }

    /// Show list of mod operations (installing/uninstalling) in progress.
    fn get_ongoing_operations(&'_ self, space_y: f32) -> Option<Element<'_>> {
        let list = &self.mods_download_in_progress;
        if list.is_empty() {
            return None;
        }
        let operations = list
            .values()
            .filter(|(_, op)| matches!(op, ModOperation::Downloading))
            .map(|(title, _)| {
                const SIZE: u16 = 12;
                widget::row![icons::download_s(SIZE), widget::text(title).size(SIZE)]
                    .spacing(4)
                    .into()
            });

        Some(
            widget::container(
                widget::scrollable(
                    column!["In progress:"]
                        .extend(operations)
                        .spacing(5)
                        .padding(10),
                )
                .width(180)
                .style(LauncherTheme::style_scrollable_flat_extra_dark),
            )
            .max_height(space_y / 2.0)
            .into(),
        )
    }
}

impl ModsDownloadSearch {
    fn view_categories(&self, tick_timer: usize) -> Column<'_> {
        self.categories
            .view(self.backend, self.force_open_source, tick_timer)
    }
}

impl ModCategoryState {
    fn view(&self, backend: StoreBackendType, open_source: bool, tick_timer: usize) -> Column<'_> {
        let category_view: Element = match &self.categories {
            Ok(n) if n.is_empty() => {
                let dots = ".".repeat((tick_timer % 3) + 1);
                widget::text!("Loading{dots}").into()
            }
            Ok(n) => widget::column(n.iter().map(|n| self.view_category(n).into()))
                .spacing(4)
                .into(),
            Err(err) => widget::text(err).size(12).style(tsubtitle).into(),
        };

        let show_any_all = backend.can_pick_any_or_all();
        let m = |n| InstallModsMessage::CategoriesUseAll(n).into();

        column![
            row![icons::filter(), widget::text("Filters:").size(20)]
                .push_maybe(show_any_all.then(|| {
                    column![
                        widget::radio("All", true, Some(self.use_all), m)
                            .spacing(5)
                            .text_size(12)
                            .size(10),
                        widget::radio("Any", false, Some(self.use_all), m)
                            .spacing(5)
                            .text_size(12)
                            .size(10)
                    ]
                }))
                .spacing(10)
                .align_y(Alignment::Center),
        ]
        .push_maybe(backend.can_filter_open_source().then(|| {
            widget::checkbox("Open-source only", open_source)
                .size(14)
                .text_size(14)
                .style(|n: &LauncherTheme, s| n.style_checkbox(s, Some(Color::SecondLight)))
                .on_toggle(|n| InstallModsMessage::ForceOpenSource(n).into())
        }))
        .push(widget::Space::new(0, 0))
        .push(category_view)
        .spacing(5)
        .padding(10)
    }

    fn view_category<'a>(&'a self, category: &'a Category) -> Column<'a> {
        widget::Column::new()
            .spacing(2)
            .push_maybe(category.is_usable.then(|| {
                widget::checkbox(&category.name, self.selected.contains(&category.slug))
                    .on_toggle(|_| {
                        InstallModsMessage::CategoriesToggle(category.slug.clone()).into()
                    })
                    .size(14)
                    .text_size(14)
                    .style(|n: &LauncherTheme, s| n.style_checkbox(s, Some(Color::Light)))
            }))
            .push_maybe((!category.is_usable).then(|| widget::text(&category.name).size(14)))
            .push_maybe((!category.children.is_empty()).then(|| {
                widget::stack!(
                    widget::column(
                        category
                            .children
                            .iter()
                            .map(|n| self.view_category(n).into())
                    )
                    .spacing(2)
                    .padding(iced::Padding::new(0.0).left(13.0).top(2.0).bottom(5.0)),
                    widget::vertical_rule(1).style(barthin)
                )
            }))
    }
}
