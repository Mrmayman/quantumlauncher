use crate::{
    icon_manager,
    menu_renderer::{tsubtitle, ui::button_with_icon, FONT_MONO},
    state::{
        CustomJarState, EditInstanceMessage, ListMessage, MenuEditInstance, Message, NONE_JAR_NAME,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};
use iced::{widget, Alignment, Length};
use ql_core::json::{instance_config::PreLaunchPrefixMode, GlobalSettings};
use ql_core::InstanceSelection;

use super::Element;

impl MenuEditInstance {
    pub fn view<'a>(
        &'a self,
        selected_instance: &InstanceSelection,
        jar_choices: Option<&'a CustomJarState>,
    ) -> Element<'a> {
        let bottom_part: Element = match selected_instance {
            InstanceSelection::Instance(_) => widget::column![
                widget::row![
                    button_with_icon(
                        icon_manager::update_with_size(12),
                        "Reinstall Libraries",
                        12
                    )
                    .on_press(Message::EditInstance(
                        EditInstanceMessage::ReinstallLibraries
                    )),
                    button_with_icon(icon_manager::update_with_size(12), "Update Assets", 12)
                        .on_press(Message::EditInstance(EditInstanceMessage::UpdateAssets)),
                ]
                .spacing(5)
                .wrap(),
                button_with_icon(icon_manager::delete(), "Delete Instance", 16)
                    .on_press(Message::DeleteInstanceMenu)
            ]
            .spacing(10)
            .into(),
            InstanceSelection::Server(_) => {
                button_with_icon(icon_manager::delete(), "Delete Server", 16)
                    .on_press(Message::DeleteInstanceMenu)
                    .into()
            }
        };

        widget::scrollable(
            widget::column![
                widget::container(
                    widget::column![
                        widget::row![
                            widget::text(selected_instance.get_name().to_owned()).size(20).font(FONT_MONO),
                        ].push_maybe((!self.is_editing_name).then_some(
                            widget::button(
                                icon_manager::edit_with_size(12)
                                    .style(|t: &LauncherTheme| t.style_text(Color::Mid))
                            ).style(|t: &LauncherTheme, s|
                                t.style_button(s, StyleButton::FlatDark)
                            )
                            .on_press(Message::EditInstance(EditInstanceMessage::RenameToggle))
                        )).spacing(5),

                        widget::text!("{} {}",
                            self.config.mod_type,
                            if selected_instance.is_server() {
                                "Server"
                            } else {
                                "Client"
                            }
                        ).style(|t: &LauncherTheme| t.style_text(Color::Mid)).size(14),
                    ].padding(10).spacing(5).push_maybe(self.is_editing_name.then_some(widget::column![
                        widget::Space::with_height(1),
                        widget::text_input("Rename Instance", &self.instance_name).on_input(|n| Message::EditInstance(EditInstanceMessage::RenameEdit(n))),
                        widget::row![
                            widget::button(widget::text("Rename").size(12)).on_press(Message::EditInstance(EditInstanceMessage::RenameApply)),
                            widget::button(widget::text("Cancel").size(12)).on_press(Message::EditInstance(EditInstanceMessage::RenameToggle))
                        ].spacing(5)
                    ].spacing(5))),
                ).width(Length::Fill)
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),

                widget::container(
                    self.item_java_override()
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                widget::container(
                    self.item_custom_jar(jar_choices)
                ).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),
                widget::container(
                    self.item_mem_alloc(),
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                widget::container(
                    widget::Column::new()
                    .push_maybe((!selected_instance.is_server()).then_some(widget::column![
                        widget::checkbox("Close launcher after game opens", self.config.close_on_start.unwrap_or(false))
                            .on_toggle(|t| Message::EditInstance(EditInstanceMessage::CloseLauncherToggle(t))),
                    ].spacing(5)))
                    .push(
                        widget::column![
                            widget::Space::with_height(5),
                            widget::checkbox("DEBUG: Enable log system (recommended)", self.config.enable_logger.unwrap_or(true))
                                .on_toggle(|t| Message::EditInstance(EditInstanceMessage::LoggingToggle(t))),
                            widget::text("Once disabled, logs will be printed in launcher STDOUT.\nRun the launcher executable from the terminal/command prompt to see it").size(12).style(tsubtitle),
                            widget::horizontal_space(),
                        ].spacing(5)
                    )
                    .padding(10)
                    .spacing(10)
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),
                widget::container(
                    self.item_args()
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                widget::container(
                    widget::Column::new()
                        .push_maybe((!selected_instance.is_server()).then_some(
                            resolution_dialog(
                                self.config.global_settings.as_ref(),
                                |n| Message::EditInstance(EditInstanceMessage::WindowWidthChanged(n)),
                                |n| Message::EditInstance(EditInstanceMessage::WindowHeightChanged(n)),
                        )))
                )
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark))
                .padding(10)
                .width(Length::Fill),
                widget::container(bottom_part)
                .width(Length::Fill)
                .padding(10)
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
            ]
        ).style(LauncherTheme::style_scrollable_flat_extra_dark).spacing(1).into()
    }

    fn item_args(&self) -> widget::Column<'_, Message, LauncherTheme> {
        let current_mode = self.config.global_java_args_enable.unwrap_or(true);

        widget::column!(
            widget::row![
                "Java arguments:",
                widget::horizontal_space(),
                widget::checkbox("Apply global arguments  ", current_mode)
                    .on_toggle(|t| {
                        Message::EditInstance(EditInstanceMessage::JavaArgsModeChanged(t))
                    })
                    .style(|t: &LauncherTheme, s| t.style_checkbox(s, Some(Color::SecondLight)))
                    .size(12)
                    .text_size(12)
            ]
            .align_y(Alignment::Center),
            get_args_list(
                self.config.java_args.as_deref(),
                |n| Message::EditInstance(EditInstanceMessage::JavaArgs(n)),
                true
            ),
            "Game arguments:",
            get_args_list(
                self.config.game_args.as_deref(),
                |n| Message::EditInstance(EditInstanceMessage::GameArgs(n)),
                true
            ),
            "Pre-launch prefix:",
            get_args_list(
                self.config
                    .global_settings
                    .as_ref()
                    .and_then(|n| n.pre_launch_prefix.as_deref()),
                |n| Message::EditInstance(EditInstanceMessage::PreLaunchPrefix(n)),
                true
            ),
            widget::container(
                widget::column![
                    widget::text("Interaction with global pre-launch prefix:").size(14),
                    widget::pick_list(
                        PreLaunchPrefixMode::ALL,
                        Some(self.config.pre_launch_prefix_mode.unwrap_or_default()),
                        |mode| {
                            Message::EditInstance(EditInstanceMessage::PreLaunchPrefixModeChanged(
                                mode,
                            ))
                        }
                    )
                    .placeholder("Select mode...")
                    .width(200)
                    .text_size(14),
                    widget::text(
                        self.config
                            .pre_launch_prefix_mode
                            .unwrap_or_default()
                            .get_description()
                    )
                    .size(12)
                    .style(tsubtitle),
                ]
                .padding(10)
                .spacing(7)
            ),
        )
        .padding(10)
        .spacing(7)
        .width(Length::Fill)
    }

    fn item_mem_alloc(&self) -> widget::Column<'_, Message, LauncherTheme> {
        // 2 ^ 8 = 256 MB
        const MEM_256_MB_IN_TWOS_EXPONENT: f32 = 8.0;
        // 2 ^ 13 = 8192 MB
        const MEM_8192_MB_IN_TWOS_EXPONENT: f32 = 13.0;

        widget::column![
            "Allocated memory",
            widget::text("For normal Minecraft, allocate 2 - 3 GB")
                .size(12)
                .style(tsubtitle),
            widget::text("For old versions, allocate 512 MB - 1 GB")
                .size(12)
                .style(tsubtitle),
            widget::text("For heavy modpacks/very high render distances, allocate 4 - 8 GB")
                .size(12)
                .style(tsubtitle),
            widget::slider(
                MEM_256_MB_IN_TWOS_EXPONENT..=MEM_8192_MB_IN_TWOS_EXPONENT,
                self.slider_value,
                |n| Message::EditInstance(EditInstanceMessage::MemoryChanged(n))
            )
            .step(0.1),
            widget::text(&self.slider_text),
        ]
        .padding(10)
        .spacing(5)
    }

    fn item_java_override(&self) -> widget::Column<'_, Message, LauncherTheme> {
        widget::column![
            "Custom Java executable (full path)",
            widget::text("Note: The launcher already sets up Java automatically,\nYou won't need this in most cases").size(12),
            widget::text_input(
                "Leave blank if none",
                self.config.java_override.as_deref().unwrap_or_default()
            )
            .on_input(|t| Message::EditInstance(EditInstanceMessage::JavaOverride(t)))
        ]
        .padding(10)
        .spacing(10)
    }

    fn item_custom_jar<'a>(
        &'a self,
        jar_choices: Option<&'a CustomJarState>,
    ) -> widget::Column<'a, Message, LauncherTheme> {
        let picker: Element = if let Some(choices) = jar_choices {
            widget::pick_list(
                choices.choices.as_slice(),
                Some(
                    self.config
                        .custom_jar
                        .as_ref()
                        .map(|n| n.name.clone())
                        .unwrap_or(NONE_JAR_NAME.to_owned()),
                ),
                |t| Message::EditInstance(EditInstanceMessage::CustomJarPathChanged(t)),
            )
            .into()
        } else {
            "Loading...".into()
        };

        widget::column![
            "Custom JAR file",
            widget::text(
                r#"This feature is for *replacing* the Minecraft JAR,
not adding to it.
If you want to apply tweaks to your existing JAR file,
use "Mods->Jar Mods""#
            )
            .size(12)
            .style(tsubtitle),
            widget::Space::with_height(2),
            picker,
            widget::Column::new().push_maybe(
                self.config.custom_jar.is_some().then_some(
                    widget::column![
                        widget::text("Try this in case the game crashes otherwise")
                            .size(12)
                            .style(tsubtitle),
                        widget::checkbox(
                            "Auto-set mainClass",
                            self.config
                                .custom_jar
                                .as_ref()
                                .is_some_and(|n| n.autoset_main_class)
                        )
                        .on_toggle(|n| Message::EditInstance(
                            EditInstanceMessage::AutoSetMainClassToggle(n)
                        )),
                        widget::Space::with_height(5),
                    ]
                    .spacing(5)
                )
            ),
        ]
        .padding(10)
        .spacing(5)
    }
}

