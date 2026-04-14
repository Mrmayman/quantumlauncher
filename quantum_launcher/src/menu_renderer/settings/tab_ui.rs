use iced::{
    Alignment,
    widget::{self, column, row},
};

use crate::{
    config::LauncherConfig,
    menu_renderer::{
        Column, checkered_list, get_mode_selector, settings::get_theme_selector, tsubtitle,
    },
    state::{LauncherSettingsMessage, MenuLauncherSettings, Message},
    stylesheet::styles::LauncherTheme,
};

const SETTING_WIDTH: u16 = 180;

impl MenuLauncherSettings {
    pub(super) fn view_ui_tab<'a>(&'a self, config: &'a LauncherConfig) -> Column<'a> {
        let ui_scale_apply = row![
            widget::horizontal_space(),
            widget::button(widget::text("Apply").size(12))
                .padding([1.8, 5.0])
                .on_press(Message::LauncherSettings(
                    LauncherSettingsMessage::UiScaleApply,
                ))
        ];

        let idle_fps = config.c_idle_fps();

        checkered_list([
            column![widget::text("User Interface").size(20)],

            column![
                widget::row!["Mode: ", get_mode_selector(config)]
                    .spacing(5)
                    .align_y(Alignment::Center),
                widget::Space::with_height(5),
                widget::row!["Theme:", get_theme_selector().wrap()].spacing(5),
            ]
            .spacing(5),
            column![row![
                widget::row![widget::text!("UI Scale ({:.2}x)  ", self.temp_scale).size(15)]
                    .push_maybe(
                        ((self.temp_scale - config.ui_scale.unwrap_or(1.0)).abs() > 0.01)
                            .then_some(ui_scale_apply)
                    )
                    .align_y(Alignment::Center).width(SETTING_WIDTH),
                widget::slider(0.5..=3.0, self.temp_scale, |n| Message::LauncherSettings(
                    LauncherSettingsMessage::UiScale(n)
                ))
                .step(0.1),
            ]
            .align_y(Alignment::Center)
            .spacing(5)],

            get_ui_opacity(config),

            column![
                // TODO: This requires launcher restart
                // widget::checkbox("Custom Window Decorations", !config.c_window_decorations()).on_toggle(|n| {
                //     LauncherSettingsMessage::ToggleWindowDecorations(n).into()
                // }),
                // widget::text("Use custom window borders and close/minimize/maximize buttons").size(12),
                // widget::Space::with_height(5),

                widget::checkbox("Antialiasing (UI) - Requires Restart", config.ui_antialiasing.unwrap_or(true))
                    .on_toggle(|n| Message::LauncherSettings(
                        LauncherSettingsMessage::ToggleAntialiasing(n)
                    )),
                widget::text("Makes text/menus crisper. Also nudges the launcher into using your dedicated GPU for the User Interface")
                    .size(12).style(tsubtitle),
                widget::Space::with_height(5),

                widget::checkbox("Remember window size", config.window.as_ref().is_none_or(|n| n.save_window_size))
                    .on_toggle(|n| LauncherSettingsMessage::ToggleWindowSize(n).into()),
                widget::Space::with_height(5),
                widget::checkbox("Remember last selected instance", config.persistent.clone().unwrap_or_default().selected_remembered)
                    .on_toggle(|n| LauncherSettingsMessage::ToggleInstanceRemembering(n).into()),
            ]
            .spacing(5),

            column![
                row![
                    widget::text!("UI Idle FPS ({idle_fps})")
                        .size(15)
                        .width(SETTING_WIDTH),
                    widget::slider(2.0..=20.0, idle_fps as f64, |n| Message::LauncherSettings(
                        LauncherSettingsMessage::UiIdleFps(n)
                    ))
                    .step(1.0).shift_step(1.0),
                ]
                .align_y(Alignment::Center)
                .spacing(5),
                widget::text(r#"(Default: 6) Reduces resource usage when launcher is idle.
Only increase if progress bars stutter or "not responding" dialogs show"#).size(12).style(tsubtitle),
            ].spacing(5)
        ])
    }
}

fn get_ui_opacity(config: &LauncherConfig) -> widget::Column<'static, Message, LauncherTheme> {
    let ui_opacity = config.c_ui_opacity();
    let t = |t| widget::text(t).size(12).style(tsubtitle);

    widget::column![
        widget::row![
            widget::text!("Window Opacity ({ui_opacity:.2}x)")
                .width(SETTING_WIDTH)
                .size(15),
            widget::slider(0.5..=1.0, ui_opacity, |n| Message::LauncherSettings(
                LauncherSettingsMessage::UiOpacity(n)
            ))
            .step(0.1)
        ]
        .spacing(5)
        .align_y(Alignment::Center),
        t("Window background transparency\n(May not work on all systems/GPUs)"),
        t("0.5 (translucent) ..  1.0 (opaque)"),
    ]
    .spacing(5)
}
