use iced::{widget, Alignment, Length};

use crate::{
    icon_manager,
    state::{AccountMessage, MenuLoginAlternate, MenuLoginMS, Message, NEW_ACCOUNT_NAME},
};

use super::{back_button, button_with_icon, center_x, Element};

impl MenuLoginAlternate {
    pub fn view(&'_ self, tick_timer: usize) -> Element<'_> {
        let status: Element =
            if self.is_loading {
                let dots = ".".repeat((tick_timer % 3) + 1);
                widget::column![widget::text!("Loading{dots}")]
                    .padding(8)
                    .into()
            } else {
                widget::column![button_with_icon(icon_manager::tick(), "Login", 16)
                    .on_press(Message::Account(AccountMessage::AltLogin))]
                .align_x(Alignment::Center)
                .push_maybe(self.is_littleskin.then_some(
                    widget::button("Login with OAuth").on_press(Message::Account(
                        AccountMessage::LittleSkinOauthButtonClicked,
                    )),
                ))
                .spacing(5)
                .into()
            };

        let padding = iced::Padding {
            top: 5.0,
            bottom: 5.0,
            right: 10.0,
            left: 10.0,
        };

        let password_input = widget::text_input("Enter Password...", &self.password)
            .padding(padding)
            .on_input(|n| Message::Account(AccountMessage::AltPasswordInput(n)));
        let password_input = if self.password.is_empty() || self.show_password {
            password_input
        } else {
            password_input.font(iced::Font::with_name("Password Asterisks"))
        };

        if let Some(oauth) = &self.oauth {
            self.view_oauth(oauth)
        } else {
            widget::column![
                back_button().on_press(if self.is_from_welcome_screen {
                    Message::WelcomeContinueToAuth
                } else {
                    Message::Account(AccountMessage::Selected(NEW_ACCOUNT_NAME.to_owned()))
                }),
                widget::column![
                    widget::text(if self.is_littleskin {
                        "LittleSkin Login"
                    } else {
                        "ElyBy Login"
                    })
                    .size(20),
                    widget::vertical_space(),
                    widget::text("Username/Email:").size(12),
                    center_x(
                        widget::text_input("Enter Username/Email...", &self.username)
                            .padding(padding)
                            .on_input(|n| Message::Account(AccountMessage::AltUsernameInput(n)))
                    ),
                    widget::text("Password:").size(12),
                    center_x(password_input),
                    widget::checkbox("Show Password", self.show_password)
                        .size(14)
                        .text_size(14)
                        .on_toggle(|t| Message::Account(AccountMessage::AltShowPassword(t))),
                    widget::Column::new().push_maybe(self.otp.as_deref().map(|otp| {
                        widget::column![
                            widget::text("OTP:").size(12),
                            widget::text_input("Enter Username/Email...", otp)
                                .padding(padding)
                                .on_input(|n| Message::Account(AccountMessage::AltOtpInput(n))),
                        ]
                        .spacing(5)
                    })),
                    status,
                    widget::Space::with_height(5),
                    widget::row![
                        widget::text("Or").size(14),
                        widget::button(widget::text("Create an account").size(14)).on_press(
                            Message::CoreOpenLink(
                                if self.is_littleskin {
                                    "https://littleskin.cn/auth/register"
                                } else {
                                    "https://account.ely.by/register"
                                }
                                .to_owned()
                            )
                        )
                    ]
                    .align_y(Alignment::Center)
                    .spacing(5)
                    .wrap(),
                    widget::vertical_space(),
                ]
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .spacing(5)
            ]
            .padding(10)
            .into()
        }
    }

    fn view_oauth(&'_ self, oauth: &crate::state::LittleSkinOauth) -> Element<'_> {
        let time_left = {
            let now = std::time::Instant::now();
            if oauth.device_code_expires_at > now {
                (oauth.device_code_expires_at - now).as_secs()
            } else {
                0
            }
        };

        let code_row = widget::row![
            widget::text!("Code: {}", oauth.user_code).size(16),
            widget::button(widget::text("Copy").size(13))
                .on_press(Message::CoreCopyText(oauth.user_code.clone())),
        ]
        .align_y(Alignment::Center)
        .spacing(10);
        let url_row = widget::row![
            widget::text!("Link: {}", oauth.verification_uri).size(16),
            widget::button(widget::text("Open").size(13))
                .on_press(Message::CoreOpenLink(oauth.verification_uri.clone())),
        ]
        .align_y(Alignment::Center)
        .spacing(10);
        widget::column![
            widget::vertical_space(),
            widget::text("LittleSkin Device Login").size(20),
            widget::text("Open this link and enter the code:").size(14),
            widget::Space::with_height(5),
            widget::container(widget::column![code_row, url_row]).padding(10),
            widget::Space::with_height(5),
            widget::text!("Expires in: {}s", time_left).size(13),
            widget::vertical_space(),
            widget::text("Waiting for login...").size(14),
            widget::vertical_space(),
        ]
        .width(Length::Fill)
        .push_maybe(
            self.device_code_error
                .as_ref()
                .map(|err| widget::text(err).size(14)),
        )
        .spacing(5)
        .align_x(Alignment::Center)
        .into()
    }
}

impl MenuLoginMS {
    pub fn view<'a>(&self) -> Element<'a> {
        widget::column![
            back_button().on_press(if self.is_from_welcome_screen {
                Message::WelcomeContinueToAuth
            } else {
                Message::Account(AccountMessage::Selected(NEW_ACCOUNT_NAME.to_owned()))
            }),
            widget::row!(
                widget::horizontal_space(),
                widget::column!(
                    widget::vertical_space(),
                    widget::text("Login to Microsoft").size(20),
                    "Open this link and enter the code:",
                    widget::text!("Code: {}", self.code),
                    widget::button("Copy").on_press(Message::CoreCopyText(self.code.clone())),
                    widget::text!("Link: {}", self.url),
                    widget::button("Open").on_press(Message::CoreOpenLink(self.url.clone())),
                    widget::vertical_space(),
                )
                .spacing(5)
                .align_x(Alignment::Center),
                widget::horizontal_space()
            )
        ]
        .padding(10)
        .into()
    }
}
