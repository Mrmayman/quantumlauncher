use iced::widget::{column, row, tooltip::Position};
use iced::{widget, Alignment, Length};
use ql_core::{Progress, WEBSITE};
use ql_instances::auth::AccountType;

use crate::stylesheet::styles::{LauncherThemeLightness, BORDER_RADIUS, BORDER_WIDTH};
use crate::{
    config::LauncherConfig,
    icons,
    state::{
        AccountMessage, InstallModsMessage, LauncherSettingsMessage, LicenseTab, ManageModsMessage,
        MenuCurseforgeManualDownload, MenuLauncherUpdate, MenuLicense, Message, ProgressBar,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

mod create;
mod edit_instance;
mod launch;
mod log;
mod login;
mod mods;
mod onboarding;
mod settings;

pub use onboarding::changelog;

pub const DISCORD: &str = "https://discord.gg/bWqRaSXar5";
pub const GITHUB: &str = "https://github.com/Mrmayman/quantumlauncher";

pub const FONT_MONO: iced::Font = iced::Font::with_name("JetBrains Mono");
pub const FONT_DEFAULT: iced::Font = iced::Font::with_name("Inter");

pub type Element<'a> = iced::Element<'a, Message, LauncherTheme>;

const PADDING_NOT_BOTTOM: iced::Padding = iced::Padding {
    top: 10.0,
    bottom: 0.0,
    left: 10.0,
    right: 10.0,
};

fn ctx_button(e: &'_ str) -> widget::Button<'_, Message, LauncherTheme> {
    widget::button(widget::text(e).size(13))
        .width(Length::Fill)
        .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::FlatDark))
        .padding(2)
}

pub fn checkered_list<'a, Item: Into<Element<'a>>>(
    children: impl IntoIterator<Item = Item>,
) -> widget::Column<'a, Message, LauncherTheme> {
    widget::column(children.into_iter().enumerate().map(|(i, e)| {
        widget::container(e)
            .width(Length::Fill)
            .padding(16)
            .style(move |t: &LauncherTheme| {
                t.style_container_sharp_box(
                    0.0,
                    if i % 2 == 0 {
                        Color::Dark
                    } else {
                        Color::ExtraDark
                    },
                )
            })
            .into()
    }))
}

pub fn select_box<'a>(
    e: impl Into<Element<'a>>,
    is_checked: bool,
    message: Message,
) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(underline(e, Color::Dark))
        .on_press(message)
        .style(move |t: &LauncherTheme, s| {
            t.style_button(
                s,
                if is_checked {
                    StyleButton::Flat
                } else {
                    StyleButton::FlatExtraDark
                },
            )
        })
}

pub fn link<'a>(
    e: impl Into<Element<'a>>,
    url: String,
) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(underline(e, Color::Light))
        .on_press(Message::CoreOpenLink(url))
        .padding(0)
        .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatDark))
}

pub fn underline<'a>(
    e: impl Into<Element<'a>>,
    color: Color,
) -> widget::Stack<'a, Message, LauncherTheme> {
    widget::stack!(
        column![e.into()],
        column![
            widget::vertical_space(),
            widget::horizontal_rule(1).style(move |t: &LauncherTheme| t.style_rule(color, 1)),
            widget::Space::with_height(1),
        ]
    )
}

pub fn underline_maybe<'a>(e: impl Into<Element<'a>>, color: Color, un: bool) -> Element<'a> {
    if un {
        underline(e, color).into()
    } else {
        e.into()
    }
}

pub fn center_x<'a>(e: impl Into<Element<'a>>) -> widget::Row<'a, Message, LauncherTheme> {
    row![
        widget::horizontal_space(),
        e.into(),
        widget::horizontal_space(),
    ]
}

pub fn tooltip<'a>(
    e: impl Into<Element<'a>>,
    tooltip: impl Into<Element<'a>>,
    position: Position,
) -> widget::Tooltip<'a, Message, LauncherTheme> {
    widget::tooltip(e, tooltip, position)
        .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark))
}

pub fn back_button<'a>() -> widget::Button<'a, Message, LauncherTheme> {
    button_with_icon(icons::back_s(14), "Back", 14)
}

