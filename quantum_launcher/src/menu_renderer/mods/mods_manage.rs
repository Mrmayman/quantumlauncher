use crate::menu_renderer::{select_box, subbutton_with_icon, FONT_MONO};
use crate::message_handler::ForgeKind;
use crate::state::{ImageState, InstallPaperMessage};
use crate::stylesheet::styles::{BORDER_RADIUS, BORDER_WIDTH};
use crate::{
    icon_manager,
    menu_renderer::{back_button, back_to_launch_screen, button_with_icon, tooltip, Element},
    state::{
        EditPresetsMessage, InstallFabricMessage, InstallModsMessage, InstallOptifineMessage,
        ManageJarModsMessage, ManageModsMessage, MenuEditMods, Message, ModListEntry,
        SelectedState,
    },
    stylesheet::{color::Color, styles::LauncherTheme},
};
use iced::widget::tooltip::Position;
use iced::{widget, Alignment, Length};
use ql_core::json::InstanceConfigJson;
use ql_core::{InstanceSelection, SelectedMod};

pub const MODS_SIDEBAR_WIDTH: u16 = 190;

impl MenuEditMods {
    pub fn view<'a>(
        &'a self,
        selected_instance: &'a InstanceSelection,
        tick_timer: usize,
        images: &'a ImageState,
    ) -> Element<'a> {
        if let Some(progress) = &self.mod_update_progress {
            return widget::column!(widget::text("Updating mods").size(20), progress.view())
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
        } else if self.submenu1_shown {
            let submenu = widget::column![
                ctx_button("Export list as text")
                    .on_press(Message::ManageMods(ManageModsMessage::ExportMenuOpen)),
                ctx_button("Export QMP Preset")
                    .on_press(Message::EditPresets(EditPresetsMessage::Open)),
                widget::horizontal_rule(1)
                    .style(|t: &LauncherTheme| t.style_rule(Color::SecondDark, 1)),
                ctx_button("See recommended mods").on_press(Message::RecommendedMods(
                    crate::state::RecommendedModMessage::Open
                )),
            ]
            .spacing(4);

            widget::stack!(
                menu_main,
                widget::row![
                    widget::Space::with_width(MODS_SIDEBAR_WIDTH + 30),
                    widget::column![
                        widget::Space::with_height(40),
                        widget::container(submenu).padding(10).width(200).style(
                            |t: &LauncherTheme| t.style_container_round_box(
                                BORDER_WIDTH,
                                Color::Dark,
                                BORDER_RADIUS
                            )
                        )
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
            widget::column!(
                back_button().on_press(back_to_launch_screen(selected_instance, None)),
                self.get_mod_installer_buttons(selected_instance),
                widget::column!(
                    button_with_icon(icon_manager::download_with_size(14), "Download Content", 15)
                        .on_press(Message::InstallMods(InstallModsMessage::Open)),
                    button_with_icon(icon_manager::jar_file(), "Jarmod Patches", 15)
                        .on_press(Message::ManageJarMods(ManageJarModsMessage::Open))
                )
                .spacing(5),
                Self::open_mod_folder_button(selected_instance),
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
            let dots = ".".repeat((tick_timer % 3) + 1);
            widget::text!("Checking for mod updates{dots}")
                .size(13)
                .into()
        } else if self.available_updates.is_empty() {
            widget::column!().into()
        } else {
            widget::container(
                widget::column!(
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

                            widget::checkbox(text, *is_enabled)
                                .on_toggle(move |b| {
                                    Message::ManageMods(ManageModsMessage::UpdateCheckToggle(i, b))
                                })
                                .text_size(12)
                                .into()
                        }
                    ))
                    .spacing(10),
                    button_with_icon(icon_manager::update(), "Update", 16)
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
        match self.config.mod_type.as_str() {
            "Vanilla" => match selected_instance {
                InstanceSelection::Instance(_) => widget::column![
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
                InstanceSelection::Server(_) => widget::column!(
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

            "Forge" => widget::Column::new()
                .push_maybe(
                    (!selected_instance.is_server())
                        .then(|| Self::get_optifine_install_button(&self.config)),
                )
                .push(Self::get_uninstall_panel(&self.config.mod_type))
                .spacing(5)
                .into(),
            "OptiFine" => widget::column!(
                widget::button(widget::text("Install Forge with OptiFine").size(14))
                    .on_press(Message::InstallForge(ForgeKind::OptiFine)),
                Self::get_uninstall_panel(&self.config.mod_type,),
            )
            .spacing(5)
            .into(),

            "NeoForge" | "Fabric" | "Quilt" | "Paper" => {
                Self::get_uninstall_panel(&self.config.mod_type).into()
            }

            _ => {
                widget::column!(widget::text!("Unknown mod type: {}", self.config.mod_type)).into()
            }
        }
    }

    fn get_optifine_install_button(
        config: &InstanceConfigJson,
    ) -> widget::Button<'_, Message, LauncherTheme> {
        if let Some(optifine) = config
            .mod_type_info
            .as_ref()
            .and_then(|n| n.optifine_jar.as_deref())
        {
            widget::button(
                widget::row![
                    icon_manager::delete_with_size(14),
                    widget::text("Uninstall OptiFine").size(14)
                ]
                .align_y(iced::alignment::Vertical::Center)
                .spacing(11)
                .padding(2),
            )
            .on_press_with(|| {
                Message::UninstallLoaderConfirm(
                    Box::new(Message::ManageMods(ManageModsMessage::DeleteOptiforge(
                        optifine.to_owned(),
                    ))),
                    "OptiFine".to_owned(),
                )
            })
        } else {
            widget::button(widget::text("Install OptiFine with Forge").size(14))
                .on_press(Message::InstallOptifine(InstallOptifineMessage::ScreenOpen))
        }
    }

    fn get_uninstall_panel(mod_type: &'_ str) -> widget::Button<'_, Message, LauncherTheme> {
        widget::button(
            widget::row![
                icon_manager::delete_with_size(14),
                widget::text!("Uninstall {mod_type}").size(14)
            ]
            .align_y(iced::alignment::Vertical::Center)
            .spacing(11)
            .padding(2),
        )
        .on_press(Message::UninstallLoaderConfirm(
            Box::new(Message::UninstallLoaderStart),
            mod_type.to_owned(),
        ))
    }

    fn open_mod_folder_button(selected_instance: &'_ InstanceSelection) -> Element<'_> {
        let path = {
            let path = selected_instance.get_dot_minecraft_path().join("mods");
            path.exists().then_some(path)
        };

        button_with_icon(icon_manager::folder_with_size(14), "Open Mods Folder", 15)
            .on_press_maybe(path.map(Message::CoreOpenPath))
            .into()
    }

    fn get_mod_list<'a>(&'a self, images: &'a ImageState) -> Element<'a> {
        if self.sorted_mods_list.is_empty() {
            return widget::column!(
                "Download some mods to get started",
                widget::button("View Recommended Mods").on_press(Message::RecommendedMods(
                    crate::state::RecommendedModMessage::Open
                ))
            )
            .spacing(10)
            .padding(10)
            .width(Length::Fill)
            .into();
        }

        widget::container(
            widget::column!(
                widget::Column::new()
                    .push_maybe(
                        (self.config.mod_type == "Vanilla" && !self.sorted_mods_list.is_empty())
                        .then_some(
                            widget::container(
                                widget::text(
                                    // WARN: No loader installed
                                    "You haven't installed any mod loader! Install Fabric/Forge/Quilt/NeoForge as per your mods"
                                ).size(12)
                            ).padding(10).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                        )
                    )
                    .push(
                        widget::row![
                            widget::button(
                                widget::row![icon_manager::three_lines_with_size(12)]
                                    .align_y(iced::alignment::Vertical::Center)
                                    .padding(1),
                            )
                            .style(|t: &LauncherTheme, s| {
                                t.style_button(s, crate::stylesheet::widgets::StyleButton::RoundDark)
                            })
                            .on_press(Message::ManageMods(ManageModsMessage::ToggleSubmenu1)),
                            tooltip(
                                widget::button(
                                    widget::row![icon_manager::blank_file_with_size(12)]
                                        .align_y(iced::alignment::Vertical::Center)
                                        .padding(1),
                                )
                                .style(|t: &LauncherTheme, s| {
                                    t.style_button(s, crate::stylesheet::widgets::StyleButton::RoundDark)
                                }).on_press(Message::ManageMods(ManageModsMessage::AddFile(false))),
                                widget::text("Import mod or modpack").size(12),
                                Position::Bottom
                            ),
                            subbutton_with_icon(icon_manager::delete_with_size(12), "Delete")
                            .on_press_maybe((!self.selected_mods.is_empty()).then_some(Message::ManageMods(ManageModsMessage::DeleteSelected))),
                            subbutton_with_icon(icon_manager::toggle_off_with_size(12), "Toggle")
                            .on_press_maybe((!self.selected_mods.is_empty()).then_some(Message::ManageMods(ManageModsMessage::ToggleSelected))),
                            subbutton_with_icon(icon_manager::tick_with_size(12), if matches!(self.selected_state, SelectedState::All) {
                                "Unselect All"
                            } else {
                                "Select All"
                            })
                            .on_press(Message::ManageMods(ManageModsMessage::SelectAll)),
                        ]
                        .spacing(5)
                        .wrap()
                    )
                    .push(if self.selected_mods.is_empty() {
                        widget::text("Select some mods to perform actions on them")
                    } else {
                        widget::text!("{} mods selected", self.selected_mods.len())
                    }.size(12).style(|t: &LauncherTheme| t.style_text(Color::Mid)))
                    .padding(10)
                    .spacing(10),
                widget::responsive(|s| self.get_mod_list_contents(s, images)),
            )
            .spacing(0),
        )
        .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))
        .into()
    }

    fn get_mod_list_contents<'a>(
        &'a self,
        size: iced::Size,
        images: &'a ImageState,
    ) -> Element<'a> {
        widget::scrollable(widget::column({
            self.sorted_mods_list
                .iter()
                .map(|mod_list_entry| self.get_mod_entry(mod_list_entry, size, images))
        }))
        .direction(widget::scrollable::Direction::Both {
            vertical: widget::scrollable::Scrollbar::new(),
            horizontal: widget::scrollable::Scrollbar::new(),
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
        const PADDING: iced::Padding = iced::Padding {
            top: 2.0,
            bottom: 4.0,
            right: 15.0,
            left: 20.0,
        };
        const ICON_SIZE: u16 = 18;
        const SPACING: u16 = 25;

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
                        images.view(url, Some(ICON_SIZE), no_icon)
                    } else {
                        no_icon
                    };

                    let checkbox = select_box(
                        widget::row![
                            image,
                            widget::text(&config.name)
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

                            let measured: f32 = (config.installed_version.len() as f32) * 7.2;
                            let occupied =
                                measured + self.width_name + PADDING.left + PADDING.right + 20.0;
                            let space = size.width - occupied;
                            (space > -10.0).then_some(widget::Space::with_width(space))
                        })
                        .align_y(Alignment::Center)
                        .padding(PADDING)
                        .spacing(SPACING),
                        is_selected,
                        Message::ManageMods(ManageModsMessage::ToggleCheckbox(
                            config.name.clone(),
                            Some(id.clone()),
                        )),
                    )
                    .padding(0);

                    if is_enabled {
                        checkbox.into()
                    } else {
                        tooltip(checkbox, "Disabled", Position::FollowCursor).into()
                    }
                } else {
                    widget::row![
                        widget::text("(dependency) ")
                            .size(12)
                            .style(|t: &LauncherTheme| t.style_text(Color::Mid)),
                        widget::text(&config.name)
                            .size(13)
                            .style(|t: &LauncherTheme| t.style_text(Color::SecondLight))
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
                    Message::ManageMods(ManageModsMessage::ToggleCheckbox(file_name.clone(), None)),
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
}

fn install_ldr(loader: &str) -> widget::Button<'_, Message, LauncherTheme> {
    widget::button(loader).width(97)
}

fn ctx_button(e: &'_ str) -> widget::Button<'_, Message, LauncherTheme> {
    widget::button(widget::text(e).size(13))
        .width(Length::Fill)
        .style(|t: &LauncherTheme, s| {
            t.style_button(s, crate::stylesheet::widgets::StyleButton::FlatDark)
        })
        .padding(2)
}
