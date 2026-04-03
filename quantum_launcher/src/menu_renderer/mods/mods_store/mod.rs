use iced::{
    Alignment, Length,
    widget::{self, column, row},
};
use ql_core::Loader;
use ql_mod_manager::store::{ModId, QueryType, SearchMod, StoreBackendType};

use crate::{
    icons,
    menu_renderer::{
        Element, barthin, mods::description::view_project_description, tooltip, tsubtitle,
    },
    state::{ImageState, InstallModsMessage, MenuModsDownload, Message},
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
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
                self.get_side_panel(tick_timer),
                self.mods_display(images, tick_timer)
            ]
        ]
        .into()
    }

    fn mods_display<'a>(
        &'a self,
        images: &'a ImageState,
        tick_timer: usize,
    ) -> widget::Column<'a, Message, LauncherTheme> {
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
