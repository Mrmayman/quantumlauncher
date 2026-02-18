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
    const SHOW_DESC: bool = true;
    const BWIDTH: u16 = 120;
} else if #[cfg(target_os = "macos")] {
    const MENU_NAME: &str = "Applications";
    const SHOW_DESC: bool = false;
    const BWIDTH: u16 = 140;
} else {
    const MENU_NAME: &str = "the Applications Menu";
    const SHOW_DESC: bool = true;
    const BWIDTH: u16 = 150;
});

impl MenuShortcut {
    pub fn view<'a>(&'a self, accounts: &'a [String]) -> Element<'a> {
        let open_folder = widget::button(
            row![
                icons::folder_s(14),
                widget::text!("Open {MENU_NAME} Folder").size(10)
            ]
            .align_y(Alignment::Center)
            .spacing(10),
        )
        .width(BWIDTH)
        .on_press(Message::Shortcut(ShortcutMessage::OpenFolder));

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
                    open_folder,
                ]
                .align_y(Alignment::Center)
                .spacing(16),
                column![
                    widget::text!("Launch this instance directly from your Desktop/{MENU_NAME} with a single click")
                        .size(14)
                        .style(tsubtitle),
                    widget::text("Note: You can manually pin this to Taskbar/Dock/Panel later").size(12).style(tsubtitle)
                ].spacing(5),

                // Shortcut information (name, account, etc.)
                self.get_info_fields(accounts),

                row![
                    widget::container(column![
                        widget::checkbox(format!("Add to {MENU_NAME}"), self.add_to_menu)
                            .on_toggle(|t| Message::Shortcut(ShortcutMessage::ToggleAddToMenu(t)))
                            .size(12)
                            .text_size(12),
                        widget::checkbox("Add to Desktop", self.add_to_desktop)
                            .on_toggle(|t| Message::Shortcut(ShortcutMessage::ToggleAddToDesktop(t)))
                            .size(12)
                            .text_size(12),
                        widget::Space::with_height(4),
                        button_with_icon(icons::checkmark_s(14), "Create Shortcut", 14).on_press(Message::Shortcut(ShortcutMessage::SaveMenu)),
                    ].spacing(1)).padding([11, 10]),
                    widget::container(column![
                        widget::text("Or save a shortcut file to use anywhere")
                            .size(14)
                            .style(tsubtitle),
                        widget::text("(May not work everywhere)").size(12).style(tsubtitle),
                        widget::Space::with_height(5),
                        button_with_icon(icons::floppydisk_s(14), "Export Shortcut File...", 14).on_press(Message::Shortcut(ShortcutMessage::SaveCustom)),
                    ]).padding(10)
                ].spacing(10)
            ]
            .width(Length::Fill)
            .padding(16)
            .spacing(12)
        )
        .style(|t: &LauncherTheme, s| t.style_scrollable_flat_dark(s))
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
        .padding([0, 1])
    }
}
