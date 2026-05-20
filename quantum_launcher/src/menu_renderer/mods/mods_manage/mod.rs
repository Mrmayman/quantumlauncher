use crate::{
    icons,
    menu_renderer::{
        CTXI_SIZE, Column, Element, back_to_launch_screen, barthin, button_with_icon, ctx_button,
        ctx_button_empty, ctx_button_icon, dots, offsetbox, subbutton_with_icon, tsubtitle,
        view_info_message,
    },
    message_handler::ForgeKind,
    state::{
        EditPresetsMessage, ExportModsTextMessage, ImageState, InstallFabricMessage,
        InstallModsMessage, InstallOptifineMessage, InstallPaperMessage, ManageJarModsMessage,
        ManageModsMessage, MenuEditMods, MenuEditModsModal, Message, ModDescriptionMessage,
        ModListEntry, SelectedState,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};
use iced::{
    Alignment, Length,
    widget::{self, column, row},
};
use ql_core::{Instance, InstanceKind, Loader, json::InstanceConfigJson};
use ql_mod_manager::store::QueryType;

pub const MODS_SIDEBAR_WIDTH: u16 = 190;

mod entry;

impl MenuEditMods {
    pub fn view<'a>(
        &'a self,
        selected_instance: &'a Instance,
        tick_timer: usize,
        images: &'a ImageState,
        window_height: f32,
    ) -> Element<'a> {
        if let Some(progress) = &self.updates.progress {
            return column![widget::text("Updating mods").size(20), progress.view()]
                .padding(10)
                .spacing(10)
                .into();
        }

        let menu_main = widget::Column::new()
            .push_maybe(
                self.ui_state
                    .info_message
                    .as_ref()
                    .map(|n| view_info_message(n, ManageModsMessage::SetInfoMessage(None).into())),
            )
            .push_maybe(self.ui_state.info_message.as_ref().map(|_| {
                widget::horizontal_rule(2)
                    .style(|t: &LauncherTheme| t.style_rule(Color::SecondDark, 1))
            }))
            .push(row![
                self.get_sidebar(selected_instance, tick_timer),
                self.get_mod_list(images)
            ]);

        self.render_overlays(selected_instance, window_height, menu_main)
    }

    fn render_overlays<'a>(
        &'a self,
        selected_instance: &'a Instance,
        window_height: f32,
        menu_main: widget::Column<'a, Message, LauncherTheme>,
    ) -> Element<'a> {
        if self.ui_state.drag_and_drop_hovered {
            return widget::stack!(
                menu_main,
                widget::center(widget::button(
                    widget::text("Drag and drop mod files to add them").size(20)
                ))
            )
            .into();
        }

        match &self.ui_state.modal {
            Some(MenuEditModsModal::Submenu) => {
                let submenu = column![
                    ctx_button_icon(icons::refresh_s(CTXI_SIZE), "Check for updates")
                        .on_press(ManageModsMessage::UpdateCheck.into()),
                    ctx_button_icon(icons::file_info_s(CTXI_SIZE), "Export list as text")
                        .on_press(ExportModsTextMessage::Open.into()),
                    ctx_button_icon(icons::file_zip_s(CTXI_SIZE), "Export QMP Preset")
                        .on_press(EditPresetsMessage::Open.into()),
                    widget::horizontal_rule(1).style(barthin),
                    ctx_button_icon(icons::download_s(CTXI_SIZE), "See recommended mods").on_press(
                        Message::RecommendedMods(crate::state::RecommendedModMessage::Open)
                    ),
                ]
                .spacing(4);

                offsetbox(menu_main, submenu, MODS_SIDEBAR_WIDTH + 30, 40, 200).into()
            }
            Some(MenuEditModsModal::RightClick(id, (x, y))) => offsetbox(
                menu_main,
                column![
                    ctx_button_icon(icons::toggleon_s(CTXI_SIZE), "Toggle")
                        .on_press(ManageModsMessage::ToggleSelected.into()),
                    ctx_button_icon(icons::bin_s(CTXI_SIZE), "Delete")
                        .on_press(ManageModsMessage::DeleteSelected.into()),
                    ctx_button_icon(icons::file_info_s(CTXI_SIZE), "Mod Details")
                        .on_press_with(|| ModDescriptionMessage::Open(id.clone()).into()),
                    ctx_button_empty("Copy ID")
                        .on_press_with(|| Message::CoreCopyText(id.get_internal_id().to_string())),
                ]
                .spacing(2),
                *x,
                y.clamp(0.0, window_height - 130.0),
                150,
            )
            .into(),
            Some(MenuEditModsModal::FolderMenu) => {
                let folder_menu = column![
                    ctx_button("Mods Folder").on_press_with(|| {
                        Message::CoreOpenPath(
                            selected_instance.get_dot_minecraft_path().join("mods"),
                        )
                    }),
                    ctx_button("Resource Packs Folder").on_press_with(|| Message::CoreOpenPath(
                        selected_instance.get_dot_minecraft_path().join(
                            if self.file_data.details.is_legacy_texturepacks() {
                                "texturepacks"
                            } else {
                                "resourcepacks"
                            }
                        )
                    )),
                    ctx_button("Shaders Folder").on_press_with(|| {
                        Message::CoreOpenPath(
                            selected_instance
                                .get_dot_minecraft_path()
                                .join("shaderpacks"),
                        )
                    }),
                    ctx_button("Datapacks Folder").on_press_with(|| {
                        Message::CoreOpenPath(
                            selected_instance.get_dot_minecraft_path().join("datapacks"),
                        )
                    }),
                ]
                .spacing(4);

                offsetbox(menu_main, folder_menu, 30, 40, 200).into()
            }
            Some(MenuEditModsModal::AddFile) => {
                let addfile_msg = |q| Message::ManageMods(ManageModsMessage::AddFile(false, q));

                let menu = column![
                    ctx_button_icon(icons::file_jar_s(CTXI_SIZE), "Mod")
                        .on_press(addfile_msg(QueryType::Mods)),
                    ctx_button_empty("Shader Pack").on_press(addfile_msg(QueryType::Shaders)),
                    ctx_button_empty("Resource Pack")
                        .on_press(addfile_msg(QueryType::ResourcePacks)),
                    // HOOK: Datapack
                    // ctx_button_empty("Datapack").on_press(addfile_msg(QueryType::DataPacks)),
                    ctx_button_empty("Modpack/QMP").on_press(addfile_msg(QueryType::ModPacks)),
                ]
                .spacing(4);

                offsetbox(menu_main, menu, 30, 70, 150).into()
            }
            None => menu_main.into(),
        }
    }

    fn get_sidebar<'a>(
        &'a self,
        selected_instance: &'a Instance,
        tick_timer: usize,
    ) -> widget::Scrollable<'a, Message, LauncherTheme> {
        const TOP_PADDING: iced::Padding = iced::Padding {
            top: 5.0,
            right: 8.0,
            bottom: 5.0,
            left: 8.0,
        };

        let open_folders_btn = button_with_icon(icons::folder_s(14), "Open...", 14)
            .padding(TOP_PADDING)
            .on_press(
                ManageModsMessage::SetModal(
                    if let Some(MenuEditModsModal::FolderMenu) = self.ui_state.modal {
                        None
                    } else {
                        Some(MenuEditModsModal::FolderMenu)
                    },
                )
                .into(),
            );

        // .on_press(ManageModsMessage::AddFile(false).into())
        let add_file_btn = button_with_icon(icons::file_s(14), "Add File...", 14)
            .padding(TOP_PADDING)
            .on_press(
                ManageModsMessage::SetModal(
                    if let Some(MenuEditModsModal::AddFile) = self.ui_state.modal {
                        None
                    } else {
                        Some(MenuEditModsModal::AddFile)
                    },
                )
                .into(),
            );

        widget::scrollable(
            column![
                column![
                    row![
                        button_with_icon(icons::back_s(13), "Back", 14)
                            .padding(TOP_PADDING)
                            .on_press(back_to_launch_screen(None)),
                        open_folders_btn,
                    ]
                    .spacing(5),
                    add_file_btn,
                ]
                .spacing(5),
                self.get_mod_installer_buttons(selected_instance.kind),
                column![
                    button_with_icon(icons::download_s(15), "Download Content...", 14)
                        .on_press(InstallModsMessage::Open.into()),
                    button_with_icon(icons::file_jar(), "Jarmod Patches", 14)
                        .on_press(ManageJarModsMessage::Open.into()),
                ]
                .spacing(5),
                self.get_mod_update_pane(tick_timer),
            ]
            .padding(10)
            .spacing(10),
        )
        .style(LauncherTheme::style_scrollable_flat_dark)
        .height(Length::Fill)
    }

    fn get_mod_update_pane(&'_ self, tick_timer: usize) -> Column<'_> {
        if self.updates.check_handle.is_some() {
            column![widget::text!("Checking for mod updates{}", dots(tick_timer)).size(12)]
        } else if self.updates.available.is_empty() {
            column![]
        } else {
            column![
                widget::horizontal_rule(1),
                widget::text("Mod Updates Available!").size(15),
                widget::column(self.updates.available.iter().enumerate().map(
                    |(i, (id, update_name, is_enabled))| {
                        let title = self
                            .file_data
                            .mod_index
                            .mods
                            .get(id)
                            .map(|n| &*n.name)
                            .unwrap_or_default();

                        let toggle = move |b| ManageModsMessage::UpdateCheckToggle(i, b).into();

                        widget::mouse_area(row![
                            widget::checkbox("", *is_enabled).on_toggle(toggle),
                            column![
                                widget::text(title).size(12),
                                widget::text!("{update_name}").size(10).style(tsubtitle)
                            ]
                        ])
                        .on_press(toggle(!*is_enabled))
                        .into()
                    }
                ))
                .spacing(5),
                button_with_icon(icons::version_download(), "Update", 16)
                    .on_press(ManageModsMessage::UpdatePerform.into()),
            ]
            .padding(5)
            .spacing(10)
            .width(MODS_SIDEBAR_WIDTH)
        }
    }

    fn get_mod_installer_buttons(&'_ self, kind: InstanceKind) -> Element<'_> {
        match self.file_data.config.mod_type {
            Loader::Vanilla => match kind {
                InstanceKind::Client => column![
                    "Install:",
                    row![
                        install_ldr("Fabric")
                            .on_press(InstallFabricMessage::ScreenOpen { is_quilt: false }.into()),
                        install_ldr("Quilt")
                            .on_press(InstallFabricMessage::ScreenOpen { is_quilt: true }.into()),
                    ]
                    .spacing(5),
                    row![
                        install_ldr("Forge").on_press(Message::InstallForge(ForgeKind::Normal)),
                        install_ldr("NeoForge")
                            .on_press(Message::InstallForge(ForgeKind::NeoForge))
                    ]
                    .spacing(5),
                    install_ldr("OptiFine").on_press(InstallOptifineMessage::ScreenOpen.into())
                ]
                .spacing(5)
                .into(),
                InstanceKind::Server => column![
                    "Install:",
                    row![
                        install_ldr("Fabric")
                            .on_press(InstallFabricMessage::ScreenOpen { is_quilt: false }.into()),
                        install_ldr("Quilt")
                            .on_press(InstallFabricMessage::ScreenOpen { is_quilt: true }.into()),
                    ]
                    .spacing(5),
                    row![
                        install_ldr("Forge").on_press(Message::InstallForge(ForgeKind::Normal)),
                        install_ldr("NeoForge")
                            .on_press(Message::InstallForge(ForgeKind::NeoForge))
                    ]
                    .spacing(5),
                    row![
                        widget::button("Bukkit").width(97),
                        widget::button("Spigot").width(97)
                    ]
                    .spacing(5),
                    install_ldr("Paper")
                        .on_press(Message::InstallPaper(InstallPaperMessage::ScreenOpen)),
                ]
                .spacing(5)
                .into(),
            },

            Loader::Forge => widget::Column::new()
                .push_maybe(
                    matches!(kind, InstanceKind::Client)
                        .then(|| Self::get_optifine_install_button(&self.file_data.config)),
                )
                .push(Self::get_uninstall_panel(self.file_data.config.mod_type))
                .spacing(5)
                .into(),
            Loader::OptiFine => column![
                widget::button(widget::text("Install Forge with OptiFine").size(14))
                    .on_press(Message::InstallForge(ForgeKind::OptiFine)),
                Self::get_uninstall_panel(self.file_data.config.mod_type),
            ]
            .spacing(5)
            .into(),

            Loader::NeoForge | Loader::Fabric | Loader::Quilt | Loader::Paper => {
                Self::get_uninstall_panel(self.file_data.config.mod_type).into()
            }

            _ => widget::text!("Unknown mod type: {}", self.file_data.config.mod_type).into(),
        }
    }

    fn get_optifine_install_button(
        config: &InstanceConfigJson,
    ) -> widget::Button<'static, Message, LauncherTheme> {
        if let Some(optifine) = config
            .mod_type_info
            .as_ref()
            .and_then(|n| n.optifine_jar.clone())
        {
            widget::button(
                row![
                    icons::bin_s(14),
                    widget::text("Uninstall OptiFine").size(14)
                ]
                .align_y(Alignment::Center)
                .spacing(11)
                .padding(2),
            )
            .on_press_with(move || {
                Message::UninstallLoaderConfirm(
                    Box::new(ManageModsMessage::DeleteOptiforge(optifine.clone()).into()),
                    Loader::OptiFine,
                )
            })
        } else {
            widget::button(widget::text("Install OptiFine with Forge").size(14))
                .on_press(InstallOptifineMessage::ScreenOpen.into())
        }
    }

    fn get_uninstall_panel(mod_type: Loader) -> widget::Button<'static, Message, LauncherTheme> {
        widget::button(
            row![
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
            return column![
                "Download some mods to get started",
                widget::button(widget::text("View Recommended Mods").size(14))
                    .on_press(crate::state::RecommendedModMessage::Open.into())
            ]
            .spacing(10)
            .padding(10)
            .width(Length::Fill)
            .into();
        }

        let hamburger_dropdown = widget::button(
            row![icons::lines_s(12)]
                .align_y(Alignment::Center)
                .padding(1),
        );

        let hamburger_dropdown = if let Some(MenuEditModsModal::Submenu) = self.ui_state.modal {
            hamburger_dropdown.on_press(ManageModsMessage::SetModal(None).into())
        } else {
            hamburger_dropdown
                .on_press(ManageModsMessage::SetModal(Some(MenuEditModsModal::Submenu)).into())
                .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::RoundDark))
        };

        let warn_no_loader = self.file_data.config.mod_type.is_vanilla()
            && self.sorted_mods_list.iter().any(|n| {
                if let ModListEntry::Local(l) = n {
                    l.1 == QueryType::Mods
                } else {
                    true
                }
            });

        widget::container(
            column![
                widget::Column::new()
                    .push_maybe(
                        warn_no_loader.then_some(widget::container(
                            widget::text(
                                // WARN: No loader installed
                                "You haven't installed any mod loader! Install Fabric/Forge/Quilt/NeoForge as per your mods"
                            ).size(12)
                        )
                        .width(Length::Fill)
                        .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)))
                    )
                    .push(
                        row![
                            hamburger_dropdown,

                            // Search button
                            widget::button(
                                row![icons::search_s(12)]
                                    .align_y(Alignment::Center)
                                    .padding(1),
                            )
                            .style(|t: &LauncherTheme, s| {
                                t.style_button(s, if self.search.is_some() {
                                    StyleButton::Round
                                } else {
                                    StyleButton::RoundDark
                                })
                            }).on_press(
                                if self.search.is_some() {
                                    ManageModsMessage::SetSearch(None).into()
                                } else {
                                    Message::Multiple(vec![
                                        ManageModsMessage::SetSearch(
                                            Some(String::new())
                                        ).into(),
                                        Message::CoreFocusNext
                                    ])
                                }
                            ),

                            subbutton_with_icon(icons::bin_s(12), "Delete")
                            .on_press_maybe((!self.selection.selected_mods.is_empty()).then_some(ManageModsMessage::DeleteSelected.into())),
                            // Redundant?
                            // subbutton_with_icon(icons::toggleoff_s(12), "Toggle")
                            //     .on_press(ManageModsMessage::ToggleSelected.into()),
                            subbutton_with_icon(icons::deselectall_s(12), if matches!(self.selection.state, SelectedState::All) {
                                "Unselect All"
                            } else {
                                "Select All"
                            })
                            .on_press(ManageModsMessage::SelectAll.into()),
                        ]
                        .spacing(5)
                        .wrap()
                    )
                    // Content filter row
                    .push(
                        self.get_content_filters()
                    )
                    .push(
                        if self.selection.selected_mods.is_empty() {
                            widget::text("Select some content to perform actions on them")
                        } else {
                            widget::text!("{} item{} selected", self.selection.selected_mods.len(), if self.selection.selected_mods.len() == 1 { "" } else { "s" })
                        }
                        .size(11)
                        .style(|t: &LauncherTheme| t.style_text(Color::Mid))
                    )
                    .push_maybe(self.search.as_ref().map(|search|
                        widget::text_input("Search...", search).size(14).on_input(|msg|
                            ManageModsMessage::SetSearch(Some(msg)).into()
                        )
                    ))
                    .padding(10)
                    .spacing(5),
                widget::responsive(|s| self.get_mod_list_contents(s, images)),
            ],
        )
        .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))
        .into()
    }

    fn get_content_filters(&self) -> Element<'_> {
        fn query_button(
            label: widget::Text<'_, LauncherTheme>,
            is_selected: bool,
            filter: Option<QueryType>,
        ) -> widget::Button<'_, Message, LauncherTheme> {
            widget::button(label.size(11))
                .padding([3, 6])
                .style(move |t: &LauncherTheme, s| {
                    t.style_button(
                        s,
                        if is_selected {
                            StyleButton::Round
                        } else {
                            StyleButton::RoundDark
                        },
                    )
                })
                .on_press(Message::ManageMods(ManageModsMessage::SetContentFilter(
                    filter,
                )))
        }

        row![query_button(
            widget::text("All"),
            self.content_filter.is_none(),
            None
        )]
        // The launcher doesn't support datapacks yet, so
        // not including that (only `INDEX_SUPPORTED`)
        .extend(QueryType::INDEX_SUPPORTED.iter().map(|filter| {
            let is_selected = self.content_filter.is_some_and(|n| n == *filter);
            query_button(widget::text(filter.to_string()), is_selected, Some(*filter)).into()
        }))
        .spacing(5)
        .wrap()
        .into()
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
                    if self.content_filter.is_some_and(|f| f != n.project_type()) {
                        return false;
                    }
                    let Some(search) = &self.search else {
                        return true;
                    };
                    n.name().to_lowercase().contains(&search.to_lowercase())
                })
                .map(|mod_list_entry| self.render_mod_entry(mod_list_entry, size, images)),
        ))
        .direction(widget::scrollable::Direction::Both {
            vertical: widget::scrollable::Scrollbar::new(),
            horizontal: widget::scrollable::Scrollbar::new(),
        })
        .id(widget::scrollable::Id::new("MenuEditMods:mods"))
        .on_scroll(|viewport| ManageModsMessage::ListScrolled(viewport.absolute_offset()).into())
        .style(LauncherTheme::style_scrollable_flat_extra_dark)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn install_ldr(loader: &str) -> widget::Button<'_, Message, LauncherTheme> {
    widget::button(widget::text(loader).size(14)).width(90)
}
