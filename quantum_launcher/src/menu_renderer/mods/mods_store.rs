use frostmark::MarkWidget;
use iced::{
    widget::{self, column, row},
    Alignment, Length,
};
use ql_core::{Loader, ModId, StoreBackendType};
use ql_mod_manager::store::{QueryType, SearchMod};

use crate::{
    icons,
    menu_renderer::{
        back_button, button_with_icon, tooltip, tsubtitle, Element, FONT_DEFAULT, FONT_MONO,
    },
    state::{
        ImageState, InstallModsMessage, ManageModsMessage, MenuModsDownload, Message, ModOperation,
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
            widget::horizontal_rule(1)
                .style(|t: &LauncherTheme| t.style_rule(Color::SecondDark, 1)),
            row![self.get_side_panel(), self.mods_display(images, tick_timer)]
        ]
        .into()
    }

    fn get_top_bar(&self) -> widget::Container<'_, Message, LauncherTheme> {
        widget::container(
            row![
                button_with_icon(icons::back_s(12), "Back", 13)
                    .padding([4, 8])
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
        .padding(5)
    }

    fn mods_display<'a>(
        &'a self,
        images: &'a ImageState,
        tick_timer: usize,
    ) -> widget::Column<'a, Message, LauncherTheme> {
        let mods_list = self.get_mods_list(images, tick_timer);

        widget::Column::new()
            .push_maybe(
                (self.query_type == QueryType::Shaders
                    && self.config.mod_type != Loader::OptiFine

                    // Iris Shaders Mod
                    && !self.mod_index.mods.contains_key("YL57xq9U") // Modrinth ID
                    && !self.mod_index.mods.contains_key("CF:455508")) // CurseForge ID
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
                            // WARN: No loader installed
                            "You haven't installed any mod loader! Install Fabric (recommended), Forge, Quilt or NeoForge"
                        ).size(12)
                    ).padding(10).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                )
            ).push_maybe((self.query_type == QueryType::Mods && self.version_json.is_legacy_version())
                .then_some(
                    widget::container(
                        widget::text(
                            // WARN: Store for old versions
                            "Installing Mods for old versions is experimental and may be broken"
                        ).size(12)
                    ).padding(10).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                )
            ).push(
                widget::scrollable(mods_list.spacing(5))
                    .style(|theme: &LauncherTheme, status| theme.style_scrollable_flat_dark(status))
                    .id(widget::scrollable::Id::new("MenuModsDownload:main:mods_list"))
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .spacing(0)
                    .on_scroll(|viewport| InstallModsMessage::Scrolled(viewport).into()),
            )
    }

    fn get_side_panel(&'_ self) -> widget::Scrollable<'_, Message, LauncherTheme> {
        let inner =
            if self.mods_download_in_progress.is_empty() || self.results.is_none() {
                column![
                    widget::text("Type:").size(14),
                    widget::column(QueryType::STORE_QUERIES.iter().map(|n| {
                        widget::radio(n.to_string(), *n, Some(self.query_type), |v| {
                            InstallModsMessage::ChangeQueryType(v).into()
                        })
                        .spacing(5)
                        .text_size(12)
                        .size(10)
                        .into()
                    })),
                    widget::Space::with_height(5),
                    row![
                        widget::text("Filters:").size(14),
                        widget::horizontal_space(),
                        widget::radio("All", true, Some(false), |_| Message::Nothing)
                            .spacing(2)
                            .text_size(12)
                            .size(10),
                        widget::radio("Any", false, Some(false), |_| Message::Nothing)
                            .spacing(2)
                            .text_size(12)
                            .size(10),
                    ]
                    .spacing(7)
                    .align_y(Alignment::Center),
                    "(TODO)"
                ]
                .spacing(5)
            } else {
                // Mod operations (installing/uninstalling) are in progress.
                // Can't back out. Show list of operations in progress.
                column!("In progress:", {
                    widget::column(self.mods_download_in_progress.values().map(
                        |(title, operation)| {
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
                        },
                    ))
                    .spacing(5)
                })
                .spacing(5)
            }
            .padding(10);

        widget::scrollable(inner)
            .width(150)
            .height(Length::Fill)
            .style(LauncherTheme::style_scrollable_flat_extra_dark)
    }

    fn get_mods_list<'a>(
        &'a self,
        images: &'a ImageState,
        tick_timer: usize,
    ) -> widget::Column<'a, Message, LauncherTheme> {
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
        let is_installed = self
            .mod_index
            .mods
            .contains_key(&hit.get_id(backend).get_index_str())
            || self.mod_index.mods.values().any(|n| n.name == hit.title);
        let is_downloading = self
            .mods_download_in_progress
            .contains_key(&ModId::from_pair(&hit.id, backend));

        let action_button: Element = action_button(i, hit, is_installed, is_downloading);

        row!(
            action_button,
            widget::button(
                row![
                    images.view(
                        &hit.icon_url,
                        Some(32.0),
                        Some(32.0),
                        column!(widget::text("...")).into()
                    ),
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
        self.view_project_description(hit, images, tick_timer)
    }

    /// Renders the mod description page.
    fn view_project_description<'a>(
        &'a self,
        hit: &'a SearchMod,
        images: &'a ImageState,
        tick_timer: usize,
    ) -> Element<'a> {
        // Parses the Markdown description of the mod.
        let markdown_description = if let Some(desc) = &self.description {
            column!(MarkWidget::new(desc)
                .on_clicking_link(Message::CoreOpenLink)
                .on_drawing_image(|img| { images.view(img.url, img.width, img.height, "".into()) })
                .on_updating_state(|n| InstallModsMessage::TickDesc(n).into())
                .font(FONT_DEFAULT)
                .font_mono(FONT_MONO))
        } else {
            let dots = ".".repeat((tick_timer % 3) + 1);
            column!(widget::text!("Loading...{dots}"))
        };

        let url = format!(
            "{}{}/{}",
            match self.backend {
                StoreBackendType::Modrinth => "https://modrinth.com/",
                StoreBackendType::Curseforge => "https://www.curseforge.com/minecraft/",
            },
            hit.project_type,
            hit.internal_name
        );

        widget::scrollable(
            column!(
                row!(
                    back_button().on_press(InstallModsMessage::BackToMainScreen.into()),
                    widget::tooltip(
                        button_with_icon(icons::globe(), "Open Mod Page", 14)
                            .on_press(Message::CoreOpenLink(url.clone())),
                        widget::text(url),
                        widget::tooltip::Position::Bottom
                    )
                    .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                    button_with_icon(icons::floppydisk(), "Copy ID", 14)
                        .on_press(Message::CoreCopyText(hit.id.clone())),
                )
                .spacing(5),
                row!(
                    images.view(&hit.icon_url, Some(32.0), Some(32.0), "".into()),
                    widget::text(&hit.title)
                        .shaping(widget::text::Shaping::Advanced)
                        .size(24)
                )
                .spacing(10),
                widget::text(&hit.description)
                    .shaping(widget::text::Shaping::Advanced)
                    .size(20),
                markdown_description
            )
            .padding(20)
            .spacing(20),
        )
        .style(LauncherTheme::style_scrollable_flat_extra_dark)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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
