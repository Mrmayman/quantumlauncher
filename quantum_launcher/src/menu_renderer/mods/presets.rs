use std::collections::HashSet;

use iced::{
    widget::{self, column, row},
    Length,
};
use ql_core::SelectedMod;

use crate::{
    icons,
    menu_renderer::{back_button, button_with_icon, tsubtitle, Element},
    state::{
        EditPresetsMessage, ManageModsMessage, MenuEditPresets, MenuRecommendedMods, Message,
        ModListEntry, RecommendedModMessage, SelectedState,
    },
    stylesheet::{color::Color, styles::LauncherTheme},
};

impl MenuEditPresets {
    pub fn view(&'_ self) -> Element<'_> {
        if let Some(progress) = &self.progress {
            return column![
                widget::text("Installing mods").size(20),
                progress.view(),
                widget::text("Check debug log (at the bottom) for more info").size(12),
            ]
            .padding(10)
            .spacing(10)
            .into();
        }

        if self.is_building {
            return column![widget::text("Building Preset").size(20)]
                .padding(10)
                .spacing(10)
                .into();
        }

        let p_main = row![
            column![
                back_button().on_press(ManageModsMessage::Open.into()),
                widget::text(
                    r"Mod Presets (.qmp files) are a
simple way to share
your mods/configuration with
other QuantumLauncher users"
                )
                .size(13),
                // TODO: Add modrinth/curseforge modpack export
                widget::text(
                    r"In the future, you'll also get
the option to export as
Modrinth/Curseforge modpack"
                )
                .style(tsubtitle)
                .size(12),
                widget::checkbox(
                    "Include mod settings/configuration (config folder)",
                    self.include_config
                )
                .on_toggle(|t| EditPresetsMessage::ToggleIncludeConfig(t).into()),
                button_with_icon(icons::floppydisk(), "Build Preset", 16)
                    .on_press(EditPresetsMessage::BuildYourOwn.into()),
            ]
            .padding(10)
            .spacing(10),
            widget::container(
                column![
                    column![
                        widget::button(if let SelectedState::All = self.selected_state {
                            "Unselect All"
                        } else {
                            "Select All"
                        })
                        .on_press(EditPresetsMessage::SelectAll.into())
                    ]
                    .padding({
                        let p: iced::Padding = 10.into();
                        p.bottom(0)
                    }),
                    widget::scrollable(self.get_mods_list(&self.selected_mods).padding(10))
                        .style(|t: &LauncherTheme, s| t.style_scrollable_flat_extra_dark(s))
                        .width(Length::Fill),
                ]
                .spacing(10)
            )
            .style(|t: &LauncherTheme| t.style_container_sharp_box(0.0, Color::ExtraDark))
        ];

        if self.drag_and_drop_hovered {
            widget::stack!(
                p_main,
                widget::center(widget::button(
                    widget::text("Drag and drop mod files to add them").size(20)
                ))
            )
            .into()
        } else {
            p_main.into()
        }
    }

    fn get_mods_list<'a>(
        &'a self,
        selected_mods: &'a HashSet<SelectedMod>,
    ) -> widget::Column<'a, Message, LauncherTheme, iced::Renderer> {
        widget::column(self.sorted_mods_list.iter().map(|entry| {
            if entry.is_manually_installed() {
                widget::checkbox(entry.name(), selected_mods.contains(&entry.clone().into()))
                    .on_toggle(move |t| match entry {
                        ModListEntry::Downloaded { id, config } => {
                            EditPresetsMessage::ToggleCheckbox((config.name.clone(), id.clone()), t)
                                .into()
                        }
                        ModListEntry::Local { file_name } => {
                            EditPresetsMessage::ToggleCheckboxLocal(file_name.clone(), t).into()
                        }
                    })
                    .into()
            } else {
                widget::text!(" - (DEPENDENCY) {}", entry.name())
                    .shaping(widget::text::Shaping::Advanced)
                    .into()
            }
        }))
        .spacing(5)
    }
}

impl MenuRecommendedMods {
    pub fn view(&'_ self) -> Element<'_> {
        let back_button = back_button().on_press(ManageModsMessage::Open.into());

        match self {
            MenuRecommendedMods::Loading { progress, .. } => progress.view().padding(10).into(),
            MenuRecommendedMods::InstallALoader => {
                column![
                    back_button,
                    "Install a mod loader (like Fabric/Forge/NeoForge/Quilt/etc, whichever is compatible)",
                    "You need one before you can install mods"
                ].padding(10).spacing(5).into()
            }
            MenuRecommendedMods::NotSupported => {
                column![
                    back_button,
                    "No recommended mods found :)"
                ].padding(10).spacing(5).into()
            }
            MenuRecommendedMods::Loaded { mods, .. } => {

                let content = column![
                    row![
                        back_button,
                        button_with_icon(icons::download(), "Download Recommended Mods", 16)
                            .on_press(RecommendedModMessage::Download.into()),
                    ].spacing(10),
                    mods_list(mods),
                    widget::text("Credit to Void98 (https://github.com/void90user) for many of these :D").size(12).style(tsubtitle)
                ].padding(10).spacing(10);

                widget::scrollable(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(|t: &LauncherTheme, status| t.style_scrollable_flat_dark(status))
                    .into()
            }
        }
    }
}

fn mods_list(
    mods: &[(bool, ql_mod_manager::store::RecommendedMod)],
) -> widget::Column<'_, Message, LauncherTheme> {
    widget::column(mods.chunks(2).enumerate().map(|(i, chunks)| {
        widget::row(chunks.iter().enumerate().map(|(j, (enabled, m))| {
            let idx = (i * 2) + j;
            widget::mouse_area(
                row![
                    widget::checkbox("", *enabled)
                        .spacing(0)
                        .on_toggle(move |t| RecommendedModMessage::Toggle(idx, t).into()),
                    column![
                        widget::text(m.name).size(14),
                        widget::text(m.description)
                            .shaping(widget::text::Shaping::Advanced)
                            .style(tsubtitle)
                            .size(12)
                    ]
                    .spacing(2)
                ]
                .width(Length::FillPortion(1))
                .spacing(5),
            )
            .on_press(RecommendedModMessage::Toggle(idx, !*enabled).into())
            .into()
        }))
        .into()
    }))
    .spacing(10)
}
