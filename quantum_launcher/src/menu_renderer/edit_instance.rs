use crate::{
    icons,
    menu_renderer::{
        button_with_icon, checkered_list, settings::PREFIX_EXPLANATION, tsubtitle, FONT_MONO,
    },
    state::{
        CustomJarState, EditInstanceMessage, ListMessage, MenuEditInstance, Message, NONE_JAR_NAME,
    },
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};
use iced::{
    widget::{self, column, horizontal_space, row},
    Alignment, Length,
};
use ql_core::InstanceSelection;
use ql_core::{
    json::{
        instance_config::{MainClassMode, PreLaunchPrefixMode},
        GlobalSettings,
    },
    JavaVersion,
};

use super::Element;

impl MenuEditInstance {
    pub fn view<'a>(
        &'a self,
        selected_instance: &InstanceSelection,
        jar_choices: Option<&'a CustomJarState>,
    ) -> Element<'a> {
        widget::scrollable(
            checkered_list([
                self.item_rename(selected_instance),
                self.item_mem_alloc(),

                if selected_instance.is_server() {
                    column![widget::button("Edit server.properties")]
                } else {
                    resolution_dialog(
                            self.config.global_settings.as_ref(),
                            |n| Message::EditInstance(EditInstanceMessage::WindowWidthChanged(n)),
                            |n| Message::EditInstance(EditInstanceMessage::WindowHeightChanged(n)),
                    )
                },

                widget::Column::new()
                .push_maybe((!selected_instance.is_server()).then_some(column![
                    widget::checkbox("Close launcher after game opens", self.config.close_on_start.unwrap_or(false))
                        .on_toggle(|t| Message::EditInstance(EditInstanceMessage::CloseLauncherToggle(t))),
                ].spacing(5)))
                .push(
                    column![
                        widget::Space::with_height(5),
                        widget::checkbox("DEBUG: Enable log system (recommended)", self.config.enable_logger.unwrap_or(true))
                            .on_toggle(|t| Message::EditInstance(EditInstanceMessage::LoggingToggle(t))),
                        widget::text("Once disabled, logs will be printed in launcher STDOUT.\nRun the launcher executable from the terminal/command prompt to see it").size(12).style(tsubtitle),
                        widget::horizontal_space(),
                    ].spacing(5)
                )
                .spacing(10),

                self.item_args(),
                self.item_java_override(),
                self.item_custom_jar(jar_choices),

                item_footer(selected_instance)
            ]),
        ).style(LauncherTheme::style_scrollable_flat_extra_dark).spacing(1).into()
    }

    fn item_rename(
        &self,
        selected_instance: &InstanceSelection,
    ) -> widget::Column<'_, Message, LauncherTheme> {
        column![
            row![widget::text(selected_instance.get_name().to_owned())
                .size(20)
                .font(FONT_MONO)]
            .push_maybe(
                (!self.is_editing_name).then_some(
                    widget::button(
                        icons::edit_s(12).style(|t: &LauncherTheme| t.style_text(Color::Mid))
                    )
                    .style(|t: &LauncherTheme, s| t.style_button(s, StyleButton::FlatDark))
                    .on_press(Message::EditInstance(EditInstanceMessage::RenameToggle))
                )
            )
            .spacing(5),
            widget::text!(
                "{} {}",
                self.config.mod_type,
                if selected_instance.is_server() {
                    "Server"
                } else {
                    "Client"
                }
            )
            .style(|t: &LauncherTheme| t.style_text(Color::Mid))
            .size(14),
        ]
        .width(Length::Fill)
        .spacing(5)
        .push_maybe(
            self.is_editing_name.then_some(
                column![
                    widget::Space::with_height(1),
                    widget::text_input("Rename Instance", &self.instance_name)
                        .on_input(|n| Message::EditInstance(EditInstanceMessage::RenameEdit(n))),
                    row![
                        widget::button(widget::text("Rename").size(12))
                            .on_press(Message::EditInstance(EditInstanceMessage::RenameApply)),
                        widget::button(widget::text("Cancel").size(12))
                            .on_press(Message::EditInstance(EditInstanceMessage::RenameToggle))
                    ]
                    .spacing(5)
                ]
                .spacing(5),
            ),
        )
    }

    fn item_args(&self) -> widget::Column<'_, Message, LauncherTheme> {
        let current_mode = self.config.global_java_args_enable.unwrap_or(true);
        let prefix_mode = self.config.pre_launch_prefix_mode.unwrap_or_default();

        let sp = || widget::Space::with_height(5);

        column![
            row![
                "Java arguments:",
                widget::horizontal_space(),
                widget::checkbox("Use global arguments", current_mode)
                    .on_toggle(|t| {
                        Message::EditInstance(EditInstanceMessage::JavaArgsModeChanged(t))
                    })
                    .style(|t: &LauncherTheme, s| t.style_checkbox(s, Some(Color::SecondLight)))
                    .size(12)
                    .text_size(12)
            ]
            .align_y(Alignment::Center),
            get_args_list(self.config.java_args.as_deref(), |n| Message::EditInstance(
                EditInstanceMessage::JavaArgs(n)
            )),
            sp(),
            "Game arguments:",
            get_args_list(self.config.game_args.as_deref(), |n| Message::EditInstance(
                EditInstanceMessage::GameArgs(n)
            )),
            sp(),
            self.item_args_prefix(prefix_mode),
            sp(),
            args_split_by_space(self.arg_split_by_space),
        ]
        .spacing(7)
        .width(Length::Fill)
    }

    fn item_args_prefix(
        &self,
        prefix_mode: PreLaunchPrefixMode,
    ) -> widget::Column<'_, Message, LauncherTheme> {
        let checkbox = widget::checkbox("Use global prefix", !prefix_mode.is_disabled())
            .on_toggle(|t| {
                Message::EditInstance(EditInstanceMessage::PreLaunchPrefixModeChanged(if t {
                    PreLaunchPrefixMode::default()
                } else {
                    PreLaunchPrefixMode::Disable
                }))
            })
            .style(|t: &LauncherTheme, s| t.style_checkbox(s, Some(Color::SecondLight)))
            .size(12)
            .text_size(12);

        column![
            row!["Pre-launch prefix:", horizontal_space(), checkbox].align_y(Alignment::Center),
            row![get_args_list(
                self.config
                    .global_settings
                    .as_ref()
                    .and_then(|n| n.pre_launch_prefix.as_deref()),
                |n| Message::EditInstance(EditInstanceMessage::PreLaunchPrefix(n)),
            )]
            .push_maybe(
                (!prefix_mode.is_disabled()).then_some(
                    widget::column(
                        [
                            PreLaunchPrefixMode::CombineGlobalLocal,
                            PreLaunchPrefixMode::CombineLocalGlobal,
                        ]
                        .iter()
                        .map(|n| {
                            widget::radio(n.get_description(), *n, Some(prefix_mode), |n| {
                                Message::EditInstance(
                                    EditInstanceMessage::PreLaunchPrefixModeChanged(n),
                                )
                            })
                            .style(|t: &LauncherTheme, s| t.style_radio(s, Color::SecondLight))
                            .size(10)
                            .text_size(10)
                            .into()
                        }),
                    )
                    .spacing(1),
                ),
            )
            .spacing(10),
            widget::text(PREFIX_EXPLANATION).size(12).style(tsubtitle),
        ]
        .width(Length::Fill)
        .spacing(7)
    }

    fn item_mem_alloc(&self) -> widget::Column<'_, Message, LauncherTheme> {
        // 2 ^ 8 = 256 MB
        const MEM_256_MB_IN_TWOS_EXPONENT: f32 = 8.0;
        // 2 ^ 15 = 32768 MB (32 GB)
        const MEM_32768_MB_IN_TWOS_EXPONENT: f32 = 15.0;

        const RAM_16_GB_TO_MB: usize = 16384;

        column![
            "Allocated memory",
            widget::text(
                r"Normal Minecraft: 2-3 GB
Old versions: 512 MB - 1 GB
Heavy modpacks / High settings: 4-8 GB+"
            )
            .size(12)
            .style(tsubtitle),
            widget::Space::with_height(5),
            row![
                widget::text(&self.slider_text),
                widget::slider(
                    MEM_256_MB_IN_TWOS_EXPONENT..=MEM_32768_MB_IN_TWOS_EXPONENT,
                    self.slider_value,
                    |n| Message::EditInstance(EditInstanceMessage::MemoryChanged(n))
                )
                .step(0.1),
            ]
            .align_y(Alignment::Center)
            .spacing(10),
            row![
                widget::text("Or enter directly:").size(12).style(tsubtitle),
                widget::text_input("2048", &self.memory_input)
                    .on_input(|n| Message::EditInstance(EditInstanceMessage::MemoryInputChanged(n)))
                    .width(64)
                    .size(12),
                widget::text("MB").size(12).style(tsubtitle),
            ]
            .align_y(Alignment::Center)
            .spacing(5)
        ]
        .push_maybe(
            (self.config.ram_in_mb > RAM_16_GB_TO_MB).then_some(
                widget::text(
                    "Warning: Very high RAM allocated! (16+ GB)\nYour system may struggle",
                )
                .size(14),
            ),
        )
        .spacing(5)
    }

    fn item_java_override(&self) -> widget::Column<'_, Message, LauncherTheme> {
        fn radio(
            l: &str,
            a: bool,
            b: Option<bool>,
            f: impl Fn() -> Message,
        ) -> widget::Radio<'_, Message, LauncherTheme> {
            widget::radio(l, a, b, |n| if n { f() } else { Message::Nothing })
                .text_size(14)
                .size(14)
        }

        let java_override = self.config.java_override.as_deref().unwrap_or_default();
        let java_override_ver = self.config.java_override_version;
        column![
            "Java",
            radio(
                "Auto-managed by launcher",
                true,
                Some(java_override.trim().is_empty() && java_override_ver.is_none()),
                || Message::EditInstance(EditInstanceMessage::JavaOverride(String::new()))
            ),
            row![radio(
                "Specific Java version",
                true,
                Some(java_override_ver.is_some()),
                || Message::EditInstance(EditInstanceMessage::JavaOverrideVersion(25))
            )]
            .push_maybe(java_override_ver.map(|n| {
                widget::row(JavaVersion::ALL.iter().map(|v| {
                    let v = *v as usize;
                    widget::radio(format!("{v}"), v, Some(n), |v| {
                        Message::EditInstance(EditInstanceMessage::JavaOverrideVersion(v))
                    })
                    .text_size(13)
                    .size(11)
                    .spacing(4)
                    .into()
                }))
                .spacing(5)
                .wrap()
            }))
            .spacing(16)
            .align_y(Alignment::Center),
            radio(
                "Custom path",
                true,
                Some(!java_override.trim().is_empty()),
                || Message::EditInstance(EditInstanceMessage::JavaOverride(
                    "path/to/java".to_owned()
                )) // ugly hack
            ),
            row![
                widget::Space::with_width(16),
                widget::text_input("Leave blank if none", java_override)
                    .size(14)
                    .font(FONT_MONO)
                    .on_input(|t| Message::EditInstance(EditInstanceMessage::JavaOverride(t)))
            ]
            .push_maybe(
                (!java_override.trim().is_empty()).then_some(
                    button_with_icon(icons::close_s(9), "", 13)
                        .padding([8.0, 11.0])
                        .on_press(Message::EditInstance(EditInstanceMessage::JavaOverride(
                            String::new()
                        ))),
                )
            )
            .push(
                button_with_icon(icons::folder_s(14), "", 13)
                    .padding([5, 10])
                    .on_press(Message::EditInstance(
                        EditInstanceMessage::BrowseJavaOverride
                    ))
            )
            .spacing(5)
        ]
        .spacing(5)
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

        column![
            row!["Custom JAR file", horizontal_space(), picker].align_y(Alignment::Center),
            widget::text(
                "For *replacing* the Minecraft JAR, not adding to it.\nTo patch your existing JAR file, use \"Mods->Jarmod Patches\""
            )
            .size(12)
            .style(tsubtitle),
            widget::Space::with_height(10),
            widget::text("Main Class:"),
            widget::radio("Default", None, Some(self.main_class_mode), |t| {
                Message::EditInstance(EditInstanceMessage::SetMainClass(t, None))
            })
            .size(14)
            .text_size(13),
            widget::radio(
                "Safe Mode (might fix crashes?)",
                Some(MainClassMode::SafeFallback),
                Some(self.main_class_mode),
                |t| Message::EditInstance(EditInstanceMessage::SetMainClass(t, None))
            )
            .size(14)
            .text_size(13),
            row![widget::radio(
                "Custom",
                Some(MainClassMode::Custom),
                Some(self.main_class_mode),
                |t| Message::EditInstance(EditInstanceMessage::SetMainClass(
                    t,
                    Some("".to_owned())
                ))
            )
            .size(14)
            .text_size(13)]
            .push_maybe(
                (self.main_class_mode == Some(MainClassMode::Custom)).then_some(
                    widget::text_input(
                        "Enter main class...",
                        self.config
                            .main_class_override
                            .as_deref()
                            .unwrap_or_default()
                    )
                    .on_input(|t| Message::EditInstance(
                        EditInstanceMessage::SetMainClass(Some(MainClassMode::Custom), Some(t))
                    ))
                    .font(FONT_MONO)
                    .size(13)
                )
            )
            .spacing(10),
        ]
        .spacing(5)
    }
}

