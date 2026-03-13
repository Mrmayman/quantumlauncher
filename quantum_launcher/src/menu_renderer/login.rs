use iced::{widget, Alignment, Length};

use crate::{
    icons,
    menu_renderer::tsubtitle,
    state::{
        AccountMessage, MenuLoginAlternate, MenuLoginMS, MenuTokenPassword, Message,
        TokenPasswordMessage, NEW_ACCOUNT_NAME,
    },
    stylesheet::styles::LauncherTheme,
};

use super::{back_button, button_with_icon, center_x, Element};

impl MenuTokenPassword {
    pub fn view(&self, tick_timer: usize) -> Element<'_> {
        let input_padding = iced::Padding {
            top: 8.0,
            bottom: 8.0,
            right: 12.0,
            left: 12.0,
        };

        let is_create = self.confirm_password.is_some();

        let title = if is_create {
            "Create Encrypted Token Store"
        } else {
            "Unlock Encrypted Token Store"
        };

        let description = if is_create {
            "Set a password to protect your account tokens.\nThis encrypted file can be copied to other machines."
        } else {
            "Enter your password to unlock the encrypted account store."
        };

        let submit_on_enter: Message = TokenPasswordMessage::Submit.into();

        let password_input = widget::text_input("Password...", &self.password)
            .padding(input_padding)
            .width(320)
            .on_input(|n| TokenPasswordMessage::PasswordChanged(n).into())
            .on_submit(submit_on_enter);
        let password_input = if self.password.is_empty() || self.show_password {
            password_input
        } else {
            password_input.font(iced::Font::with_name("Password Asterisks"))
        };

        let show_toggle = widget::checkbox("Show Password", self.show_password)
            .size(12)
            .text_size(12)
            .on_toggle(|t| TokenPasswordMessage::ToggleShowPassword(t).into());

        let is_loading = self.is_loading;
        let submit_label = if is_create { "Create" } else { "Unlock" };
        let submit_btn: Element = if is_loading {
            let dots = ".".repeat((tick_timer % 3) + 1);
            widget::button(
                widget::row![icons::gear(), widget::text!(" Loading{dots}").size(14)]
                    .align_y(Alignment::Center),
            )
            .padding([7, 16])
            .into()
        } else {
            widget::button(
                widget::row![icons::checkmark(), widget::text(submit_label).size(14)]
                    .align_y(Alignment::Center)
                    .spacing(6),
            )
            .on_press(TokenPasswordMessage::Submit.into())
            .padding([7, 16])
            .into()
        };

        let skip_btn: Element = widget::button(
            widget::row![icons::close(), widget::text("Skip").size(14)]
                .align_y(Alignment::Center)
                .spacing(6),
        )
        .on_press(TokenPasswordMessage::Skip.into())
        .padding([7, 16])
        .into();

        // Card
        let mut card = widget::column![
            widget::text(title).size(22),
            widget::horizontal_rule(1),
            widget::text(description).size(12).style(tsubtitle),
            widget::Space::with_height(4),
            widget::row![
                widget::text("Password:").size(13),
                widget::horizontal_space(),
                show_toggle,
            ]
            .align_y(Alignment::Center),
            password_input,
        ]
        .spacing(8)
        .width(340);

        if let Some(confirm) = &self.confirm_password {
            let confirm_input = widget::text_input("Confirm Password...", confirm)
                .padding(input_padding)
                .width(320)
                .on_input(|n| TokenPasswordMessage::ConfirmPasswordChanged(n).into())
                .on_submit(TokenPasswordMessage::Submit.into());
            let confirm_input = if confirm.is_empty() || self.show_password {
                confirm_input
            } else {
                confirm_input.font(iced::Font::with_name("Password Asterisks"))
            };
            card = card
                .push(widget::text("Confirm Password:").size(13))
                .push(confirm_input);
        }

        if let Some(err) = &self.error {
            card = card.push(
                widget::text(err.as_str())
                    .size(12)
                    .style(|t: &_| tsubtitle(t)),
            );
        }

        card = card.push(widget::Space::with_height(4)).push(
            widget::row![submit_btn, skip_btn]
                .spacing(10)
                .align_y(Alignment::Center),
        );

        let card_container = widget::container(card.padding(24)).style(|t: &LauncherTheme| {
            t.style_container_round_box(
                crate::stylesheet::styles::BORDER_WIDTH,
                crate::stylesheet::color::Color::Dark,
                crate::stylesheet::styles::BORDER_RADIUS,
            )
        });

        widget::column![
            widget::vertical_space(),
            widget::row![
                widget::horizontal_space(),
                card_container,
                widget::horizontal_space(),
            ],
            widget::vertical_space(),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

impl MenuLoginAlternate {
    pub fn view(&'_ self, tick_timer: usize) -> Element<'_> {
        if let Some(oauth) = &self.oauth {
            return self.view_oauth(oauth);
        }

        let status: Element =
            if self.is_loading {
                let dots = ".".repeat((tick_timer % 3) + 1);
                widget::column![widget::text!("Loading{dots}")]
                    .padding(8)
                    .into()
            } else {
                widget::column![button_with_icon(icons::checkmark(), "Login", 16)
                    .on_press(AccountMessage::AltLogin.into())]
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
            .on_input(|n| AccountMessage::AltPasswordInput(n).into());
        let password_input = if self.password.is_empty() || self.show_password {
            password_input
        } else {
            password_input.font(iced::Font::with_name("Password Asterisks"))
        };

        widget::column![
            back_button().on_press(if self.is_from_welcome_screen {
                Message::WelcomeContinueToAuth
            } else {
                AccountMessage::Selected(NEW_ACCOUNT_NAME.to_owned()).into()
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
                        .on_input(|n| AccountMessage::AltUsernameInput(n).into())
                ),
                widget::row![
                    widget::text("Password:").size(12),
                    widget::checkbox("Show", self.show_password)
                        .size(12)
                        .text_size(12)
                        .on_toggle(|t| AccountMessage::AltShowPassword(t).into()),
                ]
                .spacing(16),
                center_x(password_input),
                widget::Column::new()
                    .push_maybe(self.otp.as_deref().map(|otp| {
                        widget::column![
                            widget::text("OTP:").size(12),
                            widget::text_input("Enter Username/Email...", otp)
                                .padding(padding)
                                .on_input(|n| AccountMessage::AltOtpInput(n).into()),
                        ]
                        .spacing(5)
                    }))
                    .push_maybe(
                        self.is_incorrect_password
                            .then_some(widget::text("Wrong Password!").style(tsubtitle).size(12))
                    ),
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
                AccountMessage::Selected(NEW_ACCOUNT_NAME.to_owned()).into()
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
