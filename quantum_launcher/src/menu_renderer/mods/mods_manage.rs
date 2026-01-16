use iced::widget::tooltip::Position;
use iced::{
    widget::{self, column},
    Alignment, Length,
};
use ql_core::{InstanceSelection, Loader, SelectedMod};

use crate::menu_renderer::ui::checkbox;
use crate::menu_renderer::{ctxbox, dots, select_box, subbutton_with_icon, tsubtitle, FONT_MONO};
use crate::message_handler::ForgeKind;
use crate::state::{ImageState, InstallPaperMessage, MenuEditModsModal};
use crate::stylesheet::widgets::StyleButton;
use crate::{
    icons,
    menu_renderer::{back_button, back_to_launch_screen, button_with_icon, tooltip, Element},
    state::{
        EditPresetsMessage, InstallFabricMessage, InstallModsMessage, InstallOptifineMessage,
        ManageJarModsMessage, ManageModsMessage, MenuEditMods, Message, ModListEntry,
        SelectedState,
    },
    stylesheet::{color::Color, styles::LauncherTheme},
};
use ql_core::json::InstanceConfigJson;

pub const MODS_SIDEBAR_WIDTH: u32 = 190;
const PADDING: iced::Padding = iced::Padding {
    top: 4.0,
    bottom: 6.0,
    right: 15.0,
    left: 20.0,
};

