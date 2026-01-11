use cfg_if::cfg_if;
use frostmark::MarkWidget;
use iced::keyboard::Modifiers;
use iced::widget::tooltip::Position;
use iced::widget::{horizontal_space, vertical_space};
use iced::{widget, Alignment, Length, Padding};
use ql_core::{InstanceSelection, LAUNCHER_VERSION_NAME};

use crate::cli::EXPERIMENTAL_SERVERS;
use crate::menu_renderer::onboarding::x86_warning;
use crate::menu_renderer::{ctx_button, ctxbox, tsubtitle, underline, underline_maybe, FONT_MONO};
use crate::state::{GameLogMessage, InstanceNotes, LaunchModal, NotesMessage, WindowMessage};
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
        widget::pane_grid(&menu.sidebar_grid_state, |_, is_sidebar, _| {
            if *is_sidebar {
                self.get_sidebar(menu).into()
            } else {
                self.get_tab(menu).into()
            }
        })
        .on_resize(10, |t| Message::MSidebarResize(t.ratio))
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
                        widget::column!(
                            "Error: This instance is corrupted/invalid!\n(Couldn't read config.json)",
                            button_with_icon(icons::bin(), "Delete Instance", 16)
                                .on_press(Message::DeleteInstanceMenu)
                        )
                        .padding(10)
                        .spacing(10)
                        .into()
                    }
                }
            }
        } else {
            widget::column!(widget::text(if menu.is_viewing_server {
                "Select a server\n\nNote: You are trying the *early-alpha* server manager feature.\nYou need playit.gg (or port-forwarding) for others to join"
            } else {
                "Select an instance"
            })
            .size(14)
            .style(|t: &LauncherTheme| t.style_text(Color::Mid)))
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

        widget::stack!(widget::column![menu.get_tab_selector(decor)]
            .push_maybe(view_info_message(menu))
            .push(
                widget::container(tab_body)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(|t: &LauncherTheme| t.style_container_bg(0.0, None)),
            ))
        .push_maybe(menu.modal.as_ref().map(|n| {
            match n {
                LaunchModal::InstanceOptions => widget::column![
                    widget::vertical_space(),
                    ctxbox(
                        widget::column![
                            ctx_button("Export Instance").on_press(Message::ExportInstanceOpen),
                            ctx_button("Create Shortcut").on_press(Message::Nothing),
                        ]
                        .spacing(5)
                    )
                    .width(150),
                    widget::Space::with_height(30)
                ]
                .padding(12),
            }
        }))
        .into()
    }

    fn get_tab_main<'a>(&'a self, menu: &'a MenuLaunch) -> Element<'a> {
        let selected = self.instance();
        let is_running = self.is_process_running(selected);

        let main_buttons = widget::row![
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
                widget::column![MarkWidget::new(mark_state).heading_scale(0.7).text_size(14)]
                    .padding(5),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
            Some(InstanceNotes::Editing { text_editor, .. }) => {
                return widget::column!(
                    widget::text("Editing Notes").size(20),
                    widget::text_editor(text_editor)
                        .size(14)
                        .height(Length::Fill)
                        .on_action(|a| Message::Notes(NotesMessage::Edit(a))),
                    widget::row![
                        button_with_icon(icons::floppydisk_s(14), "Save", 14)
                            .on_press(Message::Notes(NotesMessage::SaveEdit)),
                        button_with_icon(icons::close_s(14), "Cancel", 14)
                            .on_press(Message::Notes(NotesMessage::CancelEdit)),
                    ]
                    .spacing(5)
                )
                .padding(10)
                .spacing(10)
                .into();
            }
        };

        widget::column!(
            widget::row![widget::text(selected.get_name()).font(FONT_MONO).size(20)]
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
            widget::row![
                widget::Column::new()
                    .push_maybe(get_view_servers(menu.is_viewing_server))
                    .push(
                        widget::row![
                            widget::button(icons::lines_s(10))
                                .padding([4, 8])
                                .on_press(Message::MModal(Some(LaunchModal::InstanceOptions))),
                            widget::button(
                                widget::row![
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
        )
        .padding(16)
        .spacing(10)
        .into()
    }

    fn get_mods_button(&self) -> widget::Button<'_, Message, LauncherTheme> {
        button_with_icon(icons::download(), "Mods", 15)
            .on_press(if self.modifiers_pressed.contains(Modifiers::SHIFT) {
                Message::ManageMods(ManageModsMessage::ScreenOpenWithoutUpdate)
            } else {
                Message::ManageMods(ManageModsMessage::ScreenOpen)
            })
            .width(98)
    }

    pub fn get_tab_logs<'element>(
        &'element self,
        menu: &'element MenuLaunch,
    ) -> widget::Column<'element, Message, LauncherTheme> {
        const TEXT_SIZE: f32 = 12.0;

        let scroll = if let State::Launch(MenuLaunch { log_scroll, .. }) = &self.state {
            *log_scroll
        } else {
            0
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
            return get_no_logs_message().padding(10).spacing(10);
        };

        let log = Self::view_launcher_log(
            log_data.clone(),
            TEXT_SIZE,
            scroll,
            |s| Message::GameLog(GameLogMessage::Scroll(s)),
            |s| Message::GameLog(GameLogMessage::ScrollAbsolute(s)),
            |msg| {
                widget::text(msg.clone())
                    .font(iced::Font::with_name("JetBrains Mono"))
                    .size(TEXT_SIZE)
                    .width(Length::Fill)
                    .into()
            },
            |msg| msg.clone(),
        );

        widget::column![
            widget::row![
                widget::button(widget::text("Copy Log").size(14))
                    .on_press(Message::GameLog(GameLogMessage::Copy)),
                widget::button(widget::text("Upload Log").size(14)).on_press_maybe(
                    (!log_data.is_empty() && !menu.is_uploading_mclogs)
                        .then_some(Message::GameLog(GameLogMessage::Upload))
                ),
                widget::button(widget::text("Join Discord").size(14))
                    .on_press(Message::CoreOpenLink(DISCORD.to_owned())),
            ]
            .spacing(7),
            widget::text("Having issues? Copy and send the game log for support").size(12)
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
        .spacing(10)
    }

    fn get_sidebar<'a>(&'a self, menu: &'a MenuLaunch) -> Element<'a> {
        let list = if menu.is_viewing_server {
            self.server_list.as_deref()
        } else {
            self.client_list.as_deref()
        };

        let decor = self.config.uses_system_decorations();

        let list = if let Some(instances) = list {
            widget::column(instances.iter().map(|name| {
                let playing_icon = if self
                    .is_process_running(&InstanceSelection::new(name, menu.is_viewing_server))
                {
                    Some(widget::row![
                        horizontal_space(),
                        icons::play_s(15),
                        widget::Space::with_width(10),
                    ])
                } else {
                    None
                };

                let text = widget::text(name).size(15).style(tsubtitle);
                let is_selected = self
                    .selected_instance
                    .as_ref()
                    .is_some_and(|n| n.get_name() == name);

                underline_maybe(
                    widget::button(widget::row![text].push_maybe(playing_icon))
                        .style(|n: &LauncherTheme, status| {
                            n.style_button(status, StyleButton::FlatExtraDark)
                        })
                        .on_press_maybe((!is_selected).then(|| {
                            Message::LaunchInstanceSelected(InstanceSelection::new(
                                name,
                                menu.is_viewing_server,
                            ))
                        }))
                        .width(Length::Fill),
                    Color::Dark,
                    !is_selected,
                )
            }))
        } else {
            let dots = ".".repeat((self.tick_timer % 3) + 1);
            widget::column![widget::text!("Loading{dots}")].padding(10)
        };

        let list = widget::column![
            widget::scrollable(list)
                .height(Length::Fill)
                .style(LauncherTheme::style_scrollable_flat_extra_dark)
                .id(widget::scrollable::Id::new("MenuLaunch:sidebar"))
                .on_scroll(|n| {
                    let total = n.content_bounds().height - n.bounds().height;
                    Message::MSidebarScroll(total)
                }),
            widget::horizontal_rule(1).style(|t: &LauncherTheme| t.style_rule(Color::Dark, 1)),
            self.get_accounts_bar(menu),
        ]
        .spacing(5)
        .width(Length::Fill);

        widget::column![
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

        widget::column![
            widget::row![widget::text(" Accounts:").size(14), horizontal_space(),].push_maybe(
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
                    .on_input(Message::LaunchUsernameSet)
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
        } else if let Some(selected_instance) = &self.selected_instance {
            if self.processes.contains_key(selected_instance) {
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
        } else {
            tooltip(play_button, "Select an instance first!", Position::Bottom).into()
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

    fn get_server_play_button<'a>(&self) -> iced::widget::Tooltip<'a, Message, LauncherTheme> {
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
                    .on_press_maybe(
                        self.selected_instance
                            .is_some()
                            .then(|| Message::LaunchStart),
                    ),
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
            widget::row![horizontal_space(), icons::gear_s(12), horizontal_space()]
                .width(tab_height(decor) + 4.0)
                .height(tab_height(decor) + 4.0)
                .align_y(Alignment::Center),
        )
        .padding(0)
        .style(|n, status| n.style_button(status, StyleButton::FlatExtraDark))
        .on_press(Message::LauncherSettings(LauncherSettingsMessage::Open));

        widget::mouse_area(
            widget::container(
                widget::row!(settings_button, tab_bar, horizontal_space())
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

    let txt = widget::row!(horizontal_space(), txt, horizontal_space());

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
            widget::row![txt]
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
        .on_press(Message::MChangeTab(tab))
        .padding(0)
        .into()
    }
}

fn get_no_logs_message<'a>() -> widget::Column<'a, Message, LauncherTheme> {
    const BASE_MESSAGE: &str = "No logs found";

    widget::column!(widget::text(BASE_MESSAGE).style(|t: &LauncherTheme| t.style_text(Color::Mid)))
        // WARN: non x86_64
        .push_maybe(cfg!(not(target_arch = "x86_64")).then_some(widget::text(
            "This version is experimental. If you want to get help join our discord",
        )))
        .width(Length::Fill)
        .height(Length::Fill)
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

    widget::column!(
        widget::row!(
            horizontal_space(),
            widget::text!("QuantumLauncher v{LAUNCHER_VERSION_NAME}")
                .size(12)
                .style(|t: &LauncherTheme| t.style_text(Color::Mid))
        ),
        widget::row!(
            horizontal_space(),
            widget::text(subtext)
                .size(10)
                .style(|t: &LauncherTheme| t.style_text(Color::Mid))
        ),
    )
}

fn get_sidebar_new_button(
    menu: &MenuLaunch,
    decor: bool,
) -> widget::Button<'_, Message, LauncherTheme> {
    widget::button(
        widget::row![icons::new(), widget::text("New").size(15)]
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
            widget::row![
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
