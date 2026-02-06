use cfg_if::cfg_if;
use frostmark::MarkWidget;
use iced::widget::{column, horizontal_space, row, text_editor, tooltip::Position, vertical_space};
use iced::{keyboard::Modifiers, widget, Alignment, Length, Padding};
use ql_core::{InstanceSelection, LAUNCHER_VERSION_NAME};

use crate::cli::EXPERIMENTAL_SERVERS;
use crate::menu_renderer::onboarding::x86_warning;
use crate::menu_renderer::{ctx_button, ctxbox, sidebar, tsubtitle, underline, FONT_MONO};
use crate::state::{
    GameLogMessage, InstanceNotes, LaunchModal, MainMenuMessage, NotesMessage, WindowMessage,
};
use crate::{
    icons,
    menu_renderer::DISCORD,
    state::{
        AccountMessage, CreateInstanceMessage, InstanceLog, LaunchTab, Launcher,
        LauncherSettingsMessage, ManageModsMessage, MenuLaunch, Message, State, NEW_ACCOUNT_NAME,
        OFFLINE_ACCOUNT_NAME,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

use super::{button_with_icon, shortcut_ctrl, tooltip, Element};

pub const TAB_BUTTON_WIDTH: f32 = 64.0;

const fn tab_height(decor: bool) -> f32 {
    if decor {
        31.0
    } else {
        28.0
    }
}

const fn decorh(decor: bool) -> f32 {
    if decor {
        0.0
    } else {
        32.0
    }
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
            .on_resize(10, |t| MainMenuMessage::SidebarResize(t.ratio).into())
        )
        .push_maybe(sidebar::context_menu(menu))
        .push_maybe(self.sidebar_drag_tooltip(menu))
        .into()
    }

    fn get_tab<'a>(&'a self, menu: &'a MenuLaunch) -> Element<'a> {
        let decor = self.config.uses_system_decorations();

        let tab_body = if let Some(selected) = &self.selected_instance {
            match menu.tab {
                LaunchTab::Buttons => self.get_tab_main(menu),
                LaunchTab::Log => self.get_tab_logs(menu).into(),
                LaunchTab::Edit => {
                    if let Some(menu) = &menu.edit_instance {
                        menu.view(selected, self.custom_jar.as_ref())
                    } else {
                        column![
                            "Error: This instance is corrupted/invalid!\n(Couldn't read config.json)",
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
            column![widget::text(if menu.is_viewing_server {
                "Select a server\n\nNote: You are trying the *early-alpha* server manager feature.\nYou need playit.gg (or port-forwarding) for others to join"
            } else {
                "Select an instance"
            })
            .size(14)
            .style(|t: &LauncherTheme| t.style_text(Color::Mid))]
            .push_maybe(cfg!(target_arch = "x86").then(x86_warning))
            .push(vertical_space())
            .push(
                widget::Row::new()
                    .push_maybe(get_view_servers(menu.is_viewing_server))
                    .push(get_footer_text())
                    .align_y(Alignment::End),
            )
            .padding(16)
            .spacing(10)
            .into()
        };

        widget::stack!(column![menu.get_tab_selector(decor)]
            .push_maybe(view_info_message(menu))
            .push(
                widget::container(tab_body)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(|t: &LauncherTheme| t.style_container_bg(0.0, None)),
            ))
        .push_maybe(if let Some(LaunchModal::InstanceOptions) = &menu.modal {
            Some(
                column![
                    vertical_space(),
                    ctxbox(
                        column![
                            ctx_button("Export Instance").on_press(Message::ExportInstanceOpen),
                            ctx_button("Create Shortcut"),
                        ]
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

    fn get_tab_main<'a>(&'a self, menu: &'a MenuLaunch) -> Element<'a> {
        let selected = self.instance();
        let is_running = self.is_process_running(selected);

        let main_buttons = row![
            if menu.is_viewing_server {
                self.get_server_play_button().into()
            } else {
                self.get_client_play_button()
            },
            self.get_mods_button(),
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
                        .on_action(|a| Message::Notes(NotesMessage::Edit(a))),
                    row![
                        button_with_icon(icons::floppydisk_s(14), "Save", 14)
                            .on_press(Message::Notes(NotesMessage::SaveEdit)),
                        button_with_icon(icons::close_s(14), "Cancel", 14)
                            .on_press(Message::Notes(NotesMessage::CancelEdit)),
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
            // widget::button("Export Instance").on_press(Message::ExportInstanceOpen),
            notes,
            row![
                widget::Column::new()
                    .push_maybe(get_view_servers(menu.is_viewing_server))
                    .push(
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
                            .on_press(Message::Notes(NotesMessage::OpenEdit)),
                        ]
                        .spacing(5)
                    )
                    .spacing(5),
                get_footer_text(),
            ]
            .align_y(Alignment::End)
        ]
        .padding(16)
        .spacing(10)
        .into()
    }

    fn get_mods_button(&self) -> widget::Button<'_, Message, LauncherTheme> {
        button_with_icon(icons::download(), "Mods", 15)
            .on_press(if self.modifiers_pressed.contains(Modifiers::SHIFT) {
                ManageModsMessage::ScreenOpenWithoutUpdate.into()
            } else {
                ManageModsMessage::ScreenOpen.into()
            })
            .width(98)
    }

    pub fn get_tab_logs<'element>(
        &'element self,
        menu: &'element MenuLaunch,
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
            .on_action(|a| Message::GameLog(GameLogMessage::Action(a)));

        let small_button = |t| widget::button(widget::text(t).size(12)).padding([4, 8]);

        column![
            row![
                small_button("Copy Log").on_press(Message::GameLog(GameLogMessage::Copy)),
                small_button("Upload Log").on_press_maybe(
                    (!log_data.is_empty() && !menu.is_uploading_mclogs)
                        .then_some(Message::GameLog(GameLogMessage::Upload))
                ),
                small_button("Join Discord").on_press(Message::CoreOpenLink(DISCORD.to_owned())),
                widget::horizontal_space(),
                widget::mouse_area(widget::container(icons::arrow_up_s(12))).on_press(
                    Message::GameLog(GameLogMessage::Action(text_editor::Action::Move(
                        text_editor::Motion::PageUp
                    )))
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
                    if menu.is_viewing_server {
                        "server"
                    } else {
                        "game"
                    }
                )
                .size(18),
            ),
        )
        .push_maybe(
            menu.is_viewing_server.then_some(
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
                        let total = n.content_bounds().height - n.bounds().height;
                        MainMenuMessage::SidebarScroll(total).into()
                    })
            )
            .on_right_press(
                MainMenuMessage::Modal(Some(LaunchModal::SidebarCtxMenu(
                    None,
                    self.window_state.mouse_pos
                )))
                .into()
            )
            .on_press(MainMenuMessage::DragDrop(None).into()),
            widget::horizontal_rule(1).style(|t: &LauncherTheme| t.style_rule(Color::Dark, 1)),
            self.get_accounts_bar(menu),
        ]
        .spacing(5)
        .width(Length::Fill);

        column![
            widget::mouse_area(
                widget::container(get_sidebar_new_button(menu, decor))
                    .align_y(Alignment::End)
                    .width(Length::Fill)
                    .height(tab_height(decor) + decorh(decor))
                    .style(|t: &LauncherTheme| t.style_container_bg_semiround(
                        [true, false, false, false],
                        Some((Color::ExtraDark, t.alpha))
                    ))
            )
            .on_press(Message::Window(WindowMessage::Dragged)),
            widget::container(list)
                .height(Length::Fill)
                .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))
        ]
        .into()
    }

    pub(super) fn get_running_icon(
        &self,
        menu: &MenuLaunch,
        name: &str,
    ) -> Option<widget::Row<'static, Message, LauncherTheme>> {
        if self.is_process_running(&InstanceSelection::new(name, menu.is_viewing_server)) {
            Some(row![
                horizontal_space(),
                icons::play_s(15),
                widget::Space::with_width(10),
            ])
        } else {
            None
        }
    }

    fn is_process_running(&self, instance: &InstanceSelection) -> bool {
        self.processes.contains_key(instance)
    }

    fn get_accounts_bar(&self, menu: &MenuLaunch) -> Element<'_> {
        let something_is_happening = self.java_recv.is_some() || menu.login_progress.is_some();

        let dropdown: Element = if something_is_happening {
            widget::text_input("", self.accounts_selected.as_deref().unwrap_or_default())
                .width(Length::Fill)
                .into()
        } else {
            widget::pick_list(
                self.accounts_dropdown.clone(),
                self.accounts_selected.clone(),
                |n| Message::Account(AccountMessage::Selected(n)),
            )
            .width(Length::Fill)
            .into()
        };

        column![
            row![widget::text(" Accounts:").size(14), horizontal_space(),].push_maybe(
                self.is_account_selected().then_some(
                    widget::button(widget::text("Logout").size(11))
                        .padding(3)
                        .on_press(Message::Account(AccountMessage::LogoutCheck))
                        .style(|n: &LauncherTheme, status| n
                            .style_button(status, StyleButton::FlatExtraDark))
                )
            ),
            dropdown
        ]
        .push_maybe(
            (self.accounts_selected.as_deref() == Some(OFFLINE_ACCOUNT_NAME)).then_some(
                widget::text_input("Enter username...", &self.config.username)
                    .on_input(|t| MainMenuMessage::UsernameSet(t).into())
                    .width(Length::Fill),
            ),
        )
        .padding(Padding::from(5).top(0).bottom(7))
        .spacing(5)
        .into()
    }

    pub fn is_account_selected(&self) -> bool {
        !(self.accounts_selected.is_none()
            || self.accounts_selected.as_deref() == Some(NEW_ACCOUNT_NAME)
            || self.accounts_selected.as_deref() == Some(OFFLINE_ACCOUNT_NAME))
    }

    fn get_client_play_button(&'_ self) -> Element<'_> {
        let play_button = button_with_icon(icons::play(), "Play", 16).width(98);

        let is_account_selected = self.is_account_selected();

        if self.config.username.is_empty() && !is_account_selected {
            tooltip(play_button, "Username is empty!", Position::Bottom).into()
        } else if self.config.username.contains(' ') && !is_account_selected {
            tooltip(play_button, "Username contains spaces!", Position::Bottom).into()
        } else if self.processes.contains_key(self.instance()) {
            tooltip(
                button_with_icon(icons::play(), "Kill", 16)
                    .on_press(Message::LaunchKill)
                    .width(98),
                shortcut_ctrl("Backspace"),
                Position::Bottom,
            )
            .into()
        } else if self.is_launching_game {
            button_with_icon(icons::play(), "...", 16).width(98).into()
        } else {
            tooltip(
                play_button.on_press(Message::LaunchStart),
                shortcut_ctrl("Enter"),
                Position::Bottom,
            )
            .into()
        }
    }

    fn get_files_button(
        selected_instance: &InstanceSelection,
    ) -> widget::Button<'_, Message, LauncherTheme> {
        button_with_icon(icons::folder(), "Files", 16)
            .on_press(Message::CoreOpenPath(
                selected_instance.get_dot_minecraft_path(),
            ))
            .width(97)
    }

    fn get_server_play_button(&self) -> widget::Tooltip<'_, Message, LauncherTheme> {
        match &self.selected_instance {
            Some(n) if self.processes.contains_key(n) => tooltip(
                button_with_icon(icons::play(), "Stop", 16)
                    .width(97)
                    .on_press(Message::LaunchKill),
                shortcut_ctrl("Escape"),
                Position::Bottom,
            ),
            _ => tooltip(
                button_with_icon(icons::play(), "Start", 16)
                    .width(97)
                    .on_press(Message::LaunchStart),
                "By starting the server, you agree to the EULA",
                Position::Bottom,
            ),
        }
    }
}

fn get_view_servers(
    is_viewing_server: bool,
) -> Option<widget::Button<'static, Message, LauncherTheme>> {
    let b = widget::button(
        widget::text(if is_viewing_server {
            "View Instances..."
        } else {
            "View Servers..."
        })
        .size(12)
        .style(tsubtitle),
    )
    .padding([4, 8])
    .on_press(Message::MScreenOpen {
        message: None,
        clear_selection: false,
        is_server: Some(!is_viewing_server),
    });

    EXPERIMENTAL_SERVERS.read().unwrap().then_some(b)
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
        .on_press(Message::LauncherSettings(LauncherSettingsMessage::Open));

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
        .on_press(Message::Window(WindowMessage::Dragged))
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
        if menu.message.contains("crashed!") {
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

fn get_no_logs_message<'a>() -> widget::Column<'a, Message, LauncherTheme> {
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

fn get_sidebar_new_button(
    menu: &MenuLaunch,
    decor: bool,
) -> widget::Button<'_, Message, LauncherTheme> {
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
    .on_press(Message::CreateInstance(CreateInstanceMessage::ScreenOpen {
        is_server: menu.is_viewing_server,
    }))
    .width(Length::Fill)
}

fn view_info_message(
    menu: &'_ MenuLaunch,
) -> Option<widget::Container<'_, Message, LauncherTheme>> {
    (!menu.message.is_empty()).then_some(
        widget::container(
            row![
                widget::button(
                    icons::close()
                        .style(|t: &LauncherTheme| t.style_text(Color::Mid))
                        .size(12)
                )
                .padding(0)
                .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::FlatExtraDark))
                .on_press(Message::MScreenOpen {
                    message: None,
                    clear_selection: false,
                    is_server: Some(menu.is_viewing_server)
                }),
                widget::text(&menu.message).size(12).style(tsubtitle),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(10)
        .style(|t: &LauncherTheme| t.style_container_sharp_box(0.0, Color::ExtraDark)),
    )
}
