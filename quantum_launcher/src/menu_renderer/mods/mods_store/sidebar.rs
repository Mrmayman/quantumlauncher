use iced::{
    Alignment, Length,
    widget::{self, column, row},
};
use ql_mod_manager::store::{Category, QueryType, StoreBackendType};

use crate::{
    icons,
    menu_renderer::{Column, Element, barthin, tsubtitle},
    state::{
        InstallModsMessage, MenuModsDownload, Message, ModCategoryState, ModOperation,
        ModsDownloadSearch,
    },
    stylesheet::{color::Color, styles::LauncherTheme},
};

impl MenuModsDownload {
    pub(super) fn get_side_panel(&'_ self, tick_timer: usize) -> Column<'_> {
        column![
            widget::scrollable(
                column![
                    self.get_store_selector(),
                    widget::horizontal_rule(1).style(barthin),
                    row![icons::download_s(14), widget::text("Type:").size(18)]
                        .align_y(Alignment::Center)
                        .spacing(5),
                    widget::column(QueryType::STORE_QUERIES.iter().map(|n| {
                        widget::radio(n.to_string(), *n, Some(self.search.query_type), |v| {
                            InstallModsMessage::ChangeQueryType(v).into()
                        })
                        .spacing(5)
                        .text_size(14)
                        .size(12)
                        .into()
                    })),
                    widget::Space::with_height(5),
                    self.search.view_categories(tick_timer),
                ]
                .spacing(5)
                .padding(10),
            )
            .width(180)
            .height(Length::Fill)
            .style(LauncherTheme::style_scrollable_flat_extra_dark),
        ]
        .push_maybe(
            self.get_ongoing_operations()
                .map(|n| column![widget::horizontal_rule(1).style(barthin), n].width(180)),
        )
    }

    fn get_store_selector(&self) -> widget::Row<'static, Message, LauncherTheme> {
        let radio = |b: StoreBackendType| {
            widget::radio(b.desc(), b, Some(self.search.backend), |v| {
                InstallModsMessage::ChangeBackend(v).into()
            })
            .text_size(12)
            .size(12)
        };

        row![
            widget::text("Store:").size(14),
            column![
                radio(StoreBackendType::Modrinth),
                radio(StoreBackendType::Curseforge),
            ],
        ]
        .spacing(10)
        .align_y(Alignment::Center)
    }

    fn get_ongoing_operations(&'_ self) -> Option<Element<'_>> {
        if !self.mods_download_in_progress.is_empty() {
            // Mod operations (installing/uninstalling) are in progress.
            // Can't back out. Show list of operations in progress.

            let operations = self
                .mods_download_in_progress
                .values()
                .map(|(title, operation)| {
                    const SIZE: u16 = 12;
                    widget::row![
                        match operation {
                            ModOperation::Downloading => icons::download_s(SIZE),
                            ModOperation::Deleting => icons::bin_s(SIZE),
                        },
                        widget::text(title).size(SIZE)
                    ]
                    .spacing(4)
                    .into()
                });

            Some(
                widget::scrollable(
                    column!["In progress:"]
                        .extend(operations)
                        .spacing(5)
                        .padding(10),
                )
                .width(180)
                .height(Length::Fill)
                .style(LauncherTheme::style_scrollable_flat_extra_dark)
                .into(),
            )
        } else {
            None
        }
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
            Ok(n) => widget::column(n.iter().map(|n| self.view_category(n).into())).into(),
            Err(err) => widget::text(err).size(12).style(tsubtitle).into(),
        };

        let show_any_all = backend.can_pick_any_or_all();
        let m = |n| InstallModsMessage::CategoriesUseAll(n).into();

        column![
            row![icons::filter_s(14), widget::text("Filters:").size(18)]
                // TODO
                .push_maybe(show_any_all.then(|| {
                    widget::radio("All", true, Some(self.use_all), m)
                        .spacing(4)
                        .text_size(13)
                        .size(11)
                }))
                .push_maybe(show_any_all.then(|| {
                    widget::radio("Any", false, Some(self.use_all), m)
                        .spacing(4)
                        .text_size(13)
                        .size(11)
                }))
                .spacing(5)
                .align_y(Alignment::Center),
        ]
        .push_maybe(backend.can_filter_open_source().then(|| {
            widget::checkbox("Open-source only", open_source)
                .size(12)
                .text_size(12)
                .style(|n: &LauncherTheme, s| n.style_checkbox(s, Some(Color::SecondLight)))
                .on_toggle(|n| InstallModsMessage::ForceOpenSource(n).into())
        }))
        .push(category_view)
        .spacing(5)
    }

    fn view_category<'a>(&'a self, category: &'a Category) -> Column<'a> {
        widget::Column::new()
            .push_maybe(category.is_usable.then(|| {
                widget::checkbox(&category.name, self.selected.contains(&category.slug))
                    .on_toggle(|_| {
                        InstallModsMessage::CategoriesToggle(category.slug.clone()).into()
                    })
                    .size(12)
                    .text_size(14)
                    .style(|n: &LauncherTheme, s| n.style_checkbox(s, Some(Color::SecondLight)))
            }))
            .push_maybe((!category.is_usable).then(|| widget::text(&category.name).size(14)))
            .push(widget::stack!(
                row![
                    widget::Space::with_width(10),
                    widget::column(
                        category
                            .children
                            .iter()
                            .map(|n| self.view_category(n).into())
                    )
                ],
                widget::vertical_rule(1).style(barthin)
            ))
    }
}
