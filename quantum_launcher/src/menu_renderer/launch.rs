use std::collections::HashMap;

use iced::keyboard::Modifiers;
use iced::widget::tooltip::Position;
use iced::{widget, Alignment, Length, Padding};
use ql_core::{InstanceSelection, LAUNCHER_VERSION_NAME};

use crate::menu_renderer::{underline, FONT_MONO};
use crate::state::WindowMessage;
use crate::{
    icon_manager,
    menu_renderer::DISCORD,
    state::{
        AccountMessage, CreateInstanceMessage, InstanceLog, LaunchTabId, Launcher,
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
        let selected_instance_s = self
            .selected_instance
            .as_ref()
            .map(InstanceSelection::get_name);

        widget::pane_grid(&menu.sidebar_grid_state, |_, is_sidebar, _| {
            if *is_sidebar {
                self.get_sidebar(selected_instance_s, menu).into()
            } else {
                self.get_tab(selected_instance_s, menu).into()
            }
        })
        .on_resize(10, |t| Message::LaunchSidebarResize(t.ratio))
        .into()
    }

    fn get_tab<'a>(
        &'a self,
        selected_instance_s: Option<&'a str>,
        menu: &'a MenuLaunch,
    ) -> Element<'a> {
        let decor = self.config.c_window_decorations();

        let last_parts = widget::column![
            widget::horizontal_space(),
            widget::row![
                // Server UI enabled
                widget::column![
                    widget::vertical_space(),
                    if menu.is_viewing_server {
                        widget::button("View Instances...").on_press(Message::LaunchScreenOpen {
                            message: None,
                            clear_selection: true,
                        })
                    } else {
                        widget::button("View Servers...").on_press(Message::ServerManageOpen {
                            selected_server: None,
                            message: None,
                        })
                    },
                ],
                get_footer_text(menu),
            ],
        ]
        .spacing(5);

        let tab_body = if let Some(selected) = &self.selected_instance {
            match menu.tab {
                LaunchTabId::Buttons => {
                    let main_buttons = widget::row![
                        if menu.is_viewing_server {
                            self.get_server_play_button(selected_instance_s).into()
                        } else {
                            self.get_client_play_button(selected_instance_s)
                        },
                        self.get_mods_button(selected_instance_s),
                        Self::get_files_button(selected),
                    ]
                    .spacing(5)
                    .wrap();

                    widget::column!(
                        widget::row![widget::text(selected.get_name()).font(FONT_MONO).size(20),]
                            .push_maybe({
                                selected_instance_s.and_then(|n| {
                                    self.is_process_running(menu, n).then_some(tooltip(
                                        icon_manager::play_with_size(20),
                                        "Running...",
                                        Position::Right,
                                    ))
                                })
                            })
                            .spacing(10),
                        main_buttons,
                        // widget::button("Export Instance").on_press(Message::ExportInstanceOpen),
                    )
                    .push(last_parts)
                    .padding(10)
                    .spacing(10)
                    .into()
                }
                LaunchTabId::Log => self
                    .get_log_pane(
                        if menu.is_viewing_server {
                            &self.server_logs
                        } else {
                            &self.client_logs
                        },
                        selected_instance_s,
                        menu,
                    )
                    .into(),
                LaunchTabId::Edit => {
                    if let Some(menu) = &menu.edit_instance {
                        menu.view(selected, self.custom_jar.as_ref())
                    } else {
                        widget::column!(
                            "Error: Could not read config json!",
                            button_with_icon(icon_manager::delete(), "Delete Instance", 16)
                                .on_press(Message::DeleteInstanceMenu)
                        )
                        .padding(10)
                        .spacing(10)
                        .into()
                    }
                }
            }
        } else {
            widget::column!(
                widget::text("Select an instance")
                    .size(14)
                    .style(|t: &LauncherTheme| t.style_text(Color::Mid)),
                last_parts
            )
            .padding(10)
            .spacing(10)
            .into()
        };

        widget::column![
            menu.get_tab_selector(decor),
            widget::container(tab_body).style(|t: &LauncherTheme| t.style_container_bg(0.0, None))
        ]
        .into()
    }

    fn get_mods_button(
        &self,
        selected_instance_s: Option<&str>,
    ) -> widget::Button<'_, Message, LauncherTheme> {
        button_with_icon(icon_manager::download(), "Mods", 15)
            .on_press_maybe(selected_instance_s.is_some().then_some(
                if self.modifiers_pressed.contains(Modifiers::SHIFT) {
                    Message::ManageMods(ManageModsMessage::ScreenOpenWithoutUpdate)
                } else {
                    Message::ManageMods(ManageModsMessage::ScreenOpen)
                },
            ))
            .width(98)
    }

    pub fn get_log_pane<'element>(
        &'element self,
        logs: &'element HashMap<String, InstanceLog>,
        selected_instance: Option<&'element str>,
        menu: &'element MenuLaunch,
    ) -> widget::Column<'element, Message, LauncherTheme> {
        const TEXT_SIZE: f32 = 12.0;

        let scroll = if let State::Launch(MenuLaunch { log_scroll, .. }) = &self.state {
            *log_scroll
        } else {
            0
        };

        let Some(Some(InstanceLog {
            log: log_data,
            has_crashed,
            command,
        })) = selected_instance
            .as_ref()
            .map(|selection| logs.get(*selection))
        else {
            return get_no_logs_message().padding(10).spacing(10);
        };

        let log = Self::view_launcher_log(
            log_data.clone(),
            TEXT_SIZE,
            scroll,
            Message::LaunchLogScroll,
            Message::LaunchLogScrollAbsolute,
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
                widget::button(widget::text("Copy Log").size(14)).on_press(Message::LaunchCopyLog),
                widget::button(widget::text("Upload Log").size(14)).on_press_maybe(
                    (!log_data.is_empty() && !menu.is_uploading_mclogs)
                        .then_some(Message::LaunchUploadLog)
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
                    .on_input(move |n| {
                        Message::ServerCommandEdit(selected_instance.unwrap().to_owned(), n)
                    })
                    .on_submit(Message::ServerCommandSubmit(
                        selected_instance.unwrap().to_owned(),
                    ))
                    .width(190),
            ),
        )
        .push(log)
        .padding(10)
        .spacing(10)
    }

    fn get_sidebar<'a>(
        &'a self,
        selected_instance_s: Option<&'a str>,
        menu: &'a MenuLaunch,
    ) -> Element<'a> {
        let list = if menu.is_viewing_server {
            self.server_list.as_deref()
        } else {
            self.client_list.as_deref()
        };

        let decor = self.config.c_window_decorations();

        let list = if let Some(instances) = list {
            widget::column(instances.iter().map(|name| {
                let playing_icon = if self.is_process_running(menu, name) {
                    Some(widget::row![
                        widget::horizontal_space(),
                        icon_manager::play_with_size(15),
                        widget::Space::with_width(10),
                    ])
                } else {
                    None
                };

                let text = widget::text(name)
                    .size(15)
                    .style(|t: &LauncherTheme| t.style_text(Color::SecondLight));

                if selected_instance_s == Some(name) {
                    widget::container(widget::row!(widget::Space::with_width(5), text))
                        .style(LauncherTheme::style_container_selected_flat_button)
                        .width(Length::Fill)
                        .padding(5)
                        .into()
                } else {
                    underline(
                        widget::button(widget::row![text].push_maybe(playing_icon))
                            .style(|n: &LauncherTheme, status| {
                                n.style_button(status, StyleButton::FlatExtraDark)
                            })
                            .on_press(Message::LaunchInstanceSelected {
                                name: name.clone(),
                                is_server: menu.is_viewing_server,
                            })
                            .width(Length::Fill),
                        Color::Dark,
                    )
                    .into()
                }
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
                    Message::LaunchSidebarScroll(total)
                }),
            widget::horizontal_rule(1).style(|t: &LauncherTheme| t.style_rule(Color::Dark, 1)),
            self.get_accounts_bar(menu),
        ]
        .spacing(5)
        .width(Length::Fill);

        widget::column![
            widget::mouse_area(
                widget::container(menu.get_sidebar_new_button(decor))
                    .align_y(Alignment::End)
                    .width(menu.get_sidebar_width())
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

    fn is_process_running(&self, menu: &MenuLaunch, name: &str) -> bool {
        (!menu.is_viewing_server && self.client_processes.contains_key(name))
            || (menu.is_viewing_server && self.server_processes.contains_key(name))
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
            widget::row![
                widget::text(" Accounts:").size(14),
                widget::horizontal_space(),
            ]
            .push_maybe(
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

    fn get_client_play_button(&'_ self, selected_instance: Option<&str>) -> Element<'_> {
        let play_button = button_with_icon(icon_manager::play(), "Play", 16).width(98);

        let is_account_selected = self.is_account_selected();

        if self.config.username.is_empty() && !is_account_selected {
            tooltip(play_button, "Username is empty!", Position::Bottom).into()
        } else if self.config.username.contains(' ') && !is_account_selected {
            tooltip(play_button, "Username contains spaces!", Position::Bottom).into()
        } else if let Some(selected_instance) = selected_instance {
            if self.client_processes.contains_key(selected_instance) {
                tooltip(
                    button_with_icon(icon_manager::play(), "Kill", 16)
                        .on_press(Message::LaunchKill)
                        .width(98),
                    shortcut_ctrl("Backspace"),
                    Position::Bottom,
                )
                .into()
            } else if self.is_launching_game {
                button_with_icon(icon_manager::play(), "...", 16)
                    .width(98)
                    .into()
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
        button_with_icon(icon_manager::folder(), "Files", 16)
            .on_press(Message::CoreOpenPath(
                selected_instance.get_dot_minecraft_path(),
            ))
            .width(97)
    }

    fn get_server_play_button<'a>(
        &self,
        selected_server: Option<&'a str>,
    ) -> iced::widget::Tooltip<'a, Message, LauncherTheme> {
        match selected_server {
            Some(n) if self.server_processes.contains_key(n) => tooltip(
                button_with_icon(icon_manager::play(), "Stop", 16)
                    .width(97)
                    .on_press_maybe(selected_server.is_some().then(|| Message::LaunchKill)),
                shortcut_ctrl("Escape"),
                Position::Bottom,
            ),
            _ => tooltip(
                button_with_icon(icon_manager::play(), "Start", 16)
                    .width(97)
                    .on_press_maybe(selected_server.is_some().then(|| Message::LaunchStart)),
                "By starting the server, you agree to the EULA",
                Position::Bottom,
            ),
        }
    }
}

impl MenuLaunch {
    fn get_tab_selector(&'_ self, decor: bool) -> Element<'_> {
        let tab_bar = widget::row(
            [LaunchTabId::Buttons, LaunchTabId::Edit, LaunchTabId::Log]
                .into_iter()
                .map(|n| self.render_tab_button(n, decor)),
        )
        .align_y(Alignment::End)
        .wrap();

        let settings_button = widget::button(
            widget::row![
                widget::horizontal_space(),
                icon_manager::settings_with_size(12),
                widget::horizontal_space()
            ]
            .width(tab_height(decor) + 4.0)
            .height(tab_height(decor) + 4.0)
            .align_y(Alignment::Center),
        )
        .padding(0)
        .style(|n, status| n.style_button(status, StyleButton::FlatExtraDark))
        .on_press(Message::LauncherSettings(LauncherSettingsMessage::Open));

        widget::mouse_area(
            widget::container(
                widget::row!(settings_button, tab_bar, widget::horizontal_space())
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

    fn get_sidebar_new_button(&self, decor: bool) -> widget::Button<'_, Message, LauncherTheme> {
        widget::button(
            widget::row![icon_manager::create(), widget::text("New").size(15)]
                .align_y(iced::alignment::Vertical::Center)
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
        .on_press(if self.is_viewing_server {
            Message::ServerCreateScreenOpen
        } else {
            Message::CreateInstance(CreateInstanceMessage::ScreenOpen)
        })
        .width(Length::Fill)
    }

    fn render_tab_button(&self, n: LaunchTabId, decor: bool) -> Element<'_> {
        let padding = Padding {
            top: 5.0,
            right: 5.0,
            bottom: if decor { 5.0 } else { 7.0 },
            left: 5.0,
        };

        let txt = widget::row!(
            widget::horizontal_space(),
            widget::text(n.to_string()).size(15),
            widget::horizontal_space(),
        );
        if self.tab == n {
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
            .on_press(Message::LaunchChangeTab(n))
            .padding(0)
            .into()
        }
    }
}

fn get_no_logs_message<'a>() -> widget::Column<'a, Message, LauncherTheme> {
    const BASE_MESSAGE: &str = "No logs found";

    widget::column!(widget::text(BASE_MESSAGE).style(|t: &LauncherTheme| t.style_text(Color::Mid)))
        // WARN: non x86_64
        .push_maybe(cfg!(not(target_arch = "x86_64")).then_some(widget::text(
            "Note: This version is experimental. If you want to get help join our discord",
        )))
        .width(Length::Fill)
        .height(Length::Fill)
}

fn get_footer_text(menu: &'_ MenuLaunch) -> Element<'_> {
    let version_message = widget::column!(
        widget::vertical_space(),
        widget::row!(
            widget::horizontal_space(),
            widget::text!("QuantumLauncher v{LAUNCHER_VERSION_NAME}")
                .size(12)
                .style(|t: &LauncherTheme| t.style_text(Color::Mid))
        ),
        widget::row!(
            widget::horizontal_space(),
            widget::text("A Minecraft Launcher by Mrmayman")
                .size(10)
                .style(|t: &LauncherTheme| t.style_text(Color::Mid))
        ),
    );

    if menu.message.is_empty() {
        widget::column!(version_message)
    } else {
        widget::column!(
            widget::row!(
                widget::horizontal_space(),
                widget::container(widget::text(&menu.message).size(14))
                    .width(190)
                    .padding(10)
            ),
            version_message
        )
    }
    .spacing(10)
    .into()
}
