use iced::{widget, Length};
use ql_core::{ModId, StoreBackendType};
use ql_mod_manager::store::{QueryType, SearchMod};

use crate::{
    icon_manager,
    menu_renderer::{back_button, button_with_icon, Element, FONT_MONO},
    state::{ImageState, InstallModsMessage, ManageModsMessage, MenuModsDownload, Message},
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

mod helpers;
mod html;
mod markdown;

impl MenuModsDownload {
    /// Renders the main store page, with the search bar,
    /// back button and list of searched mods.
    fn view_main<'a>(&'a self, images: &'a ImageState, tick_timer: usize) -> Element<'a> {
        let mods_list = self.get_mods_list(images, tick_timer);

        widget::row!(
            widget::scrollable(
                widget::column!(
                    widget::text_input("Search...", &self.query)
                        .on_input(|n| Message::InstallMods(InstallModsMessage::SearchInput(n))),
                    self.get_side_panel(),
                )
                .padding(10)
                .spacing(10)
                .width(200)
            )
            .style(LauncherTheme::style_scrollable_flat_dark),
            widget::Column::new()
                .push_maybe(
                    (self.query_type == QueryType::Shaders
                        && self.config.mod_type != "OptiFine"

                        // Iris Shaders Mod
                        && !self.mod_index.mods.contains_key("YL57xq9U") // Modrinth ID
                        && !self.mod_index.mods.contains_key("CF:455508")) // CurseForge ID
                    .then_some(
                        widget::column![
                            widget::text(
                                "You haven't installed any shader mod! Either install:\n- Fabric + Sodium + Iris (recommended), or\n- OptiFine"
                            ).size(12)
                        ].padding(10)
                    )
                )
                .push_maybe(
                    (self.query_type == QueryType::Mods
                        && self.config.mod_type == "Vanilla")
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
                    widget::scrollable(mods_list.spacing(10).padding(10))
                        .style(|theme: &LauncherTheme, status|
                            theme
                                .style_scrollable_flat_extra_dark(status)
                        )
                        .id(widget::scrollable::Id::new("MenuModsDownload:main:mods_list"))
                        .height(Length::Fill)
                        .width(Length::Fill)
                        .on_scroll(|viewport| {
                            Message::InstallMods(InstallModsMessage::Scrolled(viewport))
                        }),
                )
        )
        .into()
    }

    fn get_side_panel(&'_ self) -> Element<'_> {
        let normal_controls = widget::column!(
            back_button().on_press(Message::ManageMods(
                ManageModsMessage::ScreenOpenWithoutUpdate
            )),
            widget::Space::with_height(5.0),
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
            widget::Space::with_height(5),
            widget::text("Select Type:").size(18),
            widget::column(QueryType::ALL.iter().map(|n| {
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
            // Mods are being installed. Can't back out.
            // Show list of mods being installed.
            widget::column!("Installing:", {
                widget::column(
                    self.mods_download_in_progress
                        .values()
                        .map(|title| widget::text!("- {title}").into()),
                )
            })
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
                widget::column!["No results found."]
            } else {
                widget::column(
                    results
                        .mods
                        .iter()
                        .enumerate()
                        .map(|(i, hit)| self.view_mod_entry(i, hit, images, results.backend)),
                )
            }
            .push(widget::horizontal_space())
        } else {
            let dots = ".".repeat((tick_timer % 3) + 1);
            widget::column!(widget::text!("Loading{dots}"))
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
        widget::row!(
            widget::button(
                widget::row![icon_manager::download()]
                    .spacing(10)
                    .padding(5)
            )
            .height(70)
            .on_press_maybe(
                (!self
                    .mods_download_in_progress
                    .contains_key(&ModId::from_pair(&hit.id, backend))
                    && !self.mod_index.mods.contains_key(&hit.id)
                    && !self.mod_index.mods.values().any(|n| n.name == hit.title))
                .then_some(Message::InstallMods(InstallModsMessage::Download(i)))
            ),
            widget::button(
                widget::row!(
                    images.view(
                        &hit.icon_url,
                        Some(32),
                        widget::column!(widget::text("...")).into()
                    ),
                    widget::column!(
                        icon_manager::download_with_size(20),
                        widget::text(Self::format_downloads(hit.downloads)).size(12),
                    )
                    .align_x(iced::Alignment::Center)
                    .width(40)
                    .height(60)
                    .spacing(5),
                    widget::column!(
                        widget::text(&hit.title).size(16),
                        widget::text(safe_slice(&hit.description, 50)).size(12),
                    )
                    .spacing(5),
                    widget::horizontal_space()
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

    pub fn view<'a>(
        &'a self,
        images: &'a ImageState,
        window_size: (f32, f32),
        tick_timer: usize,
    ) -> Element<'a> {
        // If we opened a mod (`self.opened_mod`) then
        // render the mod description page.
        // else render the main store page.
        let (Some(selection), Some(results)) = (&self.opened_mod, &self.results) else {
            return self.view_main(images, tick_timer);
        };
        let Some(hit) = results.mods.get(*selection) else {
            return self.view_main(images, tick_timer);
        };
        self.view_project_description(hit, images, window_size, results.backend, tick_timer)
    }

    /// Renders the mod description page.
    fn view_project_description<'a>(
        &'a self,
        hit: &'a SearchMod,
        images: &'a ImageState,
        window_size: (f32, f32),
        backend: StoreBackendType,
        tick_timer: usize,
    ) -> Element<'a> {
        // Parses the markdown description of the mod.
        let markdown_description = if let Some(info) = self
            .mod_descriptions
            .get(&ModId::from_pair(&hit.id, backend))
        {
            widget::column!(Self::render_markdown(info, images, window_size))
        } else {
            let dots = ".".repeat((tick_timer % 3) + 1);
            widget::column!(widget::text!("Loading...{dots}"))
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
            widget::column!(
                widget::row!(
                    back_button()
                        .on_press(Message::InstallMods(InstallModsMessage::BackToMainScreen)),
                    widget::tooltip(
                        button_with_icon(icon_manager::globe(), "Open Mod Page", 14)
                            .on_press(Message::CoreOpenLink(url.clone())),
                        widget::text(url),
                        widget::tooltip::Position::Bottom
                    )
                    .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                    button_with_icon(icon_manager::save(), "Copy ID", 14)
                        .on_press(Message::CoreCopyText(hit.id.clone())),
                )
                .spacing(5),
                widget::row!(
                    images.view(&hit.icon_url, None, "".into()),
                    widget::text(&hit.title).size(24)
                )
                .spacing(10),
                widget::text(&hit.description).size(20),
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

pub fn codeblock<'a>(e: String) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(widget::text(e.clone()).font(FONT_MONO))
        .on_press(Message::CoreCopyText(e))
        .padding(0)
        .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatDark))
}
