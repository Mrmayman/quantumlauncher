use crate::{
    icons,
    menu_renderer::{back_button, button_with_icon, tooltip, tsubtitle, Element},
    state::{MenuShortcut, Message, ShortcutMessage, OFFLINE_ACCOUNT_NAME},
    stylesheet::styles::LauncherTheme,
};
use cfg_if::cfg_if;
use iced::{
    widget::{self, column, row},
    Alignment, Length,
};

cfg_if!(if #[cfg(target_os = "windows")] {
    const MENU_NAME: &str = "the Start Menu";
    const SHOW_DESC: bool = true;
    const FOLDER_BUTTON_WIDTH: u16 = 120;
} else if #[cfg(target_os = "macos")] {
    const MENU_NAME: &str = "Applications";
    const SHOW_DESC: bool = false; // Shortcut description is unsupported on macOS
    const FOLDER_BUTTON_WIDTH: u16 = 140;
} else {
    const MENU_NAME: &str = "the Applications Menu";
    const SHOW_DESC: bool = true;
    const FOLDER_BUTTON_WIDTH: u16 = 150;
});
const ACTION_BUTTON_MENU: &str = constcat::concat!("Add to ", MENU_NAME);
const FOLDER_BUTTON: &str = constcat::concat!("Open ", MENU_NAME, " Folder");

impl MenuShortcut {
    pub fn view<'a>(&'a self, accounts: &'a [String]) -> Element<'a> {
        let action_buttons = self.get_action_buttons();

        widget::scrollable(
            column![
                row![
                    back_button().on_press(Message::MScreenOpen {
                        message: None,
                        clear_selection: false,
                        is_server: None
                    }),
                    widget::text("Create Launch Shortcut").size(20),
                    widget::horizontal_space(),
                    open_folder_button(),
                ]
                .align_y(Alignment::Center)
                .spacing(16),
                column![
                    widget::text!(
                        "Launch the instance directly from {MENU_NAME}/Desktop, with a single click"
                    )
                    .size(14)
                    .style(tsubtitle),
                    widget::text("Note: You can manually pin this to Taskbar/Dock/Panel later")
                        .size(12)
                        .style(tsubtitle)
                ]
                .spacing(5),
                // Shortcut information (name, account, etc.)
                self.get_info_fields(accounts),
                action_buttons,
            ]
            .width(Length::Fill)
            .padding(16)
            .spacing(12),
        )
        .style(|t: &LauncherTheme, s| t.style_scrollable_flat_dark(s))
        .into()
    }

    fn get_action_buttons(&self) -> widget::Row<'_, Message, LauncherTheme> {
        fn tooltip_maybe<'a>(t: Option<&'a str>, e: impl Into<Element<'a>>) -> Element<'a> {
            if let Some(t) = t {
                tooltip(e, t, widget::tooltip::Position::FollowCursor).into()
            } else {
                e.into()
            }
        }

        let disabled_tooltip = self.get_disabled_tooltip();

        row![
            widget::container(
                column![
                    widget::checkbox(ACTION_BUTTON_MENU, self.add_to_menu)
                        .on_toggle(|t| Message::Shortcut(ShortcutMessage::ToggleAddToMenu(t)))
                        .size(12)
                        .text_size(12),
                    widget::checkbox("Add to Desktop", self.add_to_desktop)
                        .on_toggle(|t| Message::Shortcut(ShortcutMessage::ToggleAddToDesktop(t)))
                        .size(12)
                        .text_size(12),
                    widget::Space::with_height(4),
                    tooltip_maybe(
                        disabled_tooltip,
                        button_with_icon(icons::checkmark_s(14), "Create Shortcut", 14)
                            .on_press_maybe(
                                disabled_tooltip
                                    .is_none()
                                    .then_some(Message::Shortcut(ShortcutMessage::SaveMenu))
                            )
                    ),
                ]
                .spacing(1)
            )
            .padding([10, 10]),
            widget::container(column![
                widget::text("Or save a shortcut file to use anywhere")
                    .size(14)
                    .style(tsubtitle),
                widget::text("(May not work everywhere)")
                    .size(12)
                    .style(tsubtitle),
                widget::Space::with_height(5),
                tooltip_maybe(
                    disabled_tooltip,
                    button_with_icon(icons::floppydisk_s(14), "Export Shortcut File...", 14)
                        .on_press_maybe(
                            disabled_tooltip
                                .is_none()
                                .then_some(Message::Shortcut(ShortcutMessage::SaveCustom))
                        )
                ),
            ])
            .padding(10)
        ]
        .spacing(10)
    }

    fn get_disabled_tooltip(&self) -> Option<&'static str> {
        if self.shortcut.name.is_empty() {
            Some("Shortcut name is empty")
        } else if self.account == OFFLINE_ACCOUNT_NAME {
            if self.account_offline.trim().is_empty() {
                Some("Username is empty")
            } else if self.account_offline.contains(' ') {
                Some("Username has spaces")
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_info_fields<'a>(
        &'a self,
        accounts: &'a [String],
    ) -> widget::Column<'a, Message, LauncherTheme> {
        fn ifield<'a>(
            name: &'a str,
            elem: impl Into<Element<'a>>,
        ) -> widget::Row<'a, Message, LauncherTheme> {
            row![widget::text(name).size(14).width(100), elem.into()]
                .spacing(10)
                .align_y(Alignment::Center)
        }

        column![ifield(
            "Name:",
            widget::text_input("(Required)", &self.shortcut.name)
                .size(14)
                .on_input(|n| Message::Shortcut(ShortcutMessage::EditName(n)))
        )]
        .push_maybe(SHOW_DESC.then(|| {
            ifield(
                "Description:",
                widget::text_input("Leave blank for none", &self.shortcut.description)
                    .size(14)
                    .on_input(|n| Message::Shortcut(ShortcutMessage::EditDescription(n))),
            )
        }))
        .push(ifield(
            "Account:",
            row![widget::pick_list(accounts, Some(&self.account), |n| {
                Message::Shortcut(ShortcutMessage::AccountSelected(n))
            })
            .text_size(14)
            .width(Length::Fill)]
            .push_maybe(
                (self.account == OFFLINE_ACCOUNT_NAME).then_some(
                    widget::text_input("Enter username...", &self.account_offline)
                        .size(14)
                        .width(Length::Fill)
                        .on_input(|n| Message::Shortcut(ShortcutMessage::AccountOffline(n))),
                ),
            )
            .spacing(5),
        ))
        .spacing(5)
        .padding([0, 1])
    }
}

fn open_folder_button() -> widget::Button<'static, Message, LauncherTheme> {
    widget::button(
        row![icons::folder_s(14), widget::text(FOLDER_BUTTON).size(10)]
            .align_y(Alignment::Center)
            .spacing(10),
    )
    .width(FOLDER_BUTTON_WIDTH)
    .on_press(Message::Shortcut(ShortcutMessage::OpenFolder))
}
