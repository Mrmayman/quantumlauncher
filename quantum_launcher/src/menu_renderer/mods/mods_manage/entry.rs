use iced::{
    Alignment,
    widget::{self, row, tooltip::Position},
};
use ql_mod_manager::store::{ModConfig, ModId, QueryType, SelectedMod};

use crate::{
    menu_renderer::{Element, FONT_MONO, select_box, tooltip},
    state::{ImageState, ManageModsMessage, MenuEditMods, Message, ModListEntry},
    stylesheet::{color::Color, styles::LauncherTheme},
};

const PADDING: iced::Padding = iced::Padding {
    top: 4.0,
    bottom: 6.0,
    right: 15.0,
    left: 20.0,
};
const ICON_SIZE: f32 = 18.0;
const SPACING: u16 = 16;

/// Pixel width of a single glyph in JetBrains Mono at our UI sizes.
/// This is relied upon for manual column alignment.
/// If font/size changes, THIS MUST BE UPDATED.
const MONO_CHAR_WIDTH: f32 = 7.2;

impl MenuEditMods {
    pub(super) fn render_mod_entry<'a>(
        &'a self,
        entry: &'a ModListEntry,
        size: iced::Size,
        images: &'a ImageState,
    ) -> Element<'a> {
        match entry {
            ModListEntry::Downloaded { id, config } => {
                self.render_downloaded_mod_entry(size, images, id, config)
            }
            ModListEntry::Local(local) => self.render_local_mod_entry(size, local),
        }
    }

    fn render_local_mod_entry<'a>(
        &'a self,
        size: iced::Size,
        local: &'a ql_mod_manager::store::LocalMod,
    ) -> Element<'a> {
        let file_name = &*local.0;
        let project_type = local.1;

        let is_enabled = !file_name.ends_with(".disabled");
        let is_selected = self
            .selection
            .selected_mods
            .contains(&SelectedMod::Local(local.clone()));

        let label = file_name.strip_suffix(".disabled").unwrap_or(file_name);
        let label_len = label.len();

        select_box(
            row![
                mod_toggler_or_indicator(
                    project_type,
                    move |_| ManageModsMessage::ToggleOneLocal(local.clone()).into(),
                    is_enabled,
                    true
                ),
                widget::text(label)
                    .font(FONT_MONO)
                    .shaping(widget::text::Shaping::Advanced)
                    .style(mod_name_style(is_enabled, local.1))
                    .size(13)
            ]
            .push_maybe({
                // Measure the length of the text
                // then from there measure the space it would occupy
                // (only possible because monospace font)

                // This is for finding the filler space
                //
                // ║ some_mod.jar              ║
                // ║ some_other_mod.jar        ║
                //
                //  ╙═╦═══════════════╜  ╙═╦═══╜
                //  Measured by:         What we want
                //  `label_len`          to find

                let measured: f32 = (label_len as f32) * MONO_CHAR_WIDTH;
                let occupied = measured + PADDING.left + PADDING.right + 100.0;
                let space = size.width - occupied;
                (space > 0.0).then_some(widget::Space::with_width(space))
            })
            .padding(PADDING)
            .spacing(SPACING),
            is_selected,
            ManageModsMessage::SelectMod(local.0.clone(), None, project_type).into(),
        )
        .padding(0)
        .into()
    }

    fn render_downloaded_mod_entry<'a>(
        &self,
        size: iced::Size,
        images: &ImageState,
        id: &'a ModId,
        config: &'a ModConfig,
    ) -> Element<'a> {
        let is_enabled = config.enabled;
        let is_selected = self
            .selection
            .selected_mods
            .contains(&SelectedMod::Downloaded {
                name: config.name.clone(),
                id: (*id).clone(),
            });

        let image = config
            .icon_url
            .as_ref()
            .map(|url| images.view(Some(url), Some(ICON_SIZE), Some(ICON_SIZE)))
            .unwrap_or_else(empty_icon);

        let toggle: Element = mod_toggler_or_indicator(
            config.project_type,
            move |_| ManageModsMessage::ToggleOne(id.clone()).into(),
            is_enabled,
            config.manually_installed,
        );

        let select = select_box(
            widget::row![
                toggle,
                image,
                widget::Space::with_width(1),
                widget::text(&*config.name)
                    .shaping(widget::text::Shaping::Advanced)
                    .style(mod_name_style(is_enabled, config.project_type))
                    .size(14)
                    .width(self.ui_state.width_name),
                widget::text(&config.installed_version)
                    .style(|t: &LauncherTheme| t.style_text(Color::Mid))
                    .font(FONT_MONO)
                    .size(12)
            ]
            .push_maybe({
                // Measure the length of the text
                // then from there measure the space it would occupy
                // (only possible because monospace font)

                // This is for finding the filler space
                //
                // ║ Some Mod         v0.0.1                ║
                // ║ Some other mod   2.4.1-fabric          ║
                //
                //  ╙═╦══════════════╜            ╙═╦══════╜
                //  Measured by:                   What we want
                //  `self.width_name`              to find

                let measured: f32 = (config.installed_version.len() as f32) * MONO_CHAR_WIDTH;
                let occupied =
                    measured + self.ui_state.width_name + PADDING.left + PADDING.right + 150.0;
                let space = size.width - occupied;
                (space > -10.0).then_some(widget::Space::with_width(space))
            })
            .align_y(Alignment::Center)
            .padding(PADDING)
            .spacing(SPACING),
            is_selected,
            ManageModsMessage::SelectMod(
                config.name.clone(),
                Some(id.clone()),
                config.project_type,
            )
            .into(),
        )
        .padding(0);

        self.with_mod_right_click(id, config, select).into()
    }

    fn with_mod_right_click<'a>(
        &self,
        id: &ModId,
        config: &ModConfig,
        entry: widget::Button<'a, Message, LauncherTheme>,
    ) -> widget::MouseArea<'a, Message, LauncherTheme> {
        let right_click_msg = ManageModsMessage::RightClick(id.clone()).into();

        widget::mouse_area(entry).on_right_press(
            if self.selection.selected_mods.len() > 1 && self.is_selected(id) {
                right_click_msg
            } else {
                Message::Multiple(vec![
                    ManageModsMessage::SelectEnsure(
                        config.name.clone(),
                        Some(id.clone()),
                        config.project_type,
                    )
                    .into(),
                    right_click_msg,
                ])
            },
        )
    }
}