pub fn resolution_dialog<'a>(
    global_settings: Option<&GlobalSettings>,
    width: impl Fn(String) -> Message + 'a,
    height: impl Fn(String) -> Message + 'a,
) -> widget::Column<'a, Message, LauncherTheme> {
    widget::column![
        "Custom Game Window Size (px):",
        widget::text("(Leave empty for default)\nCommon resolutions: 854x480, 1366x768, 1920x1080, 2560x1440, 3840x2160").size(12).style(tsubtitle),
        widget::row![
            widget::text("Width:").size(14),
            widget::text_input(
                "854",
                &global_settings
                    .and_then(|n| n.window_width)
                    .map_or(String::new(), |w| w.to_string())
            )
            .on_input(width)
            .width(100),
            widget::text("Height:").size(14),
            widget::text_input(
                "480",
                &global_settings
                    .and_then(|n| n.window_height)
                    .map_or(String::new(), |h| h.to_string())
            )
            .on_input(height)
            .width(100),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Center),
    ]
    .spacing(5)
}

pub fn get_args_list<'a>(
    args: Option<&'a [String]>,
    msg: impl Fn(ListMessage) -> Message + Clone + 'static,
    bg_extradark: bool,
) -> Element<'a> {
    const ITEM_SIZE: u16 = 10;

    let args = args.unwrap_or_default();
    fn opt<'a>(
        icon: widget::Text<'a, LauncherTheme>,
        bg_is_extra_dark: bool,
    ) -> widget::Button<'a, Message, LauncherTheme> {
        widget::button(icon)
            .padding([6, 8])
            .style(move |t: &LauncherTheme, s| {
                t.style_button(
                    s,
                    if bg_is_extra_dark {
                        StyleButton::FlatExtraDark
                    } else {
                        StyleButton::FlatDark
                    },
                )
            })
    }

    widget::Column::new()
        .push_maybe(
            (!args.is_empty()).then_some(widget::column(args.iter().enumerate().map(
                |(i, arg)| {
                    widget::row![
                        opt(icon_manager::delete_with_size(ITEM_SIZE), bg_extradark)
                            .on_press(msg(ListMessage::Delete(i))),
                        opt(icon_manager::arrow_up_with_size(ITEM_SIZE), bg_extradark)
                            .on_press(msg(ListMessage::ShiftUp(i))),
                        opt(icon_manager::arrow_down_with_size(ITEM_SIZE), bg_extradark)
                            .on_press(msg(ListMessage::ShiftDown(i))),
                        widget::text_input("Enter argument...", arg)
                            .size(ITEM_SIZE + 4)
                            .on_input({
                                let msg = msg.clone();
                                move |n| msg(ListMessage::Edit(n, i))
                            })
                    ]
                    .align_y(Alignment::Center)
                    .into()
                },
            ))),
        )
        .push(get_args_list_add_button(msg, bg_extradark))
        .spacing(5)
        .into()
}

fn get_args_list_add_button(
    msg: impl Fn(ListMessage) -> Message + Clone + 'static,
    bg_extradark: bool,
) -> widget::Button<'static, Message, LauncherTheme> {
    widget::button(
        widget::row![
            icon_manager::create_with_size(13),
            widget::text("Add").size(13)
        ]
        .align_y(iced::alignment::Vertical::Center)
        .spacing(8)
        .padding([1, 2]),
    )
    .style(move |t: &LauncherTheme, s| {
        t.style_button(
            s,
            if bg_extradark {
                StyleButton::RoundDark
            } else {
                StyleButton::Round
            },
        )
    })
    .on_press(msg(ListMessage::Add))
}
