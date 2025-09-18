use iced::{widget, Length};
use ql_core::LOGGER;

use crate::{
    icon_manager,
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
        let toggler = tooltip(
            widget::button(widget::row![
                widget::horizontal_space(),
                widget::text(if self.is_log_open { "v" } else { "^" }).size(10),
                widget::horizontal_space()
            ])
            .padding(0)
            .height(DEBUG_LOG_BUTTON_HEIGHT)
            .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatDark))
            .on_press(Message::CoreLogToggle),
            widget::text(if self.is_log_open {
                "Close launcher log"
            } else {
                "Open launcher debug log (troubleshooting)"
            })
            .size(12),
            widget::tooltip::Position::Top,
        );

        widget::column![
            widget::column![self.view_menu()].height(
                (self.window_size.1 / if self.is_log_open { 2.0 } else { 1.0 })
                    - DEBUG_LOG_BUTTON_HEIGHT
            ),
            widget::row![toggler].push_maybe(self.is_log_open.then(|| {
                widget::button(widget::text("Copy Log").size(10))
                    .padding(0)
                    .height(DEBUG_LOG_BUTTON_HEIGHT)
                    .style(|n: &LauncherTheme, status| {
                        n.style_button(status, StyleButton::FlatDark)
                    })
                    .on_press(Message::CoreCopyLog)
            })),
        ]
        .push_maybe(self.is_log_open.then(|| {
            const TEXT_SIZE: f32 = 12.0;

            let text = {
                if let Some(logger) = LOGGER.as_ref() {
                    let logger = logger.lock().unwrap();
                    logger.text.clone()
                } else {
                    Vec::new()
                }
            };

            Self::view_launcher_log(
                text,
                TEXT_SIZE,
                self.log_scroll,
                Message::CoreLogScroll,
                Message::CoreLogScrollAbsolute,
                |(msg, kind)| {
                    widget::row![
                        widget::rich_text![widget::span(kind.to_string()).color(match kind {
                            ql_core::LogType::Info => iced::Color::from_rgb8(0xf9, 0xe2, 0xaf),
                            ql_core::LogType::Error => iced::Color::from_rgb8(0xe3, 0x44, 0x59),
                            ql_core::LogType::Point => iced::Color::from_rgb8(128, 128, 128),
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
        }))
        .into()
    }

    fn view_menu(&'_ self) -> Element<'_> {
        match &self.state {
            State::Launch(menu) => self.view_main_menu(menu),
            State::AccountLoginProgress(progress) => widget::column![
                widget::text("Logging into Microsoft account").size(20),
                progress.view()
            ]
            .spacing(10)
            .padding(10)
            .into(),
            State::GenericMessage(msg) => widget::column![widget::text(msg)].padding(10).into(),
            State::AccountLogin => view_account_login(),
            State::EditMods(menu) => menu.view(
                self.selected_instance.as_ref().unwrap(),
                self.tick_timer,
                &self.images,
            ),
            State::Create(menu) => menu.view(self.client_list.as_ref()),
            State::ConfirmAction {
                msg1,
                msg2,
                yes,
                no,
            } => view_confirm(msg1, msg2, yes, no),
            State::Error { error } => view_error(error),
            State::InstallFabric(menu) => {
                menu.view(self.selected_instance.as_ref().unwrap(), self.tick_timer)
            }
            State::InstallJava => widget::column!(widget::text("Downloading Java").size(20),)
                .push_maybe(self.java_recv.as_ref().map(|n| n.view()))
                .padding(10)
                .spacing(10)
                .into(),
            // TODO: maybe remove window_size argument?
            // It's not needed right now, but could be in the future.
            State::ModsDownload(menu) => menu.view(&self.images, self.window_size, self.tick_timer),
            State::LauncherSettings(menu) => menu.view(&self.config, self.window_size),
            State::InstallPaper => {
                let dots = ".".repeat((self.tick_timer % 3) + 1);
                widget::column!(widget::text!("Installing Paper{dots}").size(20))
                    .padding(10)
                    .spacing(10)
                    .into()
            }
            State::ChangeLog => {
                let back_msg = Message::LaunchScreenOpen {
                    message: None,
                    clear_selection: true,
                };
                widget::scrollable(
                    widget::column!(
                        button_with_icon(icon_manager::back(), "Skip", 16)
                            .on_press(back_msg.clone()),
                        changelog(),
                        button_with_icon(icon_manager::back(), "Continue", 16).on_press(back_msg),
                    )
                    .padding(10)
                    .spacing(10),
                )
                .style(LauncherTheme::style_scrollable_flat_extra_dark)
                .height(Length::Fill)
                .into()
            }
            State::Welcome(menu) => menu.view(&self.config),
            State::EditJarMods(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::ImportModpack(progress) => {
                widget::column![widget::text("Installing mods..."), progress.view()]
                    .padding(10)
                    .spacing(10)
                    .into()
            }
            State::LogUploadResult { url } => {
                view_log_upload_result(url, self.selected_instance.as_ref().unwrap().is_server())
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
            State::ServerCreate(menu) => menu.view(),
            State::ManagePresets(menu) => menu.view(),
            State::RecommendedMods(menu) => menu.view(),
        }
    }
}