fn empty_icon() -> Element<'static> {
    widget::Column::new()
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .into()
}

fn mod_toggler_or_indicator<'a>(
    project_type: QueryType,
    f: impl Fn(bool) -> Message + 'a,
    is_enabled: bool,
    manually_installed: bool,
) -> Element<'a> {
    let mut size = 14;

    let (label, tooltip_text, color) = match project_type {
        QueryType::Mods => {
            if manually_installed {
                return widget::toggler(is_enabled).on_toggle(f).size(14).into();
            }
            size = 12;
            (
                " Dep",
                "Dependency",
                iced::Color::from_rgb8(0x4E, 0x6E, 0x8A),
            )
        }
        QueryType::Shaders => ("  S", "Shader", iced::Color::from_rgb8(0xB8, 0x6E, 0x3C)),
        QueryType::ModPacks => ("  M", "Modpack", iced::Color::from_rgb8(0x6E, 0x5A, 0x8A)),
        QueryType::DataPacks => ("  D", "Datapack", iced::Color::from_rgb8(0xA4, 0x4E, 0x4E)),
        QueryType::ResourcePacks => (
            "  R",
            "Resource Pack",
            iced::Color::from_rgb8(0x5E, 0x7D, 0x61),
        ),
    };
    tooltip(
        widget::text(label).size(size).color(color).width(36),
        tooltip_text,
        Position::FollowCursor,
    )
    .into()
}

fn mod_name_style(
    enabled: bool,
    project_type: QueryType,
) -> impl Fn(&LauncherTheme) -> widget::text::Style {
    move |t: &LauncherTheme| {
        // Don't want any state bugs, do we?
        t.style_text(if enabled || !project_type.is_toggleable() {
            Color::SecondLight
        } else {
            Color::Mid
        })
    }
}