pub fn ctxbox<'a>(inner: impl Into<Element<'a>>) -> widget::Container<'a, Message, LauncherTheme> {
    widget::container(widget::mouse_area(inner).on_press(Message::Nothing))
        .padding(10)
        .style(|t: &LauncherTheme| {
            t.style_container_round_box(BORDER_WIDTH, Color::Dark, BORDER_RADIUS)
        })
}

pub fn subbutton_with_icon<'a>(
    icon: impl Into<Element<'a>>,
    text: &'a str,
) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(
        row![icon.into()]
            .push_maybe((!text.is_empty()).then_some(widget::text(text).size(12)))
            .align_y(Alignment::Center)
            .spacing(8)
            .padding(1),
    )
    .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::RoundDark))
}

pub fn button_with_icon<'a>(
    icon: impl Into<Element<'a>>,
    text: &'a str,
    size: u16,
) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(
        row![icon.into()]
            .push_maybe((!text.is_empty()).then_some(widget::text(text).size(size)))
            .align_y(Alignment::Center)
            .spacing(size as f32 / 1.6),
    )
    .padding([7, 13])
}

#[allow(unreachable_code)]
pub fn shortcut_ctrl<'a>(key: &str) -> Element<'a> {
    #[cfg(target_os = "macos")]
    return widget::text!("Command + {key}").size(12).into();

    widget::text!("Control + {key}").size(12).into()
}

fn sidebar_button<'a, A: PartialEq>(
    current: &A,
    selected: &A,
    text: impl Into<Element<'a>>,
    message: Message,
) -> Element<'a> {
    let is_selected = current == selected;
    let button = widget::button(text)
        .on_press_maybe((!is_selected).then_some(message))
        .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatExtraDark))
        .width(Length::Fill);

    underline_maybe(button, Color::SecondDark, !is_selected)
}

fn tsubtitle(t: &LauncherTheme) -> widget::text::Style {
    t.style_text(Color::SecondLight)
}

fn sidebar<'a>(
    id: &'static str,
    header: Option<Element<'a>>,
    children: impl IntoIterator<Item = Element<'a>>,
) -> widget::Container<'a, Message, LauncherTheme> {
    widget::container(
        column![
            widget::Column::new()
                .push_maybe(header)
                .padding(PADDING_NOT_BOTTOM),
            widget::scrollable(widget::column(children))
                .style(LauncherTheme::style_scrollable_flat_extra_dark)
                .height(Length::Fill)
                .id(iced::widget::scrollable::Id::new(id))
        ]
        .spacing(10),
    )
    .width(190)
    .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark))
}

fn dots(tick_timer: usize) -> String {
    ".".repeat((tick_timer % 3) + 1)
}

impl MenuLauncherUpdate {
    pub fn view(&'_ self) -> Element<'_> {
        if let Some(progress) = &self.progress {
            return column!["Updating QuantumLauncher...", progress.view()]
                .padding(10)
                .into();
        }
        column![
            "A new launcher update has been found! Do you want to download it?",
            widget::Row::new()
            .push_maybe((!cfg!(target_os = "macos")).then_some(
                button_with_icon(icons::download(), "Download", 16)
                    .on_press(Message::UpdateDownloadStart))
            )
            .push(back_button().on_press(
                Message::MScreenOpen {
                    message: None,
                    clear_selection: false,
                    is_server: None
                }
            ))
            .push(button_with_icon(icons::globe(), "Open Website", 16)
                .on_press(Message::CoreOpenLink(WEBSITE.to_owned())))
            .spacing(5).wrap(),
        ]
        // WARN: Auto update configurations
        .push_maybe(cfg!(target_os = "linux").then_some(
            column![
                "If you installed this launcher from a package manager/store (flatpak/apt/dnf/pacman/..) then update from there",
                "If you downloaded it from website then it's fine."
            ]
        ))
        .padding(10)
        .spacing(10)
        .into()
    }
}

pub fn get_mode_selector(config: &LauncherConfig) -> Element<'static> {
    const PADDING: iced::Padding = iced::Padding {
        top: 5.0,
        bottom: 5.0,
        right: 10.0,
        left: 10.0,
    };

    let td = |t: &LauncherTheme| t.style_text(Color::Mid);

