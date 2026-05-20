use cfg_if::cfg_if;
use frostmark::MarkWidget;
use iced::widget::{column, horizontal_space, row, text_editor, tooltip::Position, vertical_space};
use iced::{Alignment, Length, Padding, widget};
use ql_core::{Instance, InstanceKind, LAUNCHER_VERSION_NAME};

use crate::cli::EXPERIMENTAL_MMC_IMPORT;
use crate::menu_renderer::onboarding::x86_warning;
use crate::menu_renderer::{
    CTXI_SIZE, Column, FONT_MONO, barthin, ctx_button_icon, ctxbox, sidebar, tsubtitle, underline,
    view_info_message,
};
use crate::state::{
    GameLogMessage, InstanceNotes, LaunchMessage, LaunchModal, LauncherSettingsTab,
    MainMenuMessage, NotesMessage, ShortcutMessage, SidebarMessage, SidebarScroll, WindowMessage,
};
use crate::{
    icons,
    menu_renderer::DISCORD,
    state::{
        AccountMessage, CreateInstanceMessage, InstanceLog, LaunchTab, Launcher,
        LauncherSettingsMessage, ManageModsMessage, MenuLaunch, Message, OFFLINE_ACCOUNT_NAME,
        State,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

use super::{Element, button_with_icon, shortcut_ctrl, tooltip};

pub const TAB_BUTTON_WIDTH: f32 = 64.0;

const fn tab_height(decor: bool) -> f32 {
    if decor { 31.0 } else { 28.0 }
}

const fn decorh(decor: bool) -> f32 {
    if decor { 0.0 } else { 32.0 }
}

pub(super) fn import_description() -> widget::Row<'static, Message, LauncherTheme> {
    row![
        icons::upload_s(11),
        column![
            widget::text("Import Instance").size(13),
            widget::text("(MultiMC/Prism/QuantumLauncher...)")
                .size(10)
                .style(tsubtitle)
        ]
    ]
    .align_y(Alignment::Center)
    .spacing(10)
}

impl Launcher {
    pub fn view_main_menu<'element>(
        &'element self,
        menu: &'element MenuLaunch,
    ) -> Element<'element> {
        widget::stack!(
            widget::pane_grid(&menu.sidebar_grid_state, |_, is_sidebar, _| {
                widget::mouse_area(if *is_sidebar {
                    self.get_sidebar(menu)
                } else {
                    self.get_tab(menu)
                })
                .on_press(Message::CoreHideModal)
                .into()
            })
            .on_resize(5, |t| SidebarMessage::Resize(t.ratio).into())
        )
        .push_maybe(Self::sidebar_context_menu(menu))
        .push_maybe(self.sidebar_drag_tooltip(menu))
        .into()
    }

    fn get_tab<'a>(&'a self, menu: &'a MenuLaunch) -> Element<'a> {
        let decor = self.config.uses_system_decorations();

        let tab_body = if let Some(selected) = &self.selected_instance {
            match menu.tab {
                LaunchTab::Buttons => self.get_tab_main(menu, selected),
                LaunchTab::Log => self.get_tab_logs(menu, selected.kind).into(),
                LaunchTab::Edit => {
                    if let Some(menu) = &menu.edit_instance {
                        menu.view(selected, self.custom_jar.as_ref())
                    } else {
                        column![
                            "Error: This instance hadn't finished downloading, or files are missing\n(Couldn't read config.json)",
                            button_with_icon(icons::bin(), "Delete Instance", 16)
                                .on_press(Message::DeleteInstanceMenu)
                        ]
                        .padding(10)
                        .spacing(10)
                        .into()
                    }
                }
            }
        } else {
            column![
                widget::text(if self.client_list.as_ref().is_some_and(Vec::is_empty) {
                    "Click \"New\" to create your first Minecraft instance"
                } else {
                    "Select an instance"
                })
                .size(14)
                .style(|t: &LauncherTheme| t.style_text(Color::Mid))
            ]
            .push_maybe(cfg!(target_arch = "x86").then(x86_warning))
            .push(vertical_space())
            .push(get_footer_text())
            .padding(16)
            .spacing(10)
            .into()
        };

        let mmc_import = EXPERIMENTAL_MMC_IMPORT.read().unwrap();

        widget::stack!(
            column![
                menu.get_tab_selector(decor),
                widget::horizontal_rule(1).style(barthin)
            ]
            .push_maybe(
                menu.message
                    .as_ref()
                    .map(|n| view_info_message(n, MainMenuMessage::SetInfoMessage(None).into()))
            )
            .push(
                widget::container(tab_body)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(|t: &LauncherTheme| t.style_container_bg(0.0, None)),
            )
        )
        .push_maybe(if let Some(LaunchModal::InstanceOptions) = &menu.modal {
            Some(
                column![
                    vertical_space(),
                    ctxbox(
                        column![
                            // Not ready for production yet
                            // ctx_button_icon(icons::file_zip_s(CTXI_SIZE), "Export Instance")
                            //     .on_press(Message::ExportInstanceOpen),
                            ctx_button_icon(icons::file_gear_s(CTXI_SIZE), "Create Shortcut")
                                .on_press(ShortcutMessage::Open.into()),
                        ]
                        .push_maybe(mmc_import.then_some(widget::horizontal_rule(1)))
                        .push_maybe(mmc_import.then(|| {
                            widget::button(import_description())
                                .width(Length::Fill)
                                .style(|t: &LauncherTheme, s| {
                                    t.style_button(s, StyleButton::FlatDark)
                                })
                                .padding(2)
                                .on_press(CreateInstanceMessage::Import.into())
                        }))
                        .spacing(4)
                    )
                    .width(150),
                    widget::Space::with_height(30)
                ]
                .padding(12),
            )
        } else {
            None
        })
        .into()
    }

    fn get_tab_main<'a>(&'a self, menu: &'a MenuLaunch, selected: &'a Instance) -> Element<'a> {
        let is_running = self.is_process_running(selected);

        let main_buttons = row![
            if selected.is_server() {
                self.get_server_play_button(selected)
            } else {
                self.get_client_play_button(selected)
            },
            Self::get_mods_button(),
            Self::get_files_button(selected),
        ]
        .spacing(5)
        .wrap();

        let notes: Element = match &menu.notes {
            None => vertical_space().into(),
            Some(InstanceNotes::Viewing { content, .. }) if content.trim().is_empty() => {
                vertical_space().into()
            }
            Some(InstanceNotes::Viewing { mark_state, .. }) => widget::scrollable(
                column![MarkWidget::new(mark_state).heading_scale(0.7).text_size(14)].padding(5),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
            Some(InstanceNotes::Editing { text_editor, .. }) => {
                return column![
                    widget::text("Editing Notes").size(20),
                    widget::text_editor(text_editor)
                        .size(14)
                        .height(Length::Fill)
                        .on_action(|a| NotesMessage::Edit(a).into()),
                    row![
                        button_with_icon(icons::floppydisk_s(14), "Save", 14)
                            .on_press(NotesMessage::SaveEdit.into()),
                        button_with_icon(icons::close_s(14), "Cancel", 14)
                            .on_press(NotesMessage::CancelEdit.into()),
                    ]
                    .spacing(5)
                ]
                .padding(10)
                .spacing(10)
                .into();
            }
        };

        column![
            row![widget::text(selected.get_name()).font(FONT_MONO).size(20)]
                .push_maybe(is_running.then_some(icons::play_s(20)))
                .push_maybe(
                    is_running.then_some(
                        widget::text("Running...")
                            .size(16)
                            .style(tsubtitle)
                            .font(FONT_MONO)
                    )
                )
                .spacing(16)
                .align_y(Alignment::Center),
            main_buttons,
            notes,
            row![
                row![
                    widget::button(icons::lines_s(10)).padding([5, 8]).on_press(
                        MainMenuMessage::Modal(Some(LaunchModal::InstanceOptions)).into()
                    ),
                    widget::button(
                        row![
                            icons::edit_s(10),
                            widget::text("Edit Notes").size(12).style(tsubtitle)
                        ]
                        .align_y(Alignment::Center)
                        .spacing(8),
                    )
                    .padding([4, 8])
                    .on_press(NotesMessage::OpenEdit.into()),
                ]
                .spacing(5),
                get_footer_text(),
            ]
            .align_y(Alignment::End)
        ]
        .padding(16)
        .spacing(10)
        .into()
    }

    fn get_mods_button() -> widget::Button<'static, Message, LauncherTheme> {
        button_with_icon(icons::download(), "Mods", 15)
            .on_press(ManageModsMessage::Open.into())
            .width(98)
    }

    fn get_tab_logs<'element>(
        &'element self,
        menu: &'element MenuLaunch,
        kind: InstanceKind,
    ) -> widget::Column<'element, Message, LauncherTheme> {
        const TEXT_SIZE: f32 = 12.0;

        let State::Launch(MenuLaunch {
            log_state: Some(log_state),
            ..
        }) = &self.state
        else {
            return get_no_logs_message();
        };

        let Some(InstanceLog {
            log: log_data,
            has_crashed,
            command,
        }) = self
            .selected_instance
            .as_ref()
            .and_then(|selection| self.logs.get(selection))
        else {
            return get_no_logs_message();
        };

        let log = widget::text_editor(&log_state.content)
            .font(FONT_MONO)
            .size(TEXT_SIZE)
            .height(Length::Fill)
            .on_action(|a| GameLogMessage::Action(a).into());

        let small_button = |t| widget::button(widget::text(t).size(12)).padding([4, 8]);

        column![
            row![
                small_button("Copy Log").on_press(GameLogMessage::Copy.into()),
                small_button("Upload Log").on_press_maybe(
                    (!log_data.is_empty() && !menu.is_uploading_mclogs)
                        .then_some(GameLogMessage::Upload.into())
                ),
                small_button("Join Discord").on_press(Message::CoreOpenLink(DISCORD.to_owned())),
                widget::horizontal_space(),
                widget::mouse_area(widget::container(icons::arrow_up_s(12))).on_press(
                    GameLogMessage::Action(text_editor::Action::Move(text_editor::Motion::PageUp))
                        .into()
                ),
                widget::mouse_area(widget::container(icons::arrow_down_s(12))).on_press(
                    Message::GameLog(GameLogMessage::Action(text_editor::Action::Move(
                        text_editor::Motion::PageDown
                    )))
                ),
            ]
            .spacing(7),
            widget::text(" Having issues? Copy and send the game log for support").size(12)
        ]
        .push_maybe(
            has_crashed.then_some(
                widget::text!(
                    "The {} has crashed!",
                    match kind {
                        InstanceKind::Client => "game",
                        InstanceKind::Server => "server",
                    }
                )
                .size(18),
            ),
        )
        .push_maybe(
            matches!(kind, InstanceKind::Server).then_some(
                widget::text_input("Enter command...", command)
                    .on_input(Message::ServerCommandEdit)
                    .on_submit(Message::ServerCommandSubmit)
                    .width(190),
            ),
        )
        .push(log)
        .padding(10)
        .spacing(5)
    }

    fn get_sidebar<'a>(&'a self, menu: &'a MenuLaunch) -> Element<'a> {
        let decor = self.config.uses_system_decorations();

        let list = if let Some(sidebar) = &self.config.sidebar {
            widget::column(
                sidebar
                    .list
                    .iter()
                    .map(|node| self.get_node_rendered(menu, node, sidebar::NodeMode::InTree(0))),
            )
            .push(widget::Space::with_height(10))
        } else {
            let dots = ".".repeat((self.tick_timer % 3) + 1);
            column![widget::text!("Loading{dots}")].padding(10)
        };

        let list = column![
            widget::mouse_area(
                widget::scrollable(list)
                    .height(Length::Fill)
                    .style(LauncherTheme::style_scrollable_flat_extra_dark)
                    .id(widget::scrollable::Id::new("MenuLaunch:sidebar"))
                    .on_scroll(|n| {
                        let remaining = n.content_bounds().height - n.bounds().height;
                        SidebarMessage::Scroll(SidebarScroll {
                            remaining,
                            offset: n.absolute_offset().y,
                            bounds: Some(n.bounds()),
                        })
                        .into()
                    })
            )
            .on_right_press(
                MainMenuMessage::Modal(Some(LaunchModal::SCtxMenu(
                    None,
                    self.window_state.mouse_pos
                )))
                .into()
            )
            .on_press(SidebarMessage::DragDrop(None).into()),
            widget::horizontal_rule(1).style(|t: &LauncherTheme| t.style_rule(Color::Dark, 1)),
            self.get_accounts_bar(menu),
        ]
        .spacing(5)
        .width(Length::Fill);

        column![
            widget::mouse_area(
                widget::container(get_sidebar_new_button(decor))
                    .align_y(Alignment::End)
                    .width(Length::Fill)
                    .height(tab_height(decor) + decorh(decor))
                    .style(|t: &LauncherTheme| t.style_container_bg_semiround(
                        [true, false, false, false],
                        Some((Color::ExtraDark, t.alpha))
                    ))
            )
            .on_press(WindowMessage::Dragged.into()),
            widget::container(list)
                .height(Length::Fill)
                .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))
        ]
        .into()
    }

    pub(super) fn get_running_icon(
        &self,
        name: &str,
        kind: InstanceKind,
    ) -> Option<widget::Row<'static, Message, LauncherTheme>> {
        if self.is_process_running(&Instance::new(name, kind)) {
            Some(row![
                horizontal_space(),
                icons::play_s(12),
                widget::Space::with_width(16),
            ])
        } else {
            None
        }
    }

    fn is_process_running(&self, instance: &Instance) -> bool {
        self.processes.contains_key(instance)
    }

    fn get_accounts_bar(&self, menu: &MenuLaunch) -> Element<'_> {
        let something_is_happening = self.java_recv.is_some() || menu.login_progress.is_some();

        let dropdown: Element = if something_is_happening {
            widget::text_input("", &self.account_selected)
                .width(Length::Fill)
                .into()
        } else {
            widget::pick_list(
                self.accounts_dropdown.as_slice(),
                Some(&self.account_selected),
                |n| AccountMessage::Selected(n).into(),
            )
            .width(Length::Fill)
            .into()
        };

        widget::column![
            widget::row![widget::text(" Accounts:").size(14), horizontal_space()].push_maybe(
                (self.account_selected != OFFLINE_ACCOUNT_NAME).then_some(
                    widget::button(widget::text("Logout").size(11))
                        .padding(3)
                        .on_press(AccountMessage::LogoutCheck.into())
                        .style(|n: &LauncherTheme, status| n
                            .style_button(status, StyleButton::FlatExtraDark))
                )
            ),
            dropdown
        ]
        .push_maybe(
            (self.account_selected == OFFLINE_ACCOUNT_NAME).then_some(
                widget::text_input("Enter username...", &self.config.username)
                    .on_input(|t| MainMenuMessage::UsernameSet(t).into())
                    .width(Length::Fill),
            ),
        )
        .padding(Padding::from(5).top(0).bottom(7))
        .spacing(5)
        .into()
    }

    fn get_client_play_button(
        &'_ self,
        selected: &Instance,
    ) -> widget::Tooltip<'_, Message, LauncherTheme> {
        let play_button = button_with_icon(icons::play(), "Play", 16).width(98);
        let is_offline = self.account_selected == OFFLINE_ACCOUNT_NAME;

        if self.config.username.is_empty() && is_offline {
            tooltip(play_button, "Username is empty!", Position::Bottom)
        } else if self.config.username.contains(' ') && is_offline {
            tooltip(play_button, "Username contains spaces!", Position::Bottom)
        } else if self.processes.contains_key(selected) {
            tooltip(
                button_with_icon(icons::play(), "Kill", 16)
                    .on_press(LaunchMessage::Kill.into())
                    .width(98),
                shortcut_ctrl("Backspace"),
                Position::Bottom,
            )
        } else if self.is_launching_game {
            tooltip(
                button_with_icon(icons::play(), "...", 16).width(98),
                "Launching game...",
                Position::Bottom,
            )
        } else {
            tooltip(
                play_button.on_press(LaunchMessage::Start.into()),
                shortcut_ctrl("Enter"),
                Position::Bottom,
            )
        }
    }

    fn get_files_button(
        selected_instance: &Instance,
    ) -> widget::Button<'_, Message, LauncherTheme> {
        button_with_icon(icons::folder(), "Files", 16)
            .on_press(Message::CoreOpenPath(
                selected_instance.get_dot_minecraft_path(),
            ))
            .width(97)
    }

    fn get_server_play_button(
        &self,
        selected: &Instance,
    ) -> widget::Tooltip<'_, Message, LauncherTheme> {
        if self.processes.contains_key(selected) {
            tooltip(
                button_with_icon(icons::play(), "Stop", 16)
                    .width(98)
                    .on_press(LaunchMessage::Kill.into()),
                shortcut_ctrl("Escape"),
                Position::Bottom,
            )
        } else if self.is_launching_game {
            tooltip(
                button_with_icon(icons::play(), "...", 16).width(98),
                "Starting server...",
                Position::Bottom,
            )
        } else {
            tooltip(
                button_with_icon(icons::play(), "Start", 16)
                    .width(98)
                    .on_press(LaunchMessage::Start.into()),
                "By starting the server, you agree to the EULA",
                Position::Bottom,
            )
        }
    }
}

