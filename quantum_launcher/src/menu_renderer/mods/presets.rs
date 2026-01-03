use std::collections::HashSet;

use iced::{
    widget::{self, column},
    Length,
};
use ql_core::SelectedMod;

use crate::{
    icons,
    menu_renderer::{back_button, button_with_icon, tsubtitle, Element},
    state::{
        EditPresetsMessage, ManageModsMessage, MenuEditPresets, MenuRecommendedMods, Message,
        ModListEntry, SelectedState,
    },
    stylesheet::{color::Color, styles::LauncherTheme},
};

impl MenuEditPresets {
    pub fn view(&'_ self) -> Element<'_> {
        if let Some(progress) = &self.progress {
            return column!(
                widget::text("Installing mods").size(20),
                progress.view(),
                widget::text("Check debug log (at the bottom) for more info").size(12),
            )
            .padding(10)
            .spacing(10)
            .into();
        }

        if self.is_building {
            return column!(widget::text("Building Preset").size(20))
                .padding(10)
                .spacing(10)
                .into();
        }

        let p_main = widget::row![
            column![
                back_button().on_press(Message::ManageMods(
                    ManageModsMessage::ScreenOpenWithoutUpdate
                )),
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
                .on_toggle(|t| Message::EditPresets(EditPresetsMessage::ToggleIncludeConfig(t))),
                button_with_icon(icons::floppydisk(), "Build Preset", 16)
                    .on_press(Message::EditPresets(EditPresetsMessage::BuildYourOwn)),
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
                        .on_press(Message::EditPresets(EditPresetsMessage::SelectAll)),
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
                            Message::EditPresets(EditPresetsMessage::ToggleCheckbox(
                                (config.name.clone(), id.clone()),
                                t,
                            ))
                        }
                        ModListEntry::Local { file_name } => Message::EditPresets(
                            EditPresetsMessage::ToggleCheckboxLocal(file_name.clone(), t),
                        ),
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
        let back_button = back_button().on_press(Message::ManageMods(
            ManageModsMessage::ScreenOpenWithoutUpdate,
        ));

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
                let content: Element =
                    column!(
                        back_button,
                        button_with_icon(icons::download(), "Download Recommended Mods", 16)
                            .on_press(Message::RecommendedMods(
                                crate::state::RecommendedModMessage::Download
                            )),
                        widget::column(mods.iter().enumerate().map(|(i, (e, n))| {
                            let elem: Element = widget::checkbox(n.name, *e)
                                .on_toggle(move |n| {
                                    Message::RecommendedMods(crate::state::RecommendedModMessage::Toggle(
                                        i, n,
                                    ))
                                })
                                .into();
                            column!(
                                elem,
                                widget::text(n.description)
                                    .shaping(widget::text::Shaping::Advanced)
                                    .size(12)
                            )
                                .spacing(5)
                                .into()
                        }))
                        .spacing(10)
                    )
                    .spacing(10)
                    .into();

                widget::scrollable(column![content].padding(10))
                    .style(|t: &LauncherTheme, status| t.style_scrollable_flat_dark(status))
                    .into()
            }
        }
    }
}
