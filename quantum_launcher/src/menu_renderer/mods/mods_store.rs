use frostmark::MarkWidget;
use iced::{
    widget::{self, column, row},
    Length,
};
use ql_core::{Loader, ModId, StoreBackendType};
use ql_mod_manager::store::{QueryType, SearchMod};

use crate::{
    icons,
    menu_renderer::{back_button, button_with_icon, tooltip, Element, FONT_DEFAULT, FONT_MONO},
    state::{
        ImageState, InstallModsMessage, ManageModsMessage, MenuModsDownload, Message, ModOperation,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

impl MenuModsDownload {
    /// Renders the main store page, with the search bar,
    /// back button and list of searched mods.
    fn view_main<'a>(&'a self, images: &'a ImageState, tick_timer: usize) -> Element<'a> {
        let mods_list = self.get_mods_list(images, tick_timer);

        row![
            widget::scrollable(
                column!(
                    widget::text_input("Search...", &self.query)
                        .on_input(|n| Message::InstallMods(InstallModsMessage::SearchInput(n))),
                    self.get_side_panel(),
                )
                .padding(10)
                .spacing(10)
                .width(200)
            )
            .style(LauncherTheme::style_scrollable_flat_dark),
            column![
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
                ),
                (self.query_type == QueryType::Mods
                    && self.config.mod_type.is_vanilla())
                .then_some(
                    widget::container(
                        widget::text(
                            // WARN: No loader installed
                            "You haven't installed any mod loader! Install Fabric (recommended), Forge, Quilt or NeoForge"
                        ).size(12)
                    ).padding(10).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                ),
                (self.query_type == QueryType::Mods && self.version_json.is_legacy_version())
                .then_some(
                    widget::container(
                        widget::text(
                            // WARN: Store for old versions
                            "Installing Mods for old versions is experimental and may be broken"
                        ).size(12)
                    ).padding(10).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                ),
                widget::scrollable(mods_list)
                    .style(|theme: &LauncherTheme, status|
                        theme
                            .style_scrollable_flat_extra_dark(status)
                    )
                    .id(widget::Id::new("MenuModsDownload:main:mods_list"))
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .on_scroll(|viewport| {
                        Message::InstallMods(InstallModsMessage::Scrolled(viewport))
                    }),
        ]]
        .into()
    }

    fn get_side_panel(&'_ self) -> Element<'_> {
        let normal_controls = column!(
            back_button().on_press(Message::ManageMods(
                ManageModsMessage::ScreenOpenWithoutUpdate
            )),
            widget::space().height(5.0),
            widget::text("Select store:").size(18),
            widget::radio(
                "Modrinth",
                StoreBackendType::Modrinth,
                Some(self.backend),
                |v| { Message::InstallMods(InstallModsMessage::ChangeBackend(v)) }
            )
            .text_size(14)
            .size(14),
            widget::radio(
                "CurseForge",
                StoreBackendType::Curseforge,
                Some(self.backend),
                |v| { Message::InstallMods(InstallModsMessage::ChangeBackend(v)) }
            )
            .text_size(14)
            .size(14),
            widget::space().height(5),
            widget::text("Select Type:").size(18),
            widget::column(QueryType::STORE_QUERIES.iter().map(|n| {
                widget::radio(n.to_string(), *n, Some(self.query_type), |v| {
                    Message::InstallMods(InstallModsMessage::ChangeQueryType(v))
                })
                .text_size(14)
                .size(14)
                .into()
            }))
            .spacing(5)
        )
        .spacing(5);

        if self.mods_download_in_progress.is_empty() || self.results.is_none() {
            normal_controls.into()
        } else {
            // Mod operations (installing/uninstalling) are in progress.
            // Can't back out. Show list of operations in progress.
            column!("In progress:", {
                widget::column(
                    self.mods_download_in_progress
                        .values()
                        .map(|(title, operation)| {
                            const SIZE: u32 = 12;
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
                        }),
                )
                .spacing(5)
            })
            .spacing(5)
            .into()
        }
    }

    fn get_mods_list<'a>(
        &'a self,
        images: &'a ImageState,
        tick_timer: usize,
    ) -> widget::Column<'a, Message, LauncherTheme> {
        if let Some(results) = self.results.as_ref() {
            if results.mods.is_empty() {
                column!["No results found."]
            } else {
                widget::column(
                    results
                        .mods
                        .iter()
                        .enumerate()
                        .map(|(i, hit)| self.view_mod_entry(i, hit, images, results.backend)),
                )
                .spacing(5)
            }
            .push(widget::space().width(Length::Fill))
        } else {
            let dots = ".".repeat((tick_timer % 3) + 1);
            column!(widget::text!("Loading{dots}"))
        }
        .padding(10)
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

        let side_button = |icon| widget::button(row![icon].padding(5)).height(70);

        let action_button: Element = if is_installed && !is_downloading {
            // Uninstall button - darker to respect theme
            tooltip(
                side_button(icons::bin())
                    .style(|t: &LauncherTheme, s| {
                        t.style_button(s, StyleButton::SemiDarkBorder([true; 4]))
                    })
                    .on_press(Message::InstallMods(InstallModsMessage::Uninstall(i))),
                "Uninstall",
                widget::tooltip::Position::FollowCursor,
            )
            .into()
        } else {
            // Download button
            side_button(icons::download())
                .on_press_maybe(
                    (!is_downloading)
                        .then_some(Message::InstallMods(InstallModsMessage::Download(i))),
                )
                .into()
        };

        row!(
            action_button,
            widget::button(
                row!(
                    images.view(
                        &hit.icon_url,
                        Some(32.0),
                        Some(32.0),
                        column!(widget::text("...")).into()
                    ),
                    column!(
                        icons::download_s(20),
                        widget::text(Self::format_downloads(hit.downloads)).size(12),
                    )
                    .align_x(iced::Alignment::Center)
                    .width(40)
                    .height(60)
                    .spacing(5),
                    column!(
                        widget::text(&hit.title)
                            .wrapping(widget::text::Wrapping::None)
                            .shaping(widget::text::Shaping::Advanced)
                            .height(20)
                            .width(Length::Fill),
                        widget::responsive(|size| {
                            widget::text(safe_slice(&hit.description, (size.width / 7.5) as usize))
                                .font(FONT_MONO)
                                .shaping(widget::text::Shaping::Advanced)
                                .size(12)
                                .style(|t: &LauncherTheme| t.style_text(Color::Mid))
                                .into()
                        }),
                    )
                    .spacing(5),
                )
                .padding(5)
                .spacing(10),
            )
            .height(70)
            .on_press(Message::InstallMods(InstallModsMessage::Click(i)))
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
        // Parses the markdown description of the mod.
        let markdown_description = if let Some(desc) = &self.description {
            column![MarkWidget::new(desc)
                .on_clicking_link(Message::CoreOpenLink)
                .on_drawing_image(|img| { images.view(img.url, img.width, img.height, "".into()) })
                .on_updating_state(|n| Message::InstallMods(InstallModsMessage::TickDesc(n)))
                .font(FONT_DEFAULT)
                .font_mono(FONT_MONO)]
        } else {
            let dots = ".".repeat((tick_timer % 3) + 1);
            column!(widget::text!("Loading{dots}"))
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
            column![
                row![
                    back_button()
                        .on_press(Message::InstallMods(InstallModsMessage::BackToMainScreen)),
                    widget::tooltip(
                        button_with_icon(icons::globe(), "Open Mod Page", 14)
                            .on_press(Message::CoreOpenLink(url.clone())),
                        widget::text(url),
                        widget::tooltip::Position::Bottom
                    )
                    .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                    button_with_icon(icons::floppydisk(), "Copy ID", 14)
                        .on_press(Message::CoreCopyText(hit.id.clone())),
                ]
                .spacing(5),
                row![
                    images.view(&hit.icon_url, Some(32.0), Some(32.0), "".into()),
                    widget::text(&hit.title)
                        .shaping(widget::text::Shaping::Advanced)
                        .size(24)
                ]
                .spacing(10),
                widget::text(&hit.description)
                    .shaping(widget::text::Shaping::Advanced)
                    .size(20),
                markdown_description
            ]
            .padding(20)
            .spacing(20),
        )
        .style(LauncherTheme::style_scrollable_flat_extra_dark)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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
}

fn safe_slice(s: &str, max_len: usize) -> &str {
    let mut end = 0;
    for (i, _) in s.char_indices().take(max_len) {
        end = i;
    }
    if end == 0 {
        s
    } else {
        &s[..end]
    }
}