impl MenuEditMods {
    pub fn view<'a>(
        &'a self,
        selected_instance: &'a InstanceSelection,
        tick_timer: usize,
        images: &'a ImageState,
        window_height: f32,
    ) -> Element<'a> {
        if let Some(progress) = &self.mod_update_progress {
            return column!(widget::text("Updating mods").size(20), progress.view())
                .padding(10)
                .spacing(10)
                .into();
        }

        let menu_main = widget::row!(
            self.get_sidebar(selected_instance, tick_timer),
            self.get_mod_list(images)
        );

        if self.drag_and_drop_hovered {
            widget::stack!(
                menu_main,
                widget::center(widget::button(
                    widget::text("Drag and drop mod files to add them").size(20)
                ))
            )
            .into()
        } else if let Some(MenuEditModsModal::Submenu) = &self.modal {
            let submenu = column![
                ctx_button("Export list as text")
                    .on_press(Message::ManageMods(ManageModsMessage::ExportMenuOpen)),
                ctx_button("Export QMP Preset")
                    .on_press(Message::EditPresets(EditPresetsMessage::Open)),
                widget::rule::horizontal(1)
                    .style(|t: &LauncherTheme| t.style_rule(Color::SecondDark)),
                ctx_button("See recommended mods").on_press(Message::RecommendedMods(
                    crate::state::RecommendedModMessage::Open
                )),
            ]
            .spacing(4);

            widget::stack!(
                menu_main,
                widget::row![
                    widget::space().width(MODS_SIDEBAR_WIDTH + 30),
                    column![widget::space().height(40), ctxbox(submenu).width(200)]
                ]
            )
            .into()
        } else if let Some(MenuEditModsModal::RightClick(id, (x, y))) = &self.modal {
            widget::stack!(
                menu_main,
                column![
                    widget::space().height(y.clamp(0.0, window_height - 130.0)),
                    widget::row![
                        widget::space().width(*x),
                        ctxbox(
                            column![
                                ctx_button("Toggle").on_press(Message::ManageMods(
                                    ManageModsMessage::ToggleSelected
                                )),
                                ctx_button("Delete").on_press(Message::ManageMods(
                                    ManageModsMessage::DeleteSelected
                                )),
                                ctx_button("Mod Details").on_press_maybe(
                                    self.mods.mods.get(&id.get_index_str()).map(|info| {
                                        Message::Multiple(vec![
                                            Message::InstallMods(InstallModsMessage::Open),
                                            Message::InstallMods(
                                                InstallModsMessage::ChangeBackend(id.get_backend()),
                                            ),
                                            Message::InstallMods(InstallModsMessage::SearchInput(
                                                info.name.clone(),
                                            )),
                                        ])
                                    })
                                ),
                            ]
                            .spacing(4)
                        )
                        .width(200)
                    ]
                ]
            )
            .into()
        } else {
            menu_main.into()
        }
    }

    fn get_sidebar<'a>(
        &'a self,
        selected_instance: &'a InstanceSelection,
        tick_timer: usize,
    ) -> widget::Scrollable<'a, Message, LauncherTheme> {
        widget::scrollable(
            column!(
                widget::row![
                    back_button().on_press(back_to_launch_screen(
                        Some(selected_instance.is_server()),
                        None
                    )),
                    tooltip(
                        button_with_icon(icons::folder_s(14), "Open", 14).on_press_with(|| {
                            Message::CoreOpenPath(
                                selected_instance.get_dot_minecraft_path().join("mods"),
                            )
                        }),
                        widget::text("Open Mods Folder").size(12),
                        Position::Bottom
                    )
                ]
                .spacing(5),
                self.get_mod_installer_buttons(selected_instance),
                column!(
                    button_with_icon(icons::download_s(15), "Download Content...", 14)
                        .on_press(Message::InstallMods(InstallModsMessage::Open)),
                    button_with_icon(icons::file_jar(), "Jarmod Patches", 14)
                        .on_press(Message::ManageJarMods(ManageJarModsMessage::Open)),
                    tooltip(
                        button_with_icon(icons::file(), "Add File", 14)
                            .on_press(Message::ManageMods(ManageModsMessage::AddFile(false))),
                        widget::text("Includes mods and modpacks").size(12),
                        Position::Bottom
                    ),
                )
                .spacing(5),
                self.get_mod_update_pane(tick_timer),
            )
            .padding(10)
            .spacing(10),
        )
        .style(LauncherTheme::style_scrollable_flat_dark)
        .height(Length::Fill)
    }

    fn get_mod_update_pane(&'_ self, tick_timer: usize) -> Element<'_> {
        if self.update_check_handle.is_some() {
            widget::text!("Checking for mod updates{}", dots(tick_timer))
                .size(12)
                .into()
        } else if self.available_updates.is_empty() {
            column!().into()
        } else {
            widget::container(
                column!(
                    widget::text("Mod Updates Available!").size(15),
                    widget::column(self.available_updates.iter().enumerate().map(
                        |(i, (id, update_name, is_enabled))| {
                            let title = self
                                .mods
                                .mods
                                .get(&id.get_index_str())
                                .map(|n| n.name.clone())
                                .unwrap_or_default();

                            let text = if title.is_empty()
                                || update_name.contains(&title)
                                || update_name.contains(&title.replace(' ', ""))
                            {
                                update_name.clone()
                            } else {
                                format!("{title} - {update_name}")
                            };

                            checkbox(widget::text(text).size(12), *is_enabled, move |b| {
                                Message::ManageMods(ManageModsMessage::UpdateCheckToggle(i, b))
                            })
                            .into()
                        }
                    ))
                    .spacing(10),
                    button_with_icon(icons::version_download(), "Update", 16)
                        .on_press(Message::ManageMods(ManageModsMessage::UpdateMods)),
                )
                .padding(10)
                .spacing(10)
                .width(MODS_SIDEBAR_WIDTH),
            )
            .into()
        }
    }

    fn get_mod_installer_buttons(&'_ self, selected_instance: &InstanceSelection) -> Element<'_> {
        match self.config.mod_type {
            Loader::Vanilla => match selected_instance {
                InstanceSelection::Instance(_) => column![
                    "Install:",
                    widget::row!(
                        install_ldr("Fabric").on_press(Message::InstallFabric(
                            InstallFabricMessage::ScreenOpen { is_quilt: false }
                        )),
                        install_ldr("Quilt").on_press(Message::InstallFabric(
                            InstallFabricMessage::ScreenOpen { is_quilt: true }
                        )),
                    )
                    .spacing(5),
                    widget::row!(
                        install_ldr("Forge").on_press(Message::InstallForge(ForgeKind::Normal)),
                        install_ldr("NeoForge")
                            .on_press(Message::InstallForge(ForgeKind::NeoForge))
                    )
                    .spacing(5),
                    install_ldr("OptiFine")
                        .on_press(Message::InstallOptifine(InstallOptifineMessage::ScreenOpen))
                ]
                .spacing(5)
                .into(),
                InstanceSelection::Server(_) => column!(
                    "Install:",
                    widget::row!(
                        install_ldr("Fabric").on_press(Message::InstallFabric(
                            InstallFabricMessage::ScreenOpen { is_quilt: false }
                        )),
                        install_ldr("Quilt").on_press(Message::InstallFabric(
                            InstallFabricMessage::ScreenOpen { is_quilt: true }
                        )),
                    )
                    .spacing(5),
                    widget::row!(
                        install_ldr("Forge").on_press(Message::InstallForge(ForgeKind::Normal)),
                        install_ldr("NeoForge")
                            .on_press(Message::InstallForge(ForgeKind::NeoForge))
                    )
                    .spacing(5),
                    widget::row!(
                        widget::button("Bukkit").width(97),
                        widget::button("Spigot").width(97)
                    )
                    .spacing(5),
                    install_ldr("Paper")
                        .on_press(Message::InstallPaper(InstallPaperMessage::ScreenOpen)),
                )
                .spacing(5)
                .into(),
            },

            Loader::Forge => column![
                (!selected_instance.is_server())
                    .then(|| Self::get_optifine_install_button(&self.config)),
                Self::get_uninstall_panel(self.config.mod_type)
            ]
            .spacing(5)
            .into(),
            Loader::OptiFine => column!(
                widget::button(widget::text("Install Forge with OptiFine").size(14))
                    .on_press(Message::InstallForge(ForgeKind::OptiFine)),
                Self::get_uninstall_panel(self.config.mod_type),
            )
            .spacing(5)
            .into(),

            Loader::Neoforge | Loader::Fabric | Loader::Quilt | Loader::Paper => {
                Self::get_uninstall_panel(self.config.mod_type).into()
            }

            _ => column!(widget::text!("Unknown mod type: {}", self.config.mod_type)).into(),
        }
    }

    fn get_optifine_install_button(
        config: &InstanceConfigJson,
    ) -> widget::Button<'static, Message, LauncherTheme> {
        if let Some(optifine) = config
            .mod_type_info
            .as_ref()
            .and_then(|n| n.optifine_jar.as_deref())
        {
            widget::button(
                widget::row![
                    icons::bin_s(14),
                    widget::text("Uninstall OptiFine").size(14)
                ]
                .align_y(Alignment::Center)
                .spacing(11)
                .padding(2),
            )
            .on_press(Message::UninstallLoaderConfirm(
                Box::new(Message::ManageMods(ManageModsMessage::DeleteOptiforge(
                    optifine.to_owned(),
                ))),
                Loader::OptiFine,
            ))
        } else {
            widget::button(widget::text("Install OptiFine with Forge").size(14))
                .on_press(Message::InstallOptifine(InstallOptifineMessage::ScreenOpen))
        }
    }

    fn get_uninstall_panel(mod_type: Loader) -> widget::Button<'static, Message, LauncherTheme> {
        widget::button(
            widget::row![
                icons::bin_s(14),
                widget::text!("Uninstall {mod_type}").size(14)
            ]
            .align_y(Alignment::Center)
            .spacing(11)
            .padding(2),
        )
        .on_press(Message::UninstallLoaderConfirm(
            Box::new(Message::UninstallLoaderStart),
            mod_type,
        ))
    }

    fn get_mod_list<'a>(&'a self, images: &'a ImageState) -> Element<'a> {
        if self.sorted_mods_list.is_empty() {
            return column!(
                "Download some mods to get started",
                widget::button(widget::text("View Recommended Mods").size(14)).on_press(
                    Message::RecommendedMods(crate::state::RecommendedModMessage::Open)
                )
            )
            .spacing(10)
            .padding(10)
            .width(Length::Fill)
            .into();
        }

        widget::container(column![
            (self.config.mod_type.is_vanilla() && !self.sorted_mods_list.is_empty())
            .then_some(
                widget::container(
                    widget::text(
                        // WARN: No loader installed
                        "You haven't installed any mod loader! Install Fabric/Forge/Quilt/NeoForge as per your mods"
                    ).size(12)
                ).padding(10).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
            ),
            column![
                widget::row![
                    self.get_hamburger_dropdown(),
                    self.get_search_button(),

                    subbutton_with_icon(icons::bin_s(12), "Delete")
                    .on_press_maybe((!self.selected_mods.is_empty()).then_some(Message::ManageMods(ManageModsMessage::DeleteSelected))),
                    subbutton_with_icon(icons::toggleoff_s(12), "Toggle")
                    .on_press_maybe((!self.selected_mods.is_empty()).then_some(Message::ManageMods(ManageModsMessage::ToggleSelected))),
                    subbutton_with_icon(icons::deselectall_s(12), if matches!(self.selected_state, SelectedState::All) {
                        "Unselect All"
                    } else {
                        "Select All"
                    })
                    .on_press(Message::ManageMods(ManageModsMessage::SelectAll)),
                ]
                .spacing(10)
                .wrap(),
                if self.selected_mods.is_empty() {
                    widget::text("Select some mods to perform actions on them")
                } else {
                    widget::text!("{} mods selected", self.selected_mods.len())
                }.size(12).style(|t: &LauncherTheme| t.style_text(Color::Mid)),
                self.search.as_ref().map(|search|
                    widget::text_input("Search...", search).size(14).on_input(|msg|
                        Message::ManageMods(ManageModsMessage::SetSearch(Some(msg)))
                    )
                ),
            ].padding(10),
            widget::responsive(|s| self.get_mod_list_contents(s, images)),
        ])
        .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))
        .into()
    }

    fn get_hamburger_dropdown(&self) -> widget::Button<'_, Message, LauncherTheme> {
        widget::button(
            widget::row![icons::lines_s(12)]
                .align_y(Alignment::Center)
                .padding(1),
        )
        .style(|t: &LauncherTheme, s| {
            t.style_button(s, crate::stylesheet::widgets::StyleButton::RoundDark)
        })
        .on_press(Message::ManageMods(ManageModsMessage::SetModal(
            self.modal.is_none().then_some(MenuEditModsModal::Submenu),
        )))
    }

    fn get_search_button(&self) -> widget::Button<'_, Message, LauncherTheme> {
        widget::button(
            widget::row![icons::search_s(12)]
                .align_y(Alignment::Center)
                .padding(1),
        )
        .style(|t: &LauncherTheme, s| {
            t.style_button(
                s,
                if self.search.is_some() {
                    StyleButton::Round
                } else {
                    StyleButton::RoundDark
                },
            )
        })
        .on_press(if self.search.is_some() {
            Message::ManageMods(ManageModsMessage::SetSearch(None))
        } else {
            Message::Multiple(vec![
                Message::ManageMods(ManageModsMessage::SetSearch(Some(String::new()))),
                Message::CoreFocusNext,
            ])
        })
    }

    fn get_mod_list_contents<'a>(
        &'a self,
        size: iced::Size,
        images: &'a ImageState,
    ) -> Element<'a> {
        widget::scrollable(widget::column(
            self.sorted_mods_list
                .iter()
                .filter(|n| {
                    let Some(search) = &self.search else {
                        return true;
                    };
                    n.name().to_lowercase().contains(&search.to_lowercase())
                })
                .map(|mod_list_entry| self.get_mod_entry(mod_list_entry, size, images)),
        ))
        .direction(widget::scrollable::Direction::Both {
            vertical: widget::scrollable::Scrollbar::new(),
            horizontal: widget::scrollable::Scrollbar::new(),
        })
        .id(widget::Id::new("MenuEditMods:mods"))
        .on_scroll(|viewport| {
            Message::ManageMods(ManageModsMessage::ListScrolled(viewport.absolute_offset()))
        })
        .style(LauncherTheme::style_scrollable_flat_extra_dark)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn get_mod_entry<'a>(
        &'a self,
        entry: &'a ModListEntry,
        size: iced::Size,
        images: &'a ImageState,
    ) -> Element<'a> {
        const ICON_SIZE: f32 = 18.0;
        const SPACING: u32 = 25;

        let no_icon = widget::Column::new()
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .into();

        match entry {
            ModListEntry::Downloaded { id, config } => {
                if config.manually_installed {
                    let is_enabled = config.enabled;
                    let is_selected = self.selected_mods.contains(&SelectedMod::Downloaded {
                        name: config.name.clone(),
                        id: (*id).clone(),
                    });

                    let image: Element = if let Some(url) = &config.icon_url {
                        // SVGs cause absurd lag in large lists of mods
                        images.view_bitmap(url, Some(ICON_SIZE), Some(ICON_SIZE), no_icon)
                    } else {
                        no_icon
                    };

                    let checkbox = select_box(
                        widget::row![
                            image,
                            widget::text(&config.name)
                                .shaping(widget::text::Shaping::Advanced)
                                .style(move |t: &LauncherTheme| {
                                    t.style_text(if is_enabled {
                                        Color::SecondLight
                                    } else {
                                        Color::Mid
                                    })
                                })
                                .size(14)
                                .width(self.width_name),
                            widget::text(&config.installed_version)
                                .style(move |t: &LauncherTheme| t.style_text(if is_enabled {
                                    Color::Mid
                                } else {
                                    Color::SecondDark
                                }))
                                .font(FONT_MONO)
                                .size(12),
                            self.measure_version_text_len(size, config)
                        ]
                        .align_y(Alignment::Center)
                        .padding(PADDING)
                        .spacing(SPACING),
                        is_selected,
                        Message::ManageMods(ManageModsMessage::SelectMod(
                            config.name.clone(),
                            Some(id.clone()),
                        )),
                    )
                    .padding(0);

                    let checkbox: Element = if is_enabled {
                        checkbox.into()
                    } else {
                        tooltip(checkbox, "Disabled", Position::FollowCursor).into()
                    };

                    let rightclick = Message::ManageMods(ManageModsMessage::RightClick(id.clone()));

                    widget::mouse_area(checkbox)
                        .on_right_press(if self.selected_mods.len() > 1 && self.is_selected(id) {
                            rightclick
                        } else {
                            Message::Multiple(vec![
                                Message::ManageMods(ManageModsMessage::SelectEnsure(
                                    config.name.clone(),
                                    Some(id.clone()),
                                )),
                                rightclick,
                            ])
                        })
                        .into()
                } else {
                    widget::row![
                        widget::text("(dependency) ")
                            .size(12)
                            .style(|t: &LauncherTheme| t.style_text(Color::Mid)),
                        widget::text(&config.name)
                            .shaping(widget::text::Shaping::Advanced)
                            .size(13)
                            .style(tsubtitle)
                    ]
                    .padding(PADDING)
                    .into()
                }
            }
            ModListEntry::Local { file_name } => {
                let is_enabled = !file_name.ends_with(".disabled");
                let is_selected = self.selected_mods.contains(&SelectedMod::Local {
                    file_name: file_name.clone(),
                });

                let checkbox = select_box(
                    widget::row![
                        no_icon,
                        widget::text(
                            file_name
                                .strip_suffix(".disabled")
                                .unwrap_or(file_name)
                                .to_owned(),
                        )
                        .font(FONT_MONO)
                        .shaping(widget::text::Shaping::Advanced)
                        .style(move |t: &LauncherTheme| {
                            t.style_text(if is_enabled {
                                Color::SecondLight
                            } else {
                                Color::Mid
                            })
                        })
                        .size(14)
                    ]
                    .spacing(SPACING),
                    is_selected,
                    Message::ManageMods(ManageModsMessage::SelectMod(file_name.clone(), None)),
                )
                .padding(PADDING)
                .width(size.width);

                if is_enabled {
                    checkbox.into()
                } else {
                    tooltip(checkbox, "Disabled", Position::FollowCursor).into()
                }
            }
        }
    }

    /// Measure the length of the text
    /// then from there measure the space it would occupy
    /// (only possible because monospace font)
    ///
    /// This is for finding the filler space
    ///
    /// ║ Some Mod         v0.0.1                ║
    /// ║ Some other mod   2.4.1-fabric          ║
    ///
    ///  ╙═╦══════════════╜            ╙═╦══════╜
    ///  Measured by:                   What we want
    ///  `self.width_name`              to find
    fn measure_version_text_len(
        &self,
        size: iced::Size,
        config: &ql_mod_manager::store::ModConfig,
    ) -> Option<widget::Space> {
        let measured: f32 = (config.installed_version.len() as f32) * 7.2;
        let occupied = measured + self.width_name + PADDING.left + PADDING.right + 100.0;
        let space = size.width - occupied;
        (space > -10.0).then_some(widget::space().width(space))
    }
}

fn install_ldr(loader: &str) -> widget::Button<'_, Message, LauncherTheme> {
    widget::button(widget::text(loader).size(14)).width(90)
}

fn ctx_button(e: &'_ str) -> widget::Button<'_, Message, LauncherTheme> {
    widget::button(widget::text(e).size(13))
        .width(Length::Fill)
        .style(|t: &LauncherTheme, s| {
            t.style_button(s, crate::stylesheet::widgets::StyleButton::FlatDark)
        })
        .padding(2)
}
