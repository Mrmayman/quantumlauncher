use iced::widget::tooltip::Position;
use iced::{widget, Alignment, Length};
use ql_core::{InstanceSelection, Progress};

use crate::{
    config::LauncherConfig,
    icon_manager,
    state::{
        AccountMessage, CreateInstanceMessage, InstallModsMessage, LauncherSettingsMessage,
        LicenseTab, ManageModsMessage, MenuCreateInstance, MenuCurseforgeManualDownload,
        MenuLauncherUpdate, MenuLicense, MenuServerCreate, Message, ProgressBar,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

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

pub type Element<'a> = iced::Element<'a, Message, LauncherTheme>;

pub fn link<'a>(
    e: impl Into<Element<'a>>,
    url: String,
) -> widget::Button<'a, Message, LauncherTheme> {
    widget::button(underline(e))
        .on_press(Message::CoreOpenLink(url))
        .padding(0)
        .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatDark))
}

pub fn underline<'a>(e: impl Into<Element<'a>>) -> widget::Stack<'a, Message, LauncherTheme> {
    widget::stack!(
        widget::column![e.into()],
        widget::column![
            widget::vertical_space(),
            widget::horizontal_rule(1)
                .style(|theme: &LauncherTheme| theme.style_rule(Color::Light, 1)),
            widget::Space::with_height(1),
        ]
    )
}

pub fn center_x<'a>(e: impl Into<Element<'a>>) -> Element<'a> {
    widget::row![
        widget::horizontal_space(),
        e.into(),
        widget::horizontal_space(),
    ]
    .into()
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
    button_with_icon(icon_manager::back_with_size(14), "Back", 14)
}

pub fn button_with_icon<'element>(
    icon: impl Into<Element<'element>>,
    text: &'element str,
    size: u16,
) -> widget::Button<'element, Message, LauncherTheme> {
    widget::button(
        widget::row![icon.into(), widget::text(text).size(size)]
            .align_y(iced::alignment::Vertical::Center)
            .spacing(10)
            .padding(3),
    )
}

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
    if current == selected {
        widget::container(widget::row!(widget::Space::with_width(5), text.into()))
            .style(LauncherTheme::style_container_selected_flat_button)
            .width(Length::Fill)
            .padding(5)
            .into()
    } else {
        widget::button(text)
            .on_press(message)
            .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatExtraDark))
            .width(Length::Fill)
            .into()
    }
}

