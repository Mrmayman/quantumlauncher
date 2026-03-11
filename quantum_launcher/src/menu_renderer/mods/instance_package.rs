use iced::{
    widget::{self, column, row},
    Length,
};

use crate::{
    icons,
    menu_renderer::{back_button, back_to_launch_screen, button_with_icon, Element},
    state::{MenuExportInstance, Message, PackageInstanceMessage},
};

impl MenuExportInstance {
    pub fn view(&'_ self, tick_timer: usize) -> Element<'_> {
        let top = if self.is_exporting {
            "Select the contents of the \".minecraft\" folder you want to keep"
        } else {
            "Select the contents of the instance you want to clone"
        };

        let btn = button_with_icon(icons::floppydisk(), "Export", 16)
            .on_press(PackageInstanceMessage::Start.into());

        let bottom: Element = if self.is_exporting {
            column![
                widget::text("Format:").size(12),
                row![
                    widget::pick_list(["QuantumLauncher"], Some("QuantumLauncher"), |_| {
                        Message::Nothing
                    })
                    .text_line_height(1.68),
                ]
                .spacing(5)
                .wrap()
            ]
            .spacing(2)
            .into()
        } else {
            btn.into()
        };

        column![
            back_button().on_press(back_to_launch_screen(None, None)),
            top,
            widget::scrollable(if let Some(entries) = &self.entries {
                widget::column(entries.iter().enumerate().map(|(i, (entry, enabled))| {
                    let name = if entry.is_file {
                        entry.name.clone()
                    } else {
                        format!("{}/", entry.name)
                    };
                    widget::checkbox(name, *enabled)
                        .on_toggle(move |t| PackageInstanceMessage::ToggleItem(i, t).into())
                        .into()
                }))
                .padding(5)
            } else {
                let dots = ".".repeat((tick_timer % 3) + 1);
                column![widget::text!("Loading{dots}")]
            })
            .width(Length::Fill)
            .height(Length::Fill),
            bottom,
        ]
        .padding(10)
        .spacing(10)
        .into()
    }
}
