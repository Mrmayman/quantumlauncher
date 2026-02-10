use crate::{
    icons,
    menu_renderer::{back_button, button_with_icon, tsubtitle, Element},
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
    const IS_UNIX: bool = false;
} else if #[cfg(target_os = "macos")] {
    const MENU_NAME: &str = "Applications";
    const IS_UNIX: bool = false;
} else {
    const MENU_NAME: &str = "the Applications Menu";
    const IS_UNIX: bool = true;
});

impl MenuShortcut {
    pub fn view<'a>(&'a self, accounts: &'a [String]) -> Element<'a> {
        row![
            widget::scrollable(
                column![
                    row![
                        back_button().on_press(Message::MScreenOpen {
                            message: None,
                            clear_selection: false,
                            is_server: None
                        }),
                        widget::text("Create Launch Shortcut").size(20),
                    ]
                    .align_y(Alignment::Center)
                    .spacing(16),
                    column![
                        widget::text!("Launch this instance directly from your Desktop or {MENU_NAME} without opening the launcher")
                            .size(14)
                            .style(tsubtitle),
                        widget::text("Note: You can manually pin this to Taskbar/Dock/Panel later").size(12).style(tsubtitle)
                    ].spacing(5),

                    // Shortcut information (name, account, etc.)
                    self.get_info_fields(accounts),

                    row![
                        button_with_icon(icons::checkmark_s(14), "Create Shortcut", 14).on_press(Message::Shortcut(ShortcutMessage::SaveMenu)),
                        column![
                            widget::checkbox(format!("Add to {MENU_NAME}"), self.add_to_menu)
                                .on_toggle(|t| Message::Shortcut(ShortcutMessage::ToggleAddToMenu(t)))
                                .size(12)
                                .text_size(12),
                            widget::checkbox("Add to Desktop", self.add_to_desktop)
                                .on_toggle(|t| Message::Shortcut(ShortcutMessage::ToggleAddToDesktop(t)))
                                .size(12)
                                .text_size(12),
                        ]
                    ]
                    .spacing(10),
                    column![
                        widget::text("Or save a shortcut file to use anywhere")
                            .size(14)
                            .style(tsubtitle),
                        widget::text("(May not work everywhere)").size(12).style(tsubtitle),
                        widget::Space::with_height(5),
                        button_with_icon(icons::floppydisk_s(14), "Export Shortcut File...", 14).on_press(Message::Shortcut(ShortcutMessage::SaveCustom)),
                    ]
                ]
                .width(Length::Fill)
                .padding(16)
                .spacing(12)
            )
            .style(|t: &LauncherTheme, s| t.style_scrollable_flat_dark(s)),
            widget::scrollable(
                column![
                    widget::text("Existing Shortcuts").size(20),
                    widget::text(
                        "TODO: Store and show shortcut info here, and have options to delete them"
                    )
                    .size(12)
                ]
                .padding(16)
                .spacing(10)
            )
            .height(Length::Fill)
            .width(200)
            .style(|t: &LauncherTheme, s| t.style_scrollable_flat_extra_dark(s))
        ]
        .into()
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
            " Name:",
            widget::text_input("(Required)", &self.shortcut.name)
                .size(14)
                .on_input(|n| Message::Shortcut(ShortcutMessage::EditName(n)))
        )]
        .push_maybe(IS_UNIX.then(|| {
            ifield(
                " Description:",
                widget::text_input("Leave blank for none", &self.shortcut.description)
                    .size(14)
                    .on_input(|n| Message::Shortcut(ShortcutMessage::EditDescription(n))),
            )
        }))
        .push(ifield(
            " Account:",
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
    }
}