impl MenuLaunch {
    fn get_tab_selector(&'_ self, decor: bool) -> Element<'_> {
        let tab_bar = widget::row(
            [LaunchTab::Buttons, LaunchTab::Edit, LaunchTab::Log]
                .into_iter()
                .map(|n| render_tab_button(n, decor, self)),
        )
        .align_y(Alignment::End)
        .wrap();

        let settings_button = widget::button(
            row![horizontal_space(), icons::gear_s(12), horizontal_space()]
                .width(tab_height(decor) + 4.0)
                .height(tab_height(decor) + 4.0)
                .align_y(Alignment::Center),
        )
        .padding(0)
        .style(|n, status| n.style_button(status, StyleButton::FlatExtraDark))
        .on_press(LauncherSettingsMessage::Open(LauncherSettingsTab::default()).into());

        widget::mouse_area(
            widget::container(
                row![settings_button, tab_bar, horizontal_space()]
                    // .push_maybe(window_handle_buttons)
                    .height(tab_height(decor) + decorh(decor))
                    .align_y(Alignment::End),
            )
            .width(Length::Fill)
            .style(move |n| {
                n.style_container_bg_semiround(
                    [false, !decor, false, false],
                    Some((Color::ExtraDark, 1.0)),
                )
            }),
        )
        .on_press(WindowMessage::Dragged.into())
        .into()
    }
}

fn render_tab_button(tab: LaunchTab, decor: bool, menu: &'_ MenuLaunch) -> Element<'_> {
    let padding = Padding {
        top: 5.0,
        right: 5.0,
        bottom: if decor { 5.0 } else { 7.0 },
        left: 5.0,
    };

    let name = widget::text(tab.to_string()).size(15);

    let txt: Element = if let LaunchTab::Log = tab {
        if menu
            .message
            .as_ref()
            .is_some_and(|n| n.text.contains("crashed!"))
        {
            underline(name, Color::Mid).into()
        } else {
            name.into()
        }
    } else {
        name.into()
    };

    let txt = row![horizontal_space(), txt, horizontal_space()];

    if menu.tab == tab {
        widget::container(txt)
            .style(move |t: &LauncherTheme| {
                if decor {
                    t.style_container_selected_flat_button()
                } else {
                    t.style_container_selected_flat_button_semi([true, true, false, false])
                }
            })
            .padding(padding)
            .width(TAB_BUTTON_WIDTH)
            .height(tab_height(decor) + 4.0)
            .align_y(Alignment::End)
            .into()
    } else {
        widget::button(
            row![txt]
                .width(TAB_BUTTON_WIDTH)
                .height(tab_height(decor) + 4.0)
                .padding(padding)
                .align_y(Alignment::End),
        )
        .style(move |n, status| {
            n.style_button(
                status,
                StyleButton::SemiExtraDark([!decor, !decor, false, false]),
            )
        })
        .on_press(MainMenuMessage::ChangeTab(tab).into())
        .padding(0)
        .into()
    }
}

fn get_no_logs_message<'a>() -> Column<'a> {
    const BASE_MESSAGE: &str = "No logs found";

    column![widget::text(BASE_MESSAGE).style(|t: &LauncherTheme| t.style_text(Color::Mid))]
        // WARN: non x86_64
        .push_maybe(cfg!(not(target_arch = "x86_64")).then_some(widget::text(
            "This version is experimental. If you want to get help join our discord",
        )))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(10)
        .spacing(10)
}

fn get_footer_text() -> widget::Column<'static, Message, LauncherTheme> {
    cfg_if! (
        if #[cfg(feature = "simulate_linux_arm64")] {
            let subtext = "(Simulating Linux aarch64)";
        } else if #[cfg(feature = "simulate_linux_arm32")] {
            let subtext = "(Simulating Linux arm32)";
        } else if #[cfg(feature = "simulate_macos_arm64")] {
            let subtext = "(Simulating macOS aarch64)";
        } else if #[cfg(target_arch = "aarch64")] {
            let subtext = "A Minecraft Launcher by Mrmayman\n(Running on aarch64)";
        } else if #[cfg(target_arch = "arm")] {
            let subtext = "A Minecraft Launcher by Mrmayman\n(Running on arm32)";
        } else if #[cfg(target_arch = "x86")] {
            let subtext = "You are running the 32 bit version.\nTry using the 64 bit version if possible.";
        } else {
            let subtext = "A Minecraft Launcher by Mrmayman";
        }
    );

    column![
        row![
            horizontal_space(),
            widget::text!("QuantumLauncher v{LAUNCHER_VERSION_NAME}")
                .size(12)
                .style(|t: &LauncherTheme| t.style_text(Color::Mid))
        ],
        row![
            horizontal_space(),
            widget::text(subtext)
                .size(10)
                .style(|t: &LauncherTheme| t.style_text(Color::Mid))
        ],
    ]
}

fn get_sidebar_new_button(decor: bool) -> widget::Button<'static, Message, LauncherTheme> {
    widget::button(
        row![icons::new(), widget::text("New").size(15)]
            .align_y(Alignment::Center)
            .height(tab_height(decor) - 6.0)
            .spacing(10),
    )
    .style(move |n, status| {
        n.style_button(
            status,
            if decor {
                StyleButton::FlatDark
            } else {
                StyleButton::SemiDarkBorder([true, true, false, false])
            },
        )
    })
    .on_press(Message::CreateInstance(CreateInstanceMessage::ScreenOpen(
        InstanceKind::Client,
    )))
    .width(Length::Fill)
}