impl MenuCreateInstance {
    pub fn view(&'_ self, list: Option<&Vec<String>>) -> Element<'_> {
        match self {
            MenuCreateInstance::LoadingList { .. } => widget::column![
                widget::row![
                    back_button().on_press(Message::CreateInstance(CreateInstanceMessage::Cancel)),
                    // button_with_icon(icon_manager::folder(), "Import Instance", 16)
                    //     .on_press(Message::CreateInstance(CreateInstanceMessage::Import)),
                ]
                .spacing(5),
                widget::text("Loading version list...").size(20),
            ]
            .padding(10)
            .spacing(10)
            .into(),
            MenuCreateInstance::Choosing {
                instance_name,
                selected_version,
                download_assets,
                combo_state,
                ..
            } => {
                let already_exists = list.is_some_and(|n| n.contains(instance_name));

                let create_button = widget::button(
                    widget::row![icon_manager::create(), "Create Instance"]
                        .spacing(10)
                        .padding(5),
                )
                .on_press_maybe(
                    (selected_version.is_some() && !instance_name.is_empty() && !already_exists)
                        .then(|| Message::CreateInstance(CreateInstanceMessage::Start)),
                );

                let create_button: Element = if selected_version.is_none() {
                    tooltip(
                        create_button,
                        "Select a version first!",
                        Position::FollowCursor,
                    )
                    .into()
                } else if instance_name.is_empty() {
                    tooltip(
                        create_button,
                        "Enter a name for your instance.",
                        Position::FollowCursor,
                    )
                    .into()
                } else if already_exists {
                    tooltip(
                        create_button,
                        "An instance with that name already exists!",
                        Position::FollowCursor,
                    )
                    .into()
                } else {
                    create_button.into()
                };

                widget::scrollable(
                    widget::column![
                        widget::row![
                            back_button()
                                .on_press(
                                    Message::LaunchScreenOpen {
                                        message: None,
                                        clear_selection: false
                                }),
                            // button_with_icon(icon_manager::folder(), "Import Instance", 16)
                            //     .on_press(Message::CreateInstance(CreateInstanceMessage::Import)),
                        ]
                        .spacing(5),
                        widget::combo_box(combo_state, "Select a version...", selected_version.as_ref(), |version| {
                            Message::CreateInstance(CreateInstanceMessage::VersionSelected(version))
                        }),
                        widget::text_input("Enter instance name...", instance_name)
                            .on_input(|n| Message::CreateInstance(CreateInstanceMessage::NameInput(n))),
                        tooltip(
                            widget::checkbox("Download assets?", *download_assets).on_toggle(|t| Message::CreateInstance(CreateInstanceMessage::ChangeAssetToggle(t))),
                            widget::text("If disabled, creating instance will be MUCH faster, but no sound or music will play in-game").size(12),
                            Position::Bottom
                        ),
                        create_button,
                        widget::text("To install Fabric/Forge/OptiFine/etc and mods, click on Mods after installing the instance").size(12),
                    ].push_maybe(
                        {
                            let real_platform = if cfg!(target_arch = "x86") { "x86_64" } else { "aarch64" };
                            (cfg!(target_os = "linux") && (cfg!(target_arch = "x86") || cfg!(target_arch = "arm")))
                                .then_some(
                                    widget::column![
                                    // WARN: Linux i686 and arm32
                                    widget::text("Warning: On your platform (Linux 32 bit) only Minecraft 1.16.5 and below are supported.").size(20),
                                    widget::text!("If your computer isn't outdated, you might have wanted to download QuantumLauncher 64 bit ({real_platform})"),
                                ]
                                )
                        })
                        .spacing(10)
                        .padding(10),
                )
                .style(LauncherTheme::style_scrollable_flat_dark)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            }
            MenuCreateInstance::DownloadingInstance(progress) => widget::column![
                widget::text("Downloading Instance..").size(20),
                progress.view()
            ]
            .padding(10)
            .spacing(5)
            .into(),
            MenuCreateInstance::ImportingInstance(progress) => widget::column![
                widget::text("Importing Instance..").size(20),
                progress.view()
            ]
            .padding(10)
            .spacing(5)
            .into(),
        }
    }
}

impl MenuLauncherUpdate {
    pub fn view(&'_ self) -> Element<'_> {
        if let Some(progress) = &self.progress {
            widget::column!("Updating QuantumLauncher...", progress.view())
        } else {
            widget::column!(
                "A new launcher update has been found! Do you want to download it?",
                widget::row!(
                    button_with_icon(icon_manager::download(), "Download", 16)
                        .on_press(Message::UpdateDownloadStart),
                    back_button().on_press(
                        Message::LaunchScreenOpen {
                            message: None,
                            clear_selection: false
                        }
                    ),
                    button_with_icon(icon_manager::globe(), "Open Website", 16)
                        .on_press(Message::CoreOpenLink("https://mrmayman.github.io/quantumlauncher".to_owned())),
                ).push_maybe(cfg!(target_os = "linux").then_some(
                    widget::column!(
                        // WARN: Package manager
                        "Note: If you installed this launcher from a package manager (flatpak/apt/dnf/pacman/..) it's recommended to update from there",
                        "If you just downloaded it from the website then continue from here."
                    )
                )).push_maybe(cfg!(target_os = "macos").then_some(
                    // WARN: macOS updater
                    "Note: The updater may be broken on macOS, so download the new version from the website"
                ))
                .spacing(5),
            )
        }
            .padding(10)
            .spacing(10)
            .into()
    }
}

pub fn get_theme_selector(config: &'_ LauncherConfig) -> (Element<'_>, Element<'_>) {
    const PADDING: iced::Padding = iced::Padding {
        top: 5.0,
        bottom: 5.0,
        right: 10.0,
        left: 10.0,
    };

    let theme = config.theme.as_deref().unwrap_or("Dark");
    let (light, dark): (Element, Element) = if theme == "Dark" {
        (
            widget::button(widget::text("Light").size(14))
                .on_press(Message::LauncherSettings(
                    LauncherSettingsMessage::ThemePicked("Light".to_owned()),
                ))
                .into(),
            widget::container(widget::text("Dark").size(14))
                .padding(PADDING)
                .into(),
        )
    } else {
        (
            widget::container(widget::text("Light").size(14))
                .padding(PADDING)
                .into(),
            widget::button(widget::text("Dark").size(14))
                .on_press(Message::LauncherSettings(
                    LauncherSettingsMessage::ThemePicked("Dark".to_owned()),
                ))
                .into(),
        )
    };
    (light, dark)
}

fn get_color_schemes(config: &'_ LauncherConfig) -> Element<'_> {
    // HOOK: Add more themes
    let styles = [
        "Brown".to_owned(),
        "Purple".to_owned(),
        "Sky Blue".to_owned(),
        "Catppuccin".to_owned(),
        "Teal".to_owned(),
    ];

    widget::pick_list(styles, config.style.clone(), |n| {
        Message::LauncherSettings(LauncherSettingsMessage::ColorSchemePicked(n))
    })
    .into()
}

fn back_to_launch_screen(
    selected_instance: &InstanceSelection,
    message: Option<String>,
) -> Message {
    match selected_instance {
        InstanceSelection::Server(selected_server) => Message::ServerManageOpen {
            selected_server: Some(selected_server.clone()),
            message,
        },
        InstanceSelection::Instance(_) => Message::LaunchScreenOpen {
            message: None,
            clear_selection: false,
        },
    }
}

impl<T: Progress> ProgressBar<T> {
    pub fn view(&'_ self) -> Element<'_> {
        let total = T::total();
        if let Some(message) = &self.message {
            widget::column!(
                widget::progress_bar(0.0..=total, self.num),
                widget::text(message)
            )
        } else {
            widget::column!(widget::progress_bar(0.0..=total, self.num),)
        }
        .spacing(10)
        .into()
    }
}

impl MenuCurseforgeManualDownload {
    pub fn view(&'_ self) -> Element<'_> {
        widget::column![
            "Some Curseforge mods have blocked this launcher!\nYou need to manually download the files and add them to your mods",

            widget::scrollable(
                widget::column(self.unsupported.iter().map(|entry| {
                    let url = format!(
                        "https://www.curseforge.com/minecraft/{}/{}/download/{}",
                        entry.project_type,
                        entry.slug,
                        entry.file_id
                    );

                    widget::row![
                        widget::button(widget::text("Open link").size(14)).on_press(Message::CoreOpenLink(url)),
                        widget::text(&entry.name)
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(10)
                    .into()
                }))
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(LauncherTheme::style_scrollable_flat_extra_dark),

            "Warning: Ignoring this may lead to crashes!",
            widget::row![
                widget::button("+ Select above downloaded files").on_press(Message::ManageMods(ManageModsMessage::AddFile)),
                widget::button("Continue").on_press(if self.is_store {
                    Message::InstallMods(InstallModsMessage::Open)
                } else {
                    Message::ManageMods(ManageModsMessage::ScreenOpenWithoutUpdate)
                }),
            ].spacing(5)
        ]
            .padding(10)
            .spacing(10)
            .into()
    }
}

impl MenuServerCreate {
    pub fn view(&'_ self) -> Element<'_> {
        match self {
            MenuServerCreate::LoadingList => {
                widget::column!(widget::text("Loading version list...").size(20),)
            }
            MenuServerCreate::Loaded {
                name,
                versions,
                selected_version,
                ..
            } => {
                widget::column!(
                    back_button().on_press(Message::ServerManageOpen {
                        selected_server: None,
                        message: None
                    }),
                    widget::text("Create new server").size(20),
                    widget::combo_box(
                        versions,
                        "Select a version...",
                        selected_version.as_ref(),
                        Message::ServerCreateVersionSelected
                    ),
                    widget::text_input("Enter server name...", name)
                        .on_input(Message::ServerCreateNameInput),
                    widget::button("Create Server").on_press_maybe(
                        (selected_version.is_some() && !name.is_empty())
                            .then(|| Message::ServerCreateStart)
                    ),
                )
            }
            MenuServerCreate::Downloading { progress } => {
                widget::column!(widget::text("Creating Server...").size(20), progress.view())
            }
        }
        .padding(10)
        .spacing(10)
        .into()
    }
}

impl MenuLicense {
    pub fn view(&'_ self) -> Element<'_> {
        widget::row![
            self.view_sidebar(),
            widget::scrollable(
                widget::text_editor(&self.content)
                    .on_action(Message::LicenseAction)
                    .style(LauncherTheme::style_text_editor_flat_extra_dark)
            )
            .style(LauncherTheme::style_scrollable_flat_dark)
        ]
        .into()
    }

    fn view_sidebar(&'_ self) -> Element<'_> {
        widget::column![
            widget::column![back_button().on_press(Message::LauncherSettings(
                LauncherSettingsMessage::ChangeTab(crate::state::LauncherSettingsTab::About)
            ))]
            .padding(10),
            widget::container(widget::column(LicenseTab::ALL.iter().map(|tab| {
                let text = widget::text(tab.to_string());
                sidebar_button(
                    tab,
                    &self.selected_tab,
                    text,
                    Message::LicenseChangeTab(*tab),
                )
            })))
            .height(Length::Fill)
            .width(200)
            .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark))
        ]
        .into()
    }
}

pub fn view_account_login<'a>() -> Element<'a> {
    widget::column![
        back_button().on_press(Message::LaunchScreenOpen {
            message: None,
            clear_selection: false
        }),
        widget::vertical_space(),
        widget::row![
            widget::horizontal_space(),
            widget::column![
                widget::text("Login").size(20),
                widget::button("Login with Microsoft").on_press(Message::Account(
                    AccountMessage::OpenMicrosoft {
                        is_from_welcome_screen: false
                    }
                )),
                widget::button("Login with ely.by").on_press(Message::Account(
                    AccountMessage::OpenElyBy {
                        is_from_welcome_screen: false
                    }
                )),
                widget::button("Login with littleskin").on_press(Message::Account(
                    AccountMessage::OpenLittleSkin {
                        is_from_welcome_screen: false
                    }
                )),
            ]
            .align_x(iced::Alignment::Center)
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
        widget::column!(
            widget::text!("Error: {error}"),
            widget::row![
                widget::button("Back").on_press(Message::LaunchScreenOpen {
                    message: None,
                    clear_selection: true
                }),
                widget::button("Copy Error").on_press(Message::CoreErrorCopy),
                widget::button("Copy Error + Log").on_press(Message::CoreErrorCopyLog),
                widget::button("Join Discord for help")
                    .on_press(Message::CoreOpenLink(DISCORD.to_owned()))
            ]
            .spacing(5)
            .wrap()
        )
        .padding(10)
        .spacing(10),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(LauncherTheme::style_scrollable_flat_extra_dark)
    .into()
}

pub fn view_log_upload_result(url: &'_ str, is_server: bool) -> Element<'_> {
    widget::column![
        back_button().on_press(Message::LaunchScreenOpen {
            message: None,
            clear_selection: false,
        }),
        widget::column![
            widget::vertical_space(),
            widget::text(format!(
                "{} log uploaded successfully!",
                if is_server { "Server" } else { "Game" }
            ))
            .size(20),
            widget::text("Your log has been uploaded to mclo.gs. You can share the link below:")
                .size(14),
            widget::container(
                widget::row![
                    widget::text(url).font(FONT_MONO),
                    widget::button("Copy")
                        .on_press(Message::CoreCopyText(url.to_string()))
                        .style(|theme: &LauncherTheme, status| {
                            theme.style_button(status, StyleButton::Round)
                        }),
                    widget::button("Open")
                        .on_press(Message::CoreOpenLink(url.to_string()))
                        .style(|theme: &LauncherTheme, status| {
                            theme.style_button(status, StyleButton::Round)
                        }),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center)
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

    widget::column![
        widget::vertical_space(),
        widget::text!("Are you sure you want to {msg1}?").size(20),
        msg2,
        widget::row![
            widget::button(
                widget::row![
                    icon_manager::cross().style(t_white),
                    widget::text("No").style(t_white)
                ]
                .align_y(iced::alignment::Vertical::Center)
                .spacing(10)
                .padding(3),
            )
            .on_press(no.clone())
            .style(|_, status| {
                style_button_color(status, (0x72, 0x22, 0x24), (0x9f, 0x2c, 0x2f))
            }),
            widget::button(
                widget::row![
                    icon_manager::tick().style(t_white),
                    widget::text("Yes").style(t_white)
                ]
                .align_y(iced::alignment::Vertical::Center)
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