    let theme = config.ui_mode.unwrap_or_default();
    widget::row(LauncherThemeLightness::ALL.iter().map(|n| {
        let name = widget::text(n.to_string()).size(14);
        let icon = match n {
            LauncherThemeLightness::Light => icons::mode_light_s(14),
            LauncherThemeLightness::Dark => icons::mode_dark_s(14),
            LauncherThemeLightness::Auto => icons::refresh_s(14),
        };

        if *n == theme {
            widget::container(row![icon.style(td), name].spacing(5))
                .padding(PADDING)
                .into()
        } else {
            widget::button(row![icon, name].spacing(5))
                .on_press(Message::LauncherSettings(
                    LauncherSettingsMessage::ThemePicked(*n),
                ))
                .into()
        }
    }))
    .spacing(5)
    .wrap()
    .into()
}

fn back_to_launch_screen(is_server: Option<bool>, message: Option<String>) -> Message {
    Message::MScreenOpen {
        message,
        clear_selection: false,
        is_server,
    }
}

impl<T: Progress> ProgressBar<T> {
    pub fn view(&'_ self) -> widget::Column<'_, Message, LauncherTheme> {
        let total = T::total();
        if let Some(message) = &self.message {
            column![
                widget::progress_bar(0.0..=total, self.num),
                widget::text(message)
            ]
        } else {
            column![widget::progress_bar(0.0..=total, self.num)]
        }
        .spacing(10)
    }
}

impl MenuCurseforgeManualDownload {
    pub fn view(&'_ self) -> Element<'_> {
        column![
            "Some Curseforge mods have blocked this launcher!\nYou need to manually download the files and add them to your mods",

            widget::scrollable(
                widget::column(self.unsupported.iter().map(|entry| {
                    let url = format!(
                        "https://www.curseforge.com/minecraft/{}/{}/download/{}",
                        entry.project_type,
                        entry.slug,
                        entry.file_id
                    );

                    row![
                        widget::button(widget::text("Open link").size(14)).on_press(Message::CoreOpenLink(url)),
                        widget::text(&entry.name)
                            .shaping(widget::text::Shaping::Advanced)
                    ]
                    .align_y(Alignment::Center)
                    .spacing(10)
                    .into()
                }))
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(LauncherTheme::style_scrollable_flat_extra_dark),

            "Warning: Ignoring this may lead to crashes!",
            row![
                widget::button(widget::text("+ Select above downloaded files").size(14)).on_press(Message::ManageMods(ManageModsMessage::AddFile(self.delete_mods))),
                widget::button(widget::text("Continue").size(14)).on_press(if self.is_store {
                    Message::InstallMods(InstallModsMessage::Open)
                } else {
                    Message::ManageMods(ManageModsMessage::ScreenOpenWithoutUpdate)
                }),
                widget::checkbox("Delete files when done", self.delete_mods)
                    .text_size(14)
                    .on_toggle(|t|
                        Message::ManageMods(ManageModsMessage::CurseforgeManualToggleDelete(t))
                    )
            ].spacing(5).align_y(Alignment::Center).wrap()
        ]
            .padding(10)
            .spacing(10)
            .into()
    }
}

impl MenuLicense {
    pub fn view(&'_ self) -> Element<'_> {
        row![
            sidebar(
                "MenuLicense:sidebar",
                Some(
                    back_button()
                        .on_press(Message::LauncherSettings(
                            LauncherSettingsMessage::ChangeTab(
                                crate::state::LauncherSettingsTab::About
                            ),
                        ))
                        .into()
                ),
                LicenseTab::ALL.iter().map(|tab| {
                    let text = widget::text(tab.to_string());
                    sidebar_button(
                        tab,
                        &self.selected_tab,
                        text,
                        Message::LicenseChangeTab(*tab),
                    )
                }),
            ),
            widget::scrollable(
                widget::text_editor(&self.content)
                    .on_action(Message::LicenseAction)
                    .style(LauncherTheme::style_text_editor_flat_extra_dark)
            )
            .style(LauncherTheme::style_scrollable_flat_dark)
        ]
        .into()
    }
}

pub fn view_account_login<'a>() -> Element<'a> {
    column![
        back_button().on_press(back_to_launch_screen(None, None)),
        widget::vertical_space(),
        row![
            widget::horizontal_space(),
            column![
                widget::text("Login").size(20),
                widget::button("Login with Microsoft").on_press(Message::Account(
                    AccountMessage::OpenMenu {
                        is_from_welcome_screen: false,
                        kind: AccountType::Microsoft
                    }
                )),
                widget::button("Login with ely.by").on_press(Message::Account(
                    AccountMessage::OpenMenu {
                        is_from_welcome_screen: false,
                        kind: AccountType::ElyBy
                    }
                )),
                widget::button("Login with littleskin").on_press(Message::Account(
                    AccountMessage::OpenMenu {
                        is_from_welcome_screen: false,
                        kind: AccountType::LittleSkin
                    }
                )),
            ]
            .align_x(Alignment::Center)
            .spacing(5),
            widget::horizontal_space(),
        ],
        widget::vertical_space(),
    ]
    .padding(10)
    .spacing(5)
    .into()
}

pub fn view_error(error: &'_ str) -> Element<'_> {
    widget::scrollable(
        column![
            widget::text!("Error: {error}"),
            row![
                widget::button("Back").on_press(back_to_launch_screen(None, None)),
                widget::button("Copy Error").on_press(Message::CoreCopyError),
                widget::button("Copy Error + Log").on_press(Message::CoreCopyLog),
                widget::button("Join Discord for help")
                    .on_press(Message::CoreOpenLink(DISCORD.to_owned()))
            ]
            .spacing(5)
            .wrap()
        ]
        .padding(10)
        .spacing(10),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(LauncherTheme::style_scrollable_flat_extra_dark)
    .into()
}

pub fn view_log_upload_result(url: &'_ str, is_server: bool) -> Element<'_> {
    column![
        back_button().on_press(back_to_launch_screen(Some(is_server), None)),
        column![
            widget::vertical_space(),
            widget::text(format!(
                "{} log uploaded successfully!",
                if is_server { "Server" } else { "Game" }
            ))
            .size(20),
            widget::text("Your log has been uploaded to mclo.gs. You can share the link below:")
                .size(14),
            widget::container(
                row![
                    widget::text(url).font(FONT_MONO),
                    widget::button("Copy").on_press(Message::CoreCopyText(url.to_string())),
                    widget::button("Open").on_press(Message::CoreOpenLink(url.to_string()))
                ]
                .spacing(10)
                .align_y(Alignment::Center)
            )
            .padding(10),
            widget::vertical_space(),
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .spacing(10)
    ]
    .padding(10)
    .into()
}

pub fn view_confirm<'a>(
    msg1: &'a str,
    msg2: &'a str,
    yes: &'a Message,
    no: &'a Message,
) -> Element<'a> {
    let t_white = |_: &LauncherTheme| widget::text::Style {
        color: Some(iced::Color::WHITE),
    };

    column![
        widget::vertical_space(),
        widget::text!("Are you sure you want to {msg1}?").size(20),
        msg2,
        row![
            widget::button(
                row![
                    icons::cross().style(t_white),
                    widget::text("No").style(t_white)
                ]
                .align_y(Alignment::Center)
                .spacing(10)
                .padding(3),
            )
            .on_press(no.clone())
            .style(|_, status| {
                style_button_color(status, (0x72, 0x22, 0x24), (0x9f, 0x2c, 0x2f))
            }),
            widget::button(
                row![
                    icons::deselectall().style(t_white),
                    widget::text("Yes").style(t_white)
                ]
                .align_y(Alignment::Center)
                .spacing(10)
                .padding(3),
            )
            .on_press(yes.clone())
            .style(|_, status| {
                style_button_color(status, (0x3f, 0x6a, 0x31), (0x46, 0x7e, 0x35))
            }),
        ]
        .spacing(5)
        .wrap(),
        widget::vertical_space(),
    ]
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .padding(10)
    .spacing(10)
    .into()
}

fn style_button_color(
    status: widget::button::Status,
    a: (u8, u8, u8),
    h: (u8, u8, u8),
) -> widget::button::Style {
    let color = if let widget::button::Status::Hovered = status {
        iced::Color::from_rgb8(h.0, h.1, h.2)
    } else {
        iced::Color::from_rgb8(a.0, a.1, a.2)
    };

    let border = iced::Border {
        color,
        width: 2.0,
        radius: 8.0.into(),
    };

    widget::button::Style {
        background: Some(iced::Background::Color(color)),
        text_color: iced::Color::WHITE,
        border,
        ..Default::default()
    }
}
