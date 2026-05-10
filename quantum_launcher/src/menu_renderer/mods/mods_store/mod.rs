use iced::{
    Alignment, Length,
    widget::{self, column, row},
};
use ql_core::Loader;
use ql_mod_manager::store::{ModId, QueryType, SearchMod, SearchSortBy, StoreBackendType};

use crate::{
    icons,
    menu_renderer::{
        Column, Element, barthin, button_with_icon, mods::description::view_project_description,
        tooltip, tsubtitle,
    },
    state::{ImageState, InstallModsMessage, ManageModsMessage, MenuModsDownload, Message},
    stylesheet::{
        color::Color,
        styles::{BORDER_RADIUS, BORDER_WIDTH, LauncherTheme, lerp_color},
        widgets::StyleButton,
    },
};

const MOD_HEIGHT: u16 = 55;

mod sidebar;

impl MenuModsDownload {
    /// Renders the main store page, with the search bar,
    /// back button and list of searched mods.
    fn view_main<'a>(&'a self, images: &'a ImageState, tick_timer: usize) -> Element<'a> {
        column![
            self.get_top_bar(),
            widget::horizontal_rule(1).style(barthin),
            row![
                widget::container(self.get_side_panel(tick_timer))
                    .style(|_| widget::container::Style::default())
                    .width(190),
                widget::vertical_rule(1).style(barthin),
                self.mods_display(images, tick_timer)
            ],
            widget::horizontal_rule(1).style(barthin),
            self.get_type_selector(),
        ]
        .into()
    }

    fn get_type_selector(&self) -> widget::Container<'static, Message, LauncherTheme> {
        let qty = self.search.query_type;