fn item_footer(
    selected_instance: &InstanceSelection,
) -> widget::Column<'static, Message, LauncherTheme> {
    match selected_instance {
        InstanceSelection::Instance(_) => column![
            row![
                button_with_icon(icons::version_download_s(14), "Reinstall Libraries", 13)
                    .padding([4, 8])
                    .on_press(Message::EditInstance(
                        EditInstanceMessage::ReinstallLibraries
                    )),
                button_with_icon(icons::version_download_s(14), "Update Assets", 13)
                    .padding([4, 8])
                    .on_press(Message::EditInstance(EditInstanceMessage::UpdateAssets)),
                button_with_icon(icons::version_download_s(14), "Change LWJGL Version", 13)
                    .padding([4, 8])
                    .on_press(Message::EditInstance(EditInstanceMessage::LwjglScreenOpen)),
            ]
            .spacing(5)
            .wrap(),
            widget::horizontal_rule(2),
            button_with_icon(icons::bin(), "Delete Instance", 16)
                .on_press(Message::DeleteInstanceMenu)
        ]
        .spacing(10),
        InstanceSelection::Server(_) => {
            column![button_with_icon(icons::bin(), "Delete Server", 16)
                .on_press(Message::DeleteInstanceMenu)]
        }
    }
}

pub fn resolution_dialog<'a>(
    global_settings: Option<&GlobalSettings>,
    width: impl Fn(String) -> Message + 'a,
    height: impl Fn(String) -> Message + 'a,
) -> widget::Column<'a, Message, LauncherTheme> {
    column![
        "Custom Game Window Size (px):",
        widget::text("(Leave empty for default)\nCommon resolutions: 854x480, 1366x768, 1920x1080, 2560x1440, 3840x2160").size(12).style(tsubtitle),
        row![
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
        .align_y(Alignment::Center),
    ]
    .spacing(5)
}

