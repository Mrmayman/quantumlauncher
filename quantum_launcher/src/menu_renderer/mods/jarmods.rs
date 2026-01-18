use iced::{
    widget::{self, column},
    Length,
};
use ql_core::InstanceSelection;

use crate::{
    icons,
    menu_renderer::{back_button, button_with_icon, link, ui::checkbox, Element},
    state::{ManageJarModsMessage, ManageModsMessage, MenuEditJarMods, Message, SelectedState},
    stylesheet::{color::Color, styles::LauncherTheme},
};

impl MenuEditJarMods {
    pub fn view(&'_ self, selected_instance: &InstanceSelection) -> Element<'_> {
        let menu_main = widget::row!(
            widget::container(
                widget::scrollable(
                    column!(
                        back_button().on_press(Message::ManageMods(
                            ManageModsMessage::ScreenOpenWithoutUpdate
                        )),
                        column![
                            {
                                let path = selected_instance.get_instance_path().join("jarmods");

                                button_with_icon(icons::folder_s(14), "Open Folder", 14)
                                    .on_press(Message::CoreOpenPath(path))
                            },
                            button_with_icon(icons::new_s(14), "Add file", 14)
                                .on_press(Message::ManageJarMods(ManageJarModsMessage::AddFile)),
                        ]
                        .spacing(5),
                        widget::row![
                            "You can find some good jar mods at ",
                            link("McArchive", "https://mcarchive.net".to_owned()),
                        ]
                        .wrap(),
                        widget::rule::horizontal(1),
                        column![
                            "WARNING: JarMods are mainly for OLD Minecraft versions.",
                            widget::space().height(5),
                            widget::text(
                                "This is easier than copying .class files into Minecraft's jar"
                            )
                            .size(12),
                            widget::text(
                                "If you just want some mods (for newer Minecraft), click Back"
                            )
                            .size(12),
                        ],
                    )
                    .padding(10)
                    .spacing(10)
                )
                .style(LauncherTheme::style_scrollable_flat_dark)
                .height(Length::Fill)
            )
            .width(250)
            .style(|n| n.style_container_sharp_box(0.0, Color::Dark)),
            self.get_mod_list()
        );

        if self.drag_and_drop_hovered {
            widget::stack!(
                menu_main,
                widget::center(widget::button(
                    widget::text("Drag and drop JarMod files to add them").size(20)
                ))
            )
            .into()
        } else {
            menu_main.into()
        }
    }

    fn get_mod_list(&'_ self) -> Element<'_> {
        if self.jarmods.mods.is_empty() {
            return column!("Add some mods to get started")
                .spacing(10)
                .padding(10)
                .width(Length::Fill)
                .into();
        }

        widget::container(
            column!(
                column![
                    widget::text("Select some JarMods to perform actions on them").size(14),
                    widget::row![
                        widget::button("Delete")
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::DeleteSelected)),
                        widget::button("Toggle")
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::ToggleSelected)),
                        widget::button(if matches!(self.selected_state, SelectedState::All) {
                            "Unselect All"
                        } else {
                            "Select All"
                        })
                        .on_press(Message::ManageJarMods(ManageJarModsMessage::SelectAll)),
                        widget::button(icons::arrow_up())
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::MoveUp)),
                        widget::button(icons::arrow_down())
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::MoveDown)),
                    ]
                    .spacing(5)
                    .wrap()
                ]
                .padding(10)
                .spacing(5),
                self.get_mod_list_contents(),
            )
            .spacing(10),
        )
        .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))
        .into()
    }

    fn get_mod_list_contents(&self) -> Element<'_> {
        widget::scrollable(
            widget::column({
                self.jarmods.mods.iter().map(|jarmod| {
                    checkbox(
                        widget::text(format!(
                            "{}{}",
                            if jarmod.enabled { "" } else { "(DISABLED) " },
                            jarmod.filename
                        )),
                        self.selected_mods.contains(&jarmod.filename),
                        move |t| {
                            Message::ManageJarMods(ManageJarModsMessage::ToggleCheckbox(
                                jarmod.filename.clone(),
                                t,
                            ))
                        },
                    )
                })
            })
            .padding(10)
            .spacing(10),
        )
        .style(LauncherTheme::style_scrollable_flat_extra_dark)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