        widget::container(
            row![widget::text("Type:  ").size(14)]
                .extend(QueryType::STORE_QUERIES.iter().map(|n| {
                    const SIZE: u16 = 12;

                    let is_selected = self.search.query_type == *n;

                    let icon = match n {
                        QueryType::Mods => icons::file_jar_s(SIZE),
                        QueryType::ResourcePacks => icons::paintbrush_s(SIZE),
                        QueryType::Shaders => icons::mode_light_s(SIZE),
                        QueryType::ModPacks => icons::folder_s(SIZE),
                        QueryType::DataPacks => icons::edit_s(SIZE),
                    };

                    let color = color_from_querytype(*n);

                    widget::button(row![icon, widget::text(n.to_string()).size(SIZE)].spacing(5))
                        .padding([4, 8])
                        .style(move |t: &LauncherTheme, s| widget::button::Style {
                            background: Some(iced::Background::Color(color.scale_alpha(match s {
                                // Unselected
                                widget::button::Status::Active => 0.0,
                                widget::button::Status::Hovered => 0.04,
                                widget::button::Status::Pressed => 0.1,
                                // Selected
                                widget::button::Status::Disabled => 0.4,
                            }))),
                            text_color: t.get(Color::Light),
                            border: iced::Border {
                                color: color.scale_alpha(0.7),
                                width: BORDER_WIDTH,
                                radius: BORDER_RADIUS.into(),
                            },
                            shadow: iced::Shadow::default(),
                        })
                        .on_press_maybe(
                            (!is_selected)
                                .then_some(InstallModsMessage::ChangeQueryType(*n).into()),
                        )
                        .into()
                }))
                .spacing(5)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .wrap(),
        )
        .style(move |t: &LauncherTheme| widget::container::Style {
            border: iced::Border {
                color: t.get(Color::Mid),
                width: 0.0,
                radius: 0.0.into(),
            },
            background: Some(iced::Background::Color(lerp_color(
                t.get(Color::Dark),
                color_from_querytype(qty),
                0.1,
            ))),
            ..Default::default()
        })
        .padding([5, 10])
    }

    fn get_top_bar(&self) -> widget::Container<'_, Message, LauncherTheme> {
        let s = &self.search;

        widget::container(
            row![
                button_with_icon(icons::back_s(12), "Back", 13)
                    .padding([5, 8])
                    .on_press_maybe(
                        self.mods_download_in_progress
                            .is_empty()
                            .then_some(ManageModsMessage::Open.into())
                    ),
                widget::text_input("Search...", &s.term)
                    .size(14)
                    .on_input(|n| InstallModsMessage::SearchInput(n).into()),
                row![
                    widget::text("Sort by:").size(14).style(tsubtitle),
                    widget::pick_list(
                        SearchSortBy::default_choices(s.backend),
                        Some(s.sort_by),
                        |s| InstallModsMessage::ChangeSortBy(s).into()
                    )
                    .text_size(12)
                    .width(130)
                    .padding([4, 6])
                ]
                .push_maybe(
                    s.backend
                        .can_sort_ascending()
                        .then(|| sort_ascending_button(s)),
                )
                .spacing(5)
                .align_y(Alignment::Center)
            ]
            .align_y(Alignment::Center)
            .spacing(10),
        )
        .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark))
        .padding([5, 10])
    }

    fn mods_display<'a>(&'a self, images: &'a ImageState, tick_timer: usize) -> Column<'a> {
        let mods_list = self.get_mods_list(images, tick_timer);

        self.mods_view_warnings()
            .push(self.get_store_selector())
            .push(
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

    fn get_store_selector(&self) -> Element<'static> {
        let selector = |b: StoreBackendType| {
            let is_selected = self.search.backend == b;
            let color = color_from_backend(b);

            widget::button(
                widget::Row::new()
                    .push_maybe(is_selected.then_some(icons::checkmark_s(12)))
                    .push(widget::text(b.desc()).size(12))
                    .spacing(5),
            )
            .padding([4, 8])
            .style(move |t: &LauncherTheme, s| widget::button::Style {
                background: Some(iced::Background::Color(color.scale_alpha(match s {
                    // Unselected
                    widget::button::Status::Active => 0.0,
                    widget::button::Status::Hovered => 0.04,
                    widget::button::Status::Pressed => 0.1,
                    // Selected
                    widget::button::Status::Disabled => 0.2,
                }))),
                text_color: t.get(Color::Light),
                border: iced::Border {
                    color,
                    width: BORDER_WIDTH,
                    radius: BORDER_RADIUS.into(),
                },
                shadow: iced::Shadow::default(),
            })
            .on_press_maybe((!is_selected).then_some(InstallModsMessage::ChangeBackend(b).into()))
        };

        row![
            widget::text("Store:").size(14),
            selector(StoreBackendType::Modrinth),
            selector(StoreBackendType::Curseforge),
        ]
        .padding([5, 10])
        .spacing(5)
        .align_y(Alignment::Center)
        .wrap()
        .into()
    }

    fn mods_view_warnings(&self) -> Column<'static> {
        let q = self.search.query_type;
        // WARN: various mod-related stuff
        widget::Column::new()
            .push_maybe(
                (q == QueryType::Shaders
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
                (q == QueryType::Mods && self.config.mod_type.is_vanilla())
                .then_some(
                    widget::container(
                        widget::text(
                            "You haven't installed any mod loader! Install Fabric (recommended), Forge, Quilt or NeoForge"
                        ).size(12)
                    ).padding(10).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                )
            ).push_maybe((q == QueryType::Mods && self.version_json.is_legacy_version())
                .then_some(
                    widget::container(
                        widget::text(
                            "Installing Mods for old versions is experimental and may be broken"
                        ).size(12)
                    ).padding(10).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                )
            )
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
        let is_downloading = self.mods_download_in_progress.contains_key(&hit.get_id());

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
            InstallModsMessage::BackToMainScreen,
            hit,
            images,
            tick_timer,
        )
    }
}

fn color_from_querytype(n: QueryType) -> iced::Color {
    match n {
        QueryType::Mods => iced::Color::from_rgb8(0xF5, 0x9E, 0x0B),
        QueryType::ResourcePacks => iced::Color::from_rgb8(0x22, 0xC5, 0x5E),
        QueryType::Shaders => iced::Color::from_rgb8(0x8B, 0x5C, 0xF6),
        QueryType::ModPacks => iced::Color::from_rgb8(0xEF, 0x44, 0x44),
        QueryType::DataPacks => iced::Color::from_rgb8(0x06, 0xB6, 0xD4),
    }
}

fn color_from_backend(n: StoreBackendType) -> iced::Color {
    match n {
        StoreBackendType::Modrinth => iced::Color::from_rgb8(0x19, 0x7d, 0x43),
        StoreBackendType::Curseforge => iced::Color::from_rgb8(0xeb, 0x62, 0x2b),
    }
}

fn sort_ascending_button(
    s: &crate::state::ModsDownloadSearch,
) -> widget::Tooltip<'_, Message, LauncherTheme> {
    tooltip(
        widget::button(if s.sort_ascending {
            icons::sort_ascend_s(12)
        } else {
            icons::sort_descend_s(12)
        })
        .padding([4, 8])
        .on_press(InstallModsMessage::ChangeSortAscending(!s.sort_ascending).into()),
        if s.sort_ascending {
            "Ascending"
        } else {
            "Descending"
        },
        widget::tooltip::Position::Bottom,
    )
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