pub fn get_args_list(
    args: Option<&[String]>,
    msg: impl Fn(ListMessage) -> Message + Clone + 'static,
) -> Element<'_> {
    const ITEM_SIZE: u16 = 10;

    let args = args.unwrap_or_default();

    fn opt(icon: widget::Text<'_, LauncherTheme>) -> widget::Button<'_, Message, LauncherTheme> {
        widget::button(icon)
            .padding([6, 8])
            .style(move |t: &LauncherTheme, s| t.style_button(s, StyleButton::FlatDark))
    }

    widget::Column::new()
        .push_maybe(
            (!args.is_empty()).then_some(widget::column(args.iter().enumerate().map(
                |(i, arg)| {
                    row![
                        opt(icons::bin_s(ITEM_SIZE)).on_press(msg(ListMessage::Delete(i))),
                        opt(icons::arrow_up_s(ITEM_SIZE)).on_press(msg(ListMessage::ShiftUp(i))),
                        opt(icons::arrow_down_s(ITEM_SIZE))
                            .on_press(msg(ListMessage::ShiftDown(i))),
                        widget::text_input("Enter argument...", arg)
                            .size(ITEM_SIZE + 4)
                            .font(FONT_MONO)
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
        .push(row![get_args_list_add_button(msg)].spacing(10))
        .spacing(5)
        .width(Length::Fill)
        .into()
}

pub fn args_split_by_space(split: bool) -> widget::Checkbox<'static, Message, LauncherTheme> {
    widget::checkbox("Split arguments by spaces", split)
        .style(|t: &LauncherTheme, s| t.style_checkbox(s, Some(Color::SecondLight)))
        .size(12)
        .text_size(12)
        .on_toggle(|t| Message::EditInstance(EditInstanceMessage::ToggleSplitArg(t)))
}

fn get_args_list_add_button(
    msg: impl Fn(ListMessage) -> Message + Clone + 'static,
) -> widget::Button<'static, Message, LauncherTheme> {
    widget::button(
        row![icons::new_s(13), widget::text("Add").size(13)]
            .align_y(Alignment::Center)
            .spacing(8)
            .padding([1, 2]),
    )
    .style(move |t: &LauncherTheme, s| t.style_button(s, StyleButton::Round))
    .on_press(msg(ListMessage::Add))
}
