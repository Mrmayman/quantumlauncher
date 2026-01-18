use iced::{
    widget::{self, column},
    Length,
};

use crate::{
    icons,
    menu_renderer::{
        button_with_icon, changelog, tooltip, view_account_login, view_confirm, view_error,
        view_log_upload_result, Element, FONT_MONO,
    },
    state::{Launcher, Message, State},
    stylesheet::{styles::LauncherTheme, widgets::StyleButton},
    DEBUG_LOG_BUTTON_HEIGHT,
};

impl Launcher {
    pub fn view(&'_ self) -> Element<'_> {
        let round = !self.config.uses_system_decorations();
        let toggler = tooltip(
            widget::button(widget::row![
                widget::space().width(Length::Fill),
                widget::text(if self.is_log_open { "v" } else { "^" }).size(10),
                widget::space().width(Length::Fill)
            ])
            .padding(0)
            .height(DEBUG_LOG_BUTTON_HEIGHT)
            .style(move |n: &LauncherTheme, status| {
                let round = round && !self.is_log_open;
                n.style_button(
                    status,
                    StyleButton::SemiExtraDark([false, false, round, round]),
                )
            })
            .on_press(Message::CoreLogToggle),
            widget::text(if self.is_log_open {
                "Close launcher log"
            } else {
                "Open launcher debug log (troubleshooting)"
            })
            .size(12),
            widget::tooltip::Position::Top,
        );

        let view = column![
            column![self.view_menu()],
            widget::row![
                toggler,
                self.is_log_open.then(|| {
                    widget::button(widget::text("Copy Log").size(10))
                        .padding(0)
                        .height(DEBUG_LOG_BUTTON_HEIGHT)
                        .style(|n: &LauncherTheme, status| {
                            n.style_button(status, StyleButton::FlatDark)
                        })
                        .on_press(Message::CoreCopyLog)
                }),
                self.is_log_open.then(|| {
                    const TEXT_SIZE: f32 = 12.0;

                    Self::view_launcher_log(
                        ql_core::print::get(),
                        TEXT_SIZE,
                        self.log_scroll,
                        Message::CoreLogScroll,
                        Message::CoreLogScrollAbsolute,
                        |(msg, kind)| {
                            widget::row![
                                widget::rich_text![widget::span::<Message, _>(kind.to_string())
                                    .color(match kind {
                                        ql_core::LogType::Info =>
                                            iced::Color::from_rgb8(0xf9, 0xe2, 0xaf),
                                        ql_core::LogType::Error =>
                                            iced::Color::from_rgb8(0xe3, 0x44, 0x59),
                                        ql_core::LogType::Point =>
                                            iced::Color::from_rgb8(128, 128, 128),
                                    })]
                                .size(12)
                                .font(FONT_MONO),
                                widget::text!(" {msg}").font(FONT_MONO).size(12)
                            ]
                            .width(Length::Fill)
                            .into()
                        },
                        |(msg, kind)| format!("{kind} {msg}"),
                    )
                })
            ],
        ];

        // if self.window_state.is_maximized || self.config.c_window_decorations() {
        view.into()
        // } else {
        // setup_window_borders(view.into())
        // }
    }

    fn view_menu(&'_ self) -> Element<'_> {
        let menu = match &self.state {
            State::Launch(menu) => self.view_main_menu(menu),
            State::AccountLoginProgress(progress) => column![
                widget::text("Logging into Microsoft account").size(20),
                progress.view()
            ]
            .spacing(10)
            .padding(10)
            .into(),
            State::GenericMessage(msg) => column![widget::text(msg)].padding(10).into(),
            State::AccountLogin => view_account_login(),
            State::EditMods(menu) => menu.view(
                self.instance(),
                self.tick_timer,
                &self.images,
                self.window_state.size.1,
            ),
            State::Create(menu) => menu.view(self.client_list.as_deref(), self.tick_timer),
            State::ConfirmAction {
                msg1,
                msg2,
                yes,
                no,
            } => view_confirm(msg1, msg2, yes, no),
            State::Error { error } => view_error(error),
            State::InstallFabric(menu) => menu.view(self.instance(), self.tick_timer),
            State::InstallJava => column![
                widget::text("Downloading Java").size(20),
                self.java_recv.as_ref().map(|n| n.view())
            ]
            .padding(10)
            .spacing(10)
            .into(),
            State::ModsDownload(menu) => menu.view(&self.images, self.tick_timer),
            State::LauncherSettings(menu) => menu.view(&self.config),
            State::InstallPaper(menu) => menu.view(self.tick_timer),
            State::ChangeLog => {
                let back_msg = Message::MScreenOpen {
                    message: None,
                    clear_selection: true,
                    is_server: None,
                };
                widget::scrollable(
                    column!(
                        button_with_icon(icons::back(), "Skip", 16).on_press(back_msg.clone()),
                        changelog(&self.config),
                        button_with_icon(icons::back(), "Continue", 16).on_press(back_msg),
                    )
                    .padding(10)
                    .spacing(10),
                )
                .style(LauncherTheme::style_scrollable_flat_extra_dark)
                .height(Length::Fill)
                .into()
            }
            State::Welcome(menu) => menu.view(&self.config),
            State::EditJarMods(menu) => menu.view(self.instance()),
            State::ImportModpack(progress) => {
                column![widget::text("Installing mods..."), progress.view()]
                    .padding(10)
                    .spacing(10)
                    .into()
            }
            State::LogUploadResult { url } => {
                view_log_upload_result(url, self.instance().is_server())
            }

            State::LoginAlternate(menu) => menu.view(self.tick_timer),
            State::ExportInstance(menu) => menu.view(self.tick_timer),

            State::LoginMS(menu) => menu.view(),
            State::CurseforgeManualDownload(menu) => menu.view(),
            State::License(menu) => menu.view(),
            State::ExportMods(menu) => menu.view(),
            State::InstallForge(menu) => menu.view(),
            State::UpdateFound(menu) => menu.view(),
            State::InstallOptifine(menu) => menu.view(),
            State::ManagePresets(menu) => menu.view(),
            State::RecommendedMods(menu) => menu.view(),
        };

        if let State::Launch(_) = &self.state {
            menu
        } else {
            // let round = !self.config.uses_system_decorations();
            widget::Column::new()
                /*.push_maybe({
                    let maximized = self.window_state.is_maximized;
                    let custom_decor = widget::mouse_area(
                        widget::container(self.view_window_decorations())
                            .height(32)
                            .width(Length::Fill)
                            .style(move |t: &LauncherTheme| {
                                t.style_container_bg_semiround(
                                    [!maximized, !maximized, false, false],
                                    Some((Color::ExtraDark, 1.0)),
                                )
                            }),
                    )
                    .on_press(Message::Window(WindowMessage::Dragged));
                    round.then_some(custom_decor)
                })*/
                .push(
                    widget::container(menu)
                        .style(move |t: &LauncherTheme| t.style_container_bg(0.0, None))
                        .width(Length::Fill)
                        .height(Length::Fill),
                )
                .into()
        }
    }

    /*pub fn view_window_decorations(&self) -> widget::Row<'_, Message, LauncherTheme> {
        const ICON_SIZE: u32 = 10;

        fn win_button(icon: widget::Text<'_, LauncherTheme>, m: Message) -> Element<'_> {
            widget::mouse_area(
                widget::row![widget::button(
                    widget::row![icon.style(|t: &LauncherTheme| t.style_text(Color::Mid))]
                        .align_y(Alignment::Center)
                        .padding([4, 10]),
                )
                .padding(0)
                .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::RoundDark))
                .on_press(m.clone())]
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .padding([3.0, 1.5]),
            )
            .on_release(m)
            .into()
        }

        let right = matches!(
            self.config
                .ui
                .as_ref()
                .map(|n| n.window_decorations)
                .unwrap_or_default(),
            UiWindowDecorations::Right
        );

        let wcls_space = widget::mouse_area(column![].height(Length::Fill).width(6.5))
            .on_press(Message::Window(WindowMessage::ClickClose));
        let wcls = win_button(
            icons::close_s(ICON_SIZE),
            Message::Window(WindowMessage::ClickClose),
        );
        let wmax = win_button(
            icons::maximize_s(ICON_SIZE),
            Message::Window(WindowMessage::ClickMaximize),
        );
        let wmin = win_button(
            icons::minimize_s(ICON_SIZE),
            Message::Window(WindowMessage::ClickMinimize),
        );
        if right {
            widget::Row::new()
                .push(widget::space().width(Length::Fill))
                .push(wmin)
                .push(wmax)
                .push(wcls)
                .push(wcls_space)
        } else {
            widget::Row::new()
                .push(wcls_space)
                .push(wcls)
                .push(wmax)
                .push(wmin)
        }
    }*/
}

// HOOK: Decorations
/*fn setup_window_borders(view: Element<'_>) -> Element<'_> {
    fn m(
        (w, h): (impl Into<Length>, impl Into<Length>),
        i: Interaction,
        d: Direction,
    ) -> widget::MouseArea<'static, Message, LauncherTheme> {
        widget::mouse_area(column![].width(w).height(h))
            .interaction(i)
            .on_press(Message::Window(WindowMessage::Resized(d)))
    }

    widget::stack!(
        column![widget::container(view).padding(1)].padding(2),
        widget::row![
            // Left
            widget::Column::new()
                .push(m(
                    (10, 10),
                    Interaction::ResizingDiagonallyDown,
                    Direction::NorthWest
                ))
                .push(m(
                    (10, Length::Fill),
                    Interaction::ResizingHorizontally,
                    Direction::West
                ))
                .push(m(
                    (10, 10),
                    Interaction::ResizingDiagonallyUp,
                    Direction::SouthWest
                )),
            column![
                m(
                    (Length::Fill, 10),
                    Interaction::ResizingVertically,
                    Direction::North
                ),
                widget::space().height(Length::Fill),
                m(
                    (Length::Fill, 10),
                    Interaction::ResizingVertically,
                    Direction::South
                )
            ],
            widget::Column::new()
                .push(m(
                    (10, 10),
                    Interaction::ResizingDiagonallyUp,
                    Direction::NorthEast
                ))
                .push(m(
                    (10, Length::Fill),
                    Interaction::ResizingHorizontally,
                    Direction::East
                ))
                .push(m(
                    (10, 10),
                    Interaction::ResizingDiagonallyDown,
                    Direction::SouthEast
                )),
        ]
    )
    .into()
}*/
