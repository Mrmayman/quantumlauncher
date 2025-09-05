use iced::{widget, Length};
use ql_core::{ModId, SelectedMod};

use crate::{
    icon_manager,
    menu_renderer::{back_button, underline, Element},
    state::{ExportModsMessage, ManageModsMessage, MenuExportMods, Message},
    stylesheet::{
        color::Color,
        styles::{LauncherTheme, BORDER_RADIUS},
        widgets::StyleButton,
    },
};

impl MenuExportMods {
    pub fn view(&'_ self) -> Element<'_> {
        if self.selected_mods.is_empty() {
            return self.get_top_section().padding(25).into();
        }

        widget::scrollable(
            widget::column![
                self.get_top_section(),
                Self::get_controls(),
                widget::column![
                    widget::text("Preview:")
                        .size(18)
                        .style(|theme: &LauncherTheme| { theme.style_text(Color::Light) }),
                    widget::container(self.get_preview_content())
                        .style(|theme: &LauncherTheme| {
                            theme.style_container_round_box(0.0, Color::ExtraDark, BORDER_RADIUS)
                        })
                        .padding(15)
                        .width(Length::Fill),
                ]
                .spacing(10)
            ]
            .spacing(25)
            .padding(25),
        )
        .style(LauncherTheme::style_scrollable_flat_dark)
        .into()
    }

    fn get_controls<'a>() -> widget::Column<'a, Message, LauncherTheme> {
        widget::column![
            widget::text("Choose export format:").size(20),
            widget::row![
                icon_manager::text_file_with_size(28),
                widget::column![
                    widget::text("Export as Plain Text").size(17),
                    widget::text("Simple text file with mod names, one per line")
                        .size(13)
                        .style(|theme: &LauncherTheme| { theme.style_text(Color::SecondLight) }),
                ]
                .spacing(4),
                widget::horizontal_space(),
                widget::row![
                    widget::button(widget::text("Copy").size(14))
                        .padding([8, 16])
                        .on_press(Message::ExportMods(
                            ExportModsMessage::CopyPlainTextToClipboard
                        )),
                    widget::button(widget::text("Save").size(14))
                        .padding([8, 16])
                        .style(|theme: &LauncherTheme, status| {
                            theme.style_button(status, StyleButton::FlatDark)
                        })
                        .on_press(Message::ExportMods(ExportModsMessage::ExportAsPlainText)),
                ]
                .spacing(12)
            ]
            .spacing(20)
            .align_y(iced::Alignment::Center)
            .padding([10, 20]),
            widget::row![
                icon_manager::text_file_with_size(28),
                widget::column![
                    widget::text("Export as Markdown")
                        .size(17)
                        .style(|theme: &LauncherTheme| { theme.style_text(Color::Light) }),
                    widget::text("Markdown file with clickable mod links")
                        .size(13)
                        .style(|theme: &LauncherTheme| { theme.style_text(Color::SecondLight) }),
                ]
                .spacing(4),
                widget::horizontal_space(),
                widget::row![
                    widget::button(widget::text("Copy").size(14))
                        .padding([8, 16])
                        .on_press(Message::ExportMods(
                            ExportModsMessage::CopyMarkdownToClipboard,
                        )),
                    widget::button(widget::text("Save").size(14))
                        .padding([8, 16])
                        .style(|theme: &LauncherTheme, status| {
                            use crate::stylesheet::widgets::StyleButton;
                            theme.style_button(status, StyleButton::FlatDark)
                        })
                        .on_press(Message::ExportMods(ExportModsMessage::ExportAsMarkdown,))
                ]
                .spacing(12)
            ]
            .spacing(20)
            .align_y(iced::Alignment::Center)
            .padding([10, 20])
        ]
        .spacing(5)
    }

    fn get_top_section(&self) -> widget::Column<'_, Message, LauncherTheme> {
        let len = self.selected_mods.len();

        widget::column![
            widget::row![
                back_button().on_press(Message::ManageMods(
                    ManageModsMessage::ScreenOpenWithoutUpdate
                )),
                widget::text("Export Mods List")
                    .size(24)
                    .style(|theme: &LauncherTheme| { theme.style_text(Color::Light) }),
            ]
            .spacing(15)
            .align_y(iced::Alignment::Center),
            widget::text(if len == 0 {
                "No mods selected - please select some mods first".to_string()
            } else {
                format!(
                    "{} mod{} selected for export",
                    len,
                    if len == 1 { "" } else { "s" }
                )
            })
            .style(move |theme: &LauncherTheme| {
                if len > 0 {
                    theme.style_text(Color::SecondLight)
                } else {
                    theme.style_text(Color::SecondDark)
                }
            }),
        ]
        .spacing(20)
    }

    fn get_preview_content(&'_ self) -> Element<'_> {
        const ELEM_HEIGHT: u16 = 26;

        let mut preview_elements = Vec::new();

        for selected_mod in self.selected_mods.iter() {
            match selected_mod {
                SelectedMod::Downloaded { name, id } => {
                    let url = match id {
                        ModId::Modrinth(mod_id) => {
                            format!("https://modrinth.com/mod/{mod_id}")
                        }
                        ModId::Curseforge(mod_id) => {
                            format!("https://www.curseforge.com/projects/{mod_id}")
                        }
                    };

                    let link_element = widget::button(
                        widget::row![
                            widget::Space::with_width(5),
                            widget::text("-")
                                .size(13)
                                .style(|theme: &LauncherTheme| theme.style_text(Color::Mid)),
                            underline(widget::text(name).size(13)),
                            widget::text("→")
                                .size(13)
                                .style(|theme: &LauncherTheme| theme
                                    .style_text(Color::SecondLight))
                        ]
                        .height(Length::Fill)
                        .align_y(iced::Alignment::Center)
                        .spacing(8),
                    )
                    .style(|theme: &LauncherTheme, status| {
                        use crate::stylesheet::widgets::StyleButton;
                        theme.style_button(status, StyleButton::FlatExtraDark)
                    })
                    .padding(0)
                    .height(ELEM_HEIGHT)
                    .on_press(Message::CoreOpenLink(url));

                    preview_elements.push(link_element.into());
                }
                SelectedMod::Local { file_name } => {
                    let display_name = file_name
                        .strip_suffix(".jar")
                        .or_else(|| file_name.strip_suffix(".zip"))
                        .unwrap_or(file_name.as_str());

                    let text_element = widget::row![
                        widget::Space::with_width(5),
                        widget::text("-")
                            .size(13)
                            .style(|theme: &LauncherTheme| theme.style_text(Color::Mid)),
                        widget::text(display_name)
                            .size(13)
                            .style(|theme: &LauncherTheme| theme.style_text(Color::Light)),
                        widget::text("(local)")
                            .size(12)
                            .style(|theme: &LauncherTheme| theme.style_text(Color::Mid))
                    ]
                    .align_y(iced::Alignment::Center)
                    .height(ELEM_HEIGHT)
                    .spacing(8);

                    preview_elements.push(text_element.into());
                }
            }
            // preview_elements.push(
            //     widget::horizontal_rule(1)
            //         .style(|t: &LauncherTheme| t.style_rule(Color::SecondDark, 1))
            //         .into(),
            // );
        }

        widget::column(preview_elements).spacing(5).into()
    }
}
