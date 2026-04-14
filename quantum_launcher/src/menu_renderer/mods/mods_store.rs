use iced::{
    Alignment, Length,
    widget::{self, column, row},
};
use ql_core::Loader;
use ql_mod_manager::store::{Category, ModId, QueryType, SearchMod, StoreBackendType};

use crate::{
    icons,
    menu_renderer::{
        Column, Element, barthin, button_with_icon, mods::description::view_project_description,
        tooltip, tsubtitle,
    },
    state::{
        ImageState, InstallModsMessage, ManageModsMessage, MenuModsDownload, Message,
        ModCategoryState, ModOperation,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

const MOD_HEIGHT: u16 = 55;

impl MenuModsDownload {
    /// Renders the main store page, with the search bar,
    /// back button and list of searched mods.
    fn view_main<'a>(&'a self, images: &'a ImageState, tick_timer: usize) -> Element<'a> {
        column![
            self.get_top_bar(),
            widget::horizontal_rule(1).style(barthin),
            row![
                self.get_side_panel(tick_timer),
                self.mods_display(images, tick_timer)
            ]
        ]
        .into()
    }

    fn get_top_bar(&self) -> widget::Container<'_, Message, LauncherTheme> {
        widget::container(
            row![
                button_with_icon(icons::back_s(12), "Back", 13)
                    .padding([5, 8])
                    .on_press(ManageModsMessage::Open.into()),
                widget::text_input("Search...", &self.query)
                    .size(14)
                    .on_input(|n| InstallModsMessage::SearchInput(n).into()),
                widget::text("Store:").size(14).style(tsubtitle),
                column![
                    widget::radio(
                        "Modrinth",
                        StoreBackendType::Modrinth,
                        Some(self.backend),
                        |v| InstallModsMessage::ChangeBackend(v).into()
                    )
                    .text_size(10)
                    .size(10),
                    widget::radio(
                        "CurseForge",
                        StoreBackendType::Curseforge,
                        Some(self.backend),
                        |v| InstallModsMessage::ChangeBackend(v).into()
                    )
                    .text_size(10)
                    .size(10),
                ],
            ]
            .align_y(Alignment::Center)
            .spacing(10),
        )
        .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark))
        .padding([5, 10])
    }

    fn mods_display<'a>(&'a self, images: &'a ImageState, tick_timer: usize) -> Column<'a> {
        let mods_list = self.get_mods_list(images, tick_timer);

        self.mods_view_warnings().push(
            widget::scrollable(mods_list.spacing(5))
                .style(|theme: &LauncherTheme, status| theme.style_scrollable_flat_dark(status))
                .id(widget::scrollable::Id::new(
                    "MenuModsDownload:main:mods_list",
                ))
                .height(Length::Fill)
                .width(Length::Fill)
                .spacing(0)
                .on_scroll(|viewport| InstallModsMessage::Scrolled(viewport).into()),
        )
    }

    fn mods_view_warnings(&self) -> widget::Column<'static, Message, LauncherTheme> {
        // WARN: various mod-related stuff
        widget::Column::new()
            .push_maybe(
                (self.query_type == QueryType::Shaders
                    && self.config.mod_type != Loader::OptiFine
                    // Iris Shaders Mod
                    && !self.mod_index.mods.contains_key(&ModId::Modrinth("YL57xq9U".to_owned())) // Modrinth ID
                    && !self.mod_index.mods.contains_key(&ModId::Curseforge("455508".to_owned()))) // CurseForge ID
                .then_some(
                    column![
                        widget::text(
                            "You haven't installed any shader mod! Either install:\n- Fabric + Sodium + Iris (recommended), or\n- OptiFine"
                        ).size(12)
                    ].padding(10)
                )
            )
            .push_maybe(
                (self.query_type == QueryType::Mods
                    && self.config.mod_type.is_vanilla())
                .then_some(
                    widget::container(
                        widget::text(
                            "You haven't installed any mod loader! Install Fabric (recommended), Forge, Quilt or NeoForge"
                        ).size(12)
                    ).padding(10).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                )
            ).push_maybe((self.query_type == QueryType::Mods && self.version_json.is_legacy_version())
                .then_some(
                    widget::container(
                        widget::text(
                            "Installing Mods for old versions is experimental and may be broken"
                        ).size(12)
                    ).padding(10).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                )
            )
    }

    fn get_side_panel(
        &'_ self,
        tick_timer: usize,
    ) -> widget::Scrollable<'_, Message, LauncherTheme> {
        if !self.mods_download_in_progress.is_empty() {
            // Mod operations (installing/uninstalling) are in progress.
            // Can't back out. Show list of operations in progress.

            let operations = self
                .mods_download_in_progress
                .values()
                .map(|(title, operation)| {
                    const SIZE: u16 = 12;
                    widget::container(
                        widget::row![
                            match operation {
                                ModOperation::Downloading => icons::download_s(SIZE),
                                ModOperation::Deleting => icons::bin_s(SIZE),
                            },
                            widget::text(title).size(SIZE)
                        ]
                        .spacing(4),
                    )
                    .padding(8)
                    .into()
                });

            return widget::scrollable(
                column!["In progress:"]
                    .extend(operations)
                    .spacing(5)
                    .padding(10),
            )
            .width(180)
            .height(Length::Fill)
            .style(LauncherTheme::style_scrollable_flat_extra_dark);
        }

        widget::scrollable(
            column![
                row![icons::download_s(14), widget::text("Type:").size(18)]
                    .align_y(Alignment::Center)
                    .spacing(5),
                widget::column(QueryType::STORE_QUERIES.iter().map(|n| {
                    widget::radio(n.to_string(), *n, Some(self.query_type), |v| {
                        InstallModsMessage::ChangeQueryType(v).into()
                    })
                    .spacing(5)
                    .text_size(14)
                    .size(12)
                    .into()
                })),
                widget::Space::with_height(5),
                self.categories
                    .view(self.backend, self.force_open_source, tick_timer),
            ]
            .spacing(5)
            .padding(10),
        )
        .width(180)
        .height(Length::Fill)
        .style(LauncherTheme::style_scrollable_flat_extra_dark)
    }

    fn get_mods_list<'a>(&'a self, images: &'a ImageState, tick_timer: usize) -> Column<'a> {
        if let Some(results) = self.results.as_ref() {
            if results.mods.is_empty() {
                column!["No results found."].padding(10)
            } else {
                widget::column(
                    results
                        .mods
                        .iter()
                        .enumerate()
                        .map(|(i, hit)| self.view_mod_entry(i, hit, images, results.backend)),
                )
                .padding(5)
            }
            .push(widget::horizontal_space())
        } else {
            let dots = ".".repeat((tick_timer % 3) + 1);
            column![widget::text!("Loading{dots}")].padding(10)
        }
    }

    /// Renders a single mod entry (and button) in the search results.
    fn view_mod_entry<'a>(
        &'a self,
        i: usize,
        hit: &'a SearchMod,
        images: &'a ImageState,
        backend: StoreBackendType,
    ) -> Element<'a> {
        let is_installed = self.mod_index.mods.contains_key(&hit.get_id())
            || self
                .mod_index
                .mods
                .values()
                .any(|n| n.name == hit.title && n.project_source != backend);
        let is_downloading = self
            .mods_download_in_progress
            .contains_key(&ModId::from_pair(&hit.id, backend));

        let action_button: Element = action_button(i, hit, is_installed, is_downloading);

        row!(
            action_button,
            widget::button(
                row![
                    images.view(hit.icon_url.as_deref(), Some(32.0), Some(32.0)),
                    column![
                        widget::text(&hit.title)
                            .wrapping(widget::text::Wrapping::None)
                            .shaping(widget::text::Shaping::Advanced)
                            .height(19),
                        widget::text(&hit.description)
                            .wrapping(widget::text::Wrapping::None)
                            .shaping(widget::text::Shaping::Advanced)
                            .size(12)
                            .style(tsubtitle),
                    ]
                    .spacing(2),
                ]
                .padding(8)
                .spacing(16),
            )
            .height(MOD_HEIGHT)
            .width(Length::Fill)
            .padding(0)
            .on_press(InstallModsMessage::Click(i).into())
        )
        .spacing(5)
        .into()
    }

    pub fn view<'a>(&'a self, images: &'a ImageState, tick_timer: usize) -> Element<'a> {
        // If we opened a mod (`self.opened_mod`) then
        // render the mod description page.
        // else render the main store page.
        let (Some(selection), Some(results)) = (self.opened_mod, &self.results) else {
            return self.view_main(images, tick_timer);
        };
        let Some(hit) = results.mods.get(selection) else {
            return self.view_main(images, tick_timer);
        };
        // If a specific mod was selected, show the mod description page
        view_project_description(
            Ok::<_, &str>(&self.description),
            self.backend,
            InstallModsMessage::BackToMainScreen,
            hit,
            images,
            tick_timer,
        )
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

fn format_downloads(downloads: usize) -> String {
    if downloads < 999 {
        downloads.to_string()
    } else if downloads < 10000 {
        format!("{:.1}K", downloads as f32 / 1000.0)
    } else if downloads < 1_000_000 {
        format!("{}K", downloads / 1000)
    } else if downloads < 10_000_000 {
        format!("{:.1}M", downloads as f32 / 1_000_000.0)
    } else {
        format!("{}M", downloads / 1_000_000)
    }
}

fn action_button(
    i: usize,
    hit: &SearchMod,
    is_installed: bool,
    is_downloading: bool,
) -> Element<'static> {
    const WIDTH: u16 = 40;

    if is_installed && !is_downloading {
        // Uninstall button - darker to respect theme
        tooltip(
            widget::button(
                column![icons::bin()]
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
            )
            .padding(10)
            .width(WIDTH)
            .height(MOD_HEIGHT)
            .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::SemiDarkBorder([true; 4])))
            .on_press(InstallModsMessage::Uninstall(i).into()),
            "Uninstall",
            widget::tooltip::Position::FollowCursor,
        )
        .into()
    } else {
        // Download button
        widget::button(
            widget::center(
                column![
                    icons::download(),
                    widget::text(format_downloads(hit.downloads))
                        .size(10)
                        .style(tsubtitle),
                ]
                .spacing(5)
                .align_x(Alignment::Center),
            )
            .style(|_| widget::container::Style::default()),
        )
        .width(WIDTH)
        .height(MOD_HEIGHT)
        .padding(0)
        .on_press_maybe((!is_downloading).then_some(InstallModsMessage::Download(i).into()))
        .into()
    }
}
