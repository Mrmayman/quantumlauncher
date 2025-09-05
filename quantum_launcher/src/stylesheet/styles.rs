use std::{fmt::Display, str::FromStr};

use iced::widget::container::Style;
use iced::widget::scrollable::Rail;
use iced::{widget, Border};
use ql_core::err;

use super::{
    color::{Color, BROWN, CATPPUCCIN, PURPLE, SKY_BLUE, TEAL},
    widgets::{IsFlat, StyleButton, StyleScrollable},
};

pub const BORDER_WIDTH: f32 = 2.0;
pub const BORDER_RADIUS: f32 = 8.0;

#[derive(Copy, Clone, Debug, Default)]
pub enum LauncherThemeColor {
    Brown,
    #[default]
    Purple,
    SkyBlue,
    Catppuccin,
    Teal,
}

impl LauncherThemeColor {
    // HOOK: Add themes here
    pub const ALL: &'static [Self] = &[
        Self::Purple,
        Self::Brown,
        Self::SkyBlue,
        Self::Catppuccin,
        Self::Teal,
    ];
}

impl Display for LauncherThemeColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LauncherThemeColor::Brown => "Brown",
                LauncherThemeColor::Purple => "Purple",
                LauncherThemeColor::SkyBlue => "Sky Blue",
                LauncherThemeColor::Catppuccin => "Catppuccin",
                LauncherThemeColor::Teal => "Teal",
            },
        )
    }
}

impl FromStr for LauncherThemeColor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Brown" => LauncherThemeColor::Brown,
            "Purple" => LauncherThemeColor::Purple,
            "Sky Blue" => LauncherThemeColor::SkyBlue,
            "Catppuccin" => LauncherThemeColor::Catppuccin,
            "Teal" => LauncherThemeColor::Teal,
            _ => {
                err!("Unknown style: {s:?}");
                LauncherThemeColor::default()
            }
        })
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub enum LauncherThemeLightness {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Default, Debug)]
pub struct LauncherTheme {
    pub lightness: LauncherThemeLightness,
    pub color: LauncherThemeColor,
}

impl LauncherTheme {
    pub fn from_vals(color: LauncherThemeColor, lightness: LauncherThemeLightness) -> Self {
        Self { lightness, color }
    }

    pub fn get(&self, color: Color, invert: bool) -> iced::Color {
        let (palette, color) = self.get_base(invert, color);
        palette.get(color)
    }

    fn get_base(&self, invert: bool, mut color: Color) -> (&super::color::Palette, Color) {
        let palette = match self.color {
            LauncherThemeColor::Brown => &BROWN,
            LauncherThemeColor::Purple => &PURPLE,
            LauncherThemeColor::SkyBlue => &SKY_BLUE,
            LauncherThemeColor::Catppuccin => &CATPPUCCIN,
            LauncherThemeColor::Teal => &TEAL,
        };
        if let LauncherThemeLightness::Light = self.lightness {
            if let Color::ExtraDark = color {
                color = Color::Dark;
            } else if let Color::Dark = color {
                color = Color::ExtraDark;
            }
        }
        let color = if invert {
            match self.lightness {
                LauncherThemeLightness::Dark => color,
                LauncherThemeLightness::Light => color.invert(),
            }
        } else {
            match self.lightness {
                LauncherThemeLightness::Dark => color.invert(),
                LauncherThemeLightness::Light => color,
            }
        };
        (palette, color)
    }

    pub fn get_bg(&self, color: Color, invert: bool) -> iced::Background {
        let (palette, color) = self.get_base(invert, color);
        palette.get_bg(color)
    }

    pub fn get_border(&self, color: Color, invert: bool) -> Border {
        let (palette, color) = self.get_base(invert, color);
        palette.get_border(color)
    }

    fn get_border_sharp(&self, color: Color, invert: bool) -> Border {
        let (palette, color) = self.get_base(invert, color);
        Border {
            color: palette.get(color),
            width: 0.0,
            radius: 0.0.into(),
        }
    }

    fn get_border_style(&self, style: &impl IsFlat, color: Color, invert: bool) -> Border {
        if style.is_flat() {
            self.get_border_sharp(color, invert)
        } else {
            self.get_border(color, invert)
        }
    }

    fn style_scrollable_active(&self, style: StyleScrollable) -> widget::scrollable::Style {
        let border = self.get_border_style(
            &style,
            match style {
                StyleScrollable::Round | StyleScrollable::FlatDark => Color::SecondDark,
                StyleScrollable::FlatExtraDark => Color::Dark,
            },
            true,
        );
        let rail = Rail {
            background: Some(self.get_bg(Color::ExtraDark, true)),
            border,
            scroller: widget::scrollable::Scroller {
                color: self.get(Color::SecondDark, true),
                border: self.get_border_style(&style, Color::Mid, true),
            },
        };
        widget::scrollable::Style {
            container: Style {
                text_color: None,
                background: match style {
                    StyleScrollable::Round | StyleScrollable::FlatDark => None,
                    StyleScrollable::FlatExtraDark => Some(self.get_bg(Color::ExtraDark, true)),
                },
                border,
                shadow: iced::Shadow::default(),
            },
            gap: None,
            vertical_rail: rail,
            horizontal_rail: rail,
        }
    }

    fn style_scrollable_hovered(
        &self,
        style: StyleScrollable,
        is_vertical_scrollbar_hovered: bool,
        is_horizontal_scrollbar_hovered: bool,
    ) -> widget::scrollable::Style {
        let border = self.get_border_style(
            &style,
            match style {
                StyleScrollable::Round => Color::Mid,
                StyleScrollable::FlatDark => Color::SecondDark,
                StyleScrollable::FlatExtraDark => Color::Dark,
            },
            true,
        );
        let vertical_rail = self.s_scrollable_rail(style, border, is_vertical_scrollbar_hovered);
        let horizontal_rail =
            self.s_scrollable_rail(style, border, is_horizontal_scrollbar_hovered);
        widget::scrollable::Style {
            container: self.s_scrollable_get_container(style, border),
            vertical_rail,
            horizontal_rail,
            gap: None,
        }
    }

    fn s_scrollable_rail(&self, style: StyleScrollable, border: Border, hovered: bool) -> Rail {
        Rail {
            background: Some(self.get_bg(Color::ExtraDark, true)),
            border,
            scroller: widget::scrollable::Scroller {
                color: self.get(if hovered { Color::Light } else { Color::Mid }, true),
                border: self.get_border_style(&style, Color::Light, true),
            },
        }
    }

    fn style_scrollable_dragged(
        &self,
        style: StyleScrollable,
        is_vertical_scrollbar_dragged: bool,
        is_horizontal_scrollbar_dragged: bool,
    ) -> widget::scrollable::Style {
        let border = self.get_border_style(
            &style,
            match style {
                StyleScrollable::Round => Color::Mid,
                StyleScrollable::FlatDark => Color::SecondDark,
                StyleScrollable::FlatExtraDark => Color::Dark,
            },
            true,
        );
        let rail_v = Rail {
            background: Some(self.get_bg(Color::ExtraDark, true)),
            border,
            scroller: widget::scrollable::Scroller {
                color: self.get(
                    if is_vertical_scrollbar_dragged {
                        Color::White
                    } else {
                        Color::Mid
                    },
                    true,
                ),
                border: self.get_border_style(&style, Color::Light, true),
            },
        };
        let rail_h = Rail {
            background: Some(self.get_bg(Color::Dark, true)),
            border,
            scroller: widget::scrollable::Scroller {
                color: self.get(
                    if is_horizontal_scrollbar_dragged {
                        Color::White
                    } else {
                        Color::Mid
                    },
                    true,
                ),
                border: self.get_border_style(&style, Color::Light, true),
            },
        };
        widget::scrollable::Style {
            container: self.s_scrollable_get_container(style, border),
            vertical_rail: rail_v,
            horizontal_rail: rail_h,
            gap: None,
        }
    }

    fn s_scrollable_get_container(&self, style: StyleScrollable, border: Border) -> Style {
        Style {
            text_color: None,
            background: match style {
                StyleScrollable::Round | StyleScrollable::FlatDark => None,
                StyleScrollable::FlatExtraDark => Some(self.get_bg(Color::ExtraDark, true)),
            },
            border,
            shadow: iced::Shadow::default(),
        }
    }

    pub fn style_rule(&self, color: Color, thickness: u16) -> widget::rule::Style {
        widget::rule::Style {
            color: self.get(color, true),
            width: thickness,
            radius: 0.into(),
            fill_mode: widget::rule::FillMode::Full,
        }
    }

    pub fn style_container_normal(&self) -> Style {
        Style {
            border: self.get_border(Color::SecondDark, true),
            ..Default::default()
        }
    }

    pub fn style_container_selected_flat_button(&self) -> Style {
        Style {
            border: self.get_border_sharp(Color::Mid, true),
            background: Some(self.get_bg(Color::SecondDark, true)),
            text_color: Some(self.get(Color::White, true)),
            ..Default::default()
        }
    }

    pub fn style_container_sharp_box(&self, width: f32, color: Color) -> Style {
        self.style_container_round_box(width, color, 0.0)
    }

    pub fn style_container_round_box(&self, width: f32, color: Color, radius: f32) -> Style {
        Style {
            border: {
                Border {
                    color: self.get(Color::Mid, true),
                    width,
                    radius: radius.into(),
                }
            },
            background: Some(self.get_bg(color, true)),
            ..Default::default()
        }
    }

    pub fn style_scrollable_round(
        &self,
        status: widget::scrollable::Status,
    ) -> widget::scrollable::Style {
        self.style_scrollable(status, StyleScrollable::Round)
    }

    pub fn style_scrollable_flat_extra_dark(
        &self,
        status: widget::scrollable::Status,
    ) -> widget::scrollable::Style {
        self.style_scrollable(status, StyleScrollable::FlatExtraDark)
    }

    pub fn style_scrollable_flat_dark(
        &self,
        status: widget::scrollable::Status,
    ) -> widget::scrollable::Style {
        self.style_scrollable(status, StyleScrollable::FlatDark)
    }

    fn style_scrollable(
        &self,
        status: widget::scrollable::Status,
        style: StyleScrollable,
    ) -> widget::scrollable::Style {
        match status {
            widget::scrollable::Status::Active => self.style_scrollable_active(style),
            widget::scrollable::Status::Hovered {
                is_horizontal_scrollbar_hovered,
                is_vertical_scrollbar_hovered,
            } => self.style_scrollable_hovered(
                style,
                is_vertical_scrollbar_hovered,
                is_horizontal_scrollbar_hovered,
            ),
            widget::scrollable::Status::Dragged {
                is_horizontal_scrollbar_dragged,
                is_vertical_scrollbar_dragged,
            } => self.style_scrollable_dragged(
                style,
                is_vertical_scrollbar_dragged,
                is_horizontal_scrollbar_dragged,
            ),
        }
    }

    pub fn style_rule_default(&self) -> widget::rule::Style {
        self.style_rule(Color::SecondDark, 2)
    }

    pub fn style_checkbox(
        &self,
        status: widget::checkbox::Status,
        text_color: Option<Color>,
    ) -> widget::checkbox::Style {
        let text_color = text_color.map(|n| self.get(n, true));
        match status {
            widget::checkbox::Status::Active { is_checked } => widget::checkbox::Style {
                background: if is_checked {
                    self.get_bg(Color::Light, true)
                } else {
                    self.get_bg(Color::Dark, true)
                },
                icon_color: if is_checked {
                    self.get(Color::Dark, true)
                } else {
                    self.get(Color::Light, true)
                },
                border: self.get_border(Color::Mid, true),
                text_color,
            },
            widget::checkbox::Status::Hovered { is_checked } => widget::checkbox::Style {
                background: if is_checked {
                    self.get_bg(Color::White, true)
                } else {
                    self.get_bg(Color::SecondDark, true)
                },
                icon_color: if is_checked {
                    self.get(Color::SecondDark, true)
                } else {
                    self.get(Color::White, true)
                },
                border: self.get_border(Color::Mid, true),
                text_color,
            },
            widget::checkbox::Status::Disabled { is_checked } => widget::checkbox::Style {
                background: if is_checked {
                    self.get_bg(Color::SecondLight, true)
                } else {
                    self.get_bg(Color::ExtraDark, true)
                },
                icon_color: if is_checked {
                    self.get(Color::ExtraDark, true)
                } else {
                    self.get(Color::SecondLight, true)
                },
                border: self.get_border(Color::SecondDark, true),
                text_color,
            },
        }
    }

    pub fn style_button(
        &self,
        status: widget::button::Status,
        style: StyleButton,
    ) -> widget::button::Style {
        match status {
            widget::button::Status::Active => {
                let color = match style {
                    StyleButton::Round | StyleButton::Flat => Color::SecondDark,
                    StyleButton::FlatDark | StyleButton::RoundDark => Color::Dark,
                    StyleButton::FlatExtraDark => Color::ExtraDark,
                };
                widget::button::Style {
                    background: Some(self.get_bg(color, true)),
                    text_color: self.get(Color::White, true),
                    border: self.get_border_style(&style, color, true),
                    ..Default::default()
                }
            }
            widget::button::Status::Hovered => {
                let color = match style {
                    StyleButton::Round
                    | StyleButton::RoundDark
                    | StyleButton::Flat
                    | StyleButton::FlatDark => Color::Mid,
                    StyleButton::FlatExtraDark => Color::SecondDark,
                };
                widget::button::Style {
                    background: Some(self.get_bg(color, true)),
                    text_color: self.get(
                        match style {
                            StyleButton::Round | StyleButton::Flat => Color::Dark,
                            StyleButton::FlatDark
                            | StyleButton::RoundDark
                            | StyleButton::FlatExtraDark => Color::White,
                        },
                        true,
                    ),
                    border: self.get_border_style(&style, color, true),
                    ..Default::default()
                }
            }
            widget::button::Status::Pressed => widget::button::Style {
                background: Some(self.get_bg(Color::White, true)),
                text_color: self.get(Color::Dark, true),
                border: self.get_border_style(&style, Color::White, true),
                ..Default::default()
            },
            widget::button::Status::Disabled => {
                let color = match style {
                    StyleButton::Flat | StyleButton::Round | StyleButton::RoundDark => Color::Dark,
                    StyleButton::FlatDark | StyleButton::FlatExtraDark => Color::ExtraDark,
                };
                widget::button::Style {
                    background: Some(self.get_bg(color, true)),
                    text_color: self.get(Color::ExtraDark, true),
                    border: self.get_border_style(
                        &style,
                        match style {
                            StyleButton::Round => Color::SecondDark,
                            StyleButton::RoundDark
                            | StyleButton::Flat
                            | StyleButton::FlatDark
                            | StyleButton::FlatExtraDark => color,
                        },
                        true,
                    ),
                    ..Default::default()
                }
            }
        }
    }

    pub fn style_text(&self, color: Color) -> widget::text::Style {
        widget::text::Style {
            color: Some(self.get(color, true)),
        }
    }

    pub fn style_text_editor_box(
        &self,
        status: widget::text_editor::Status,
    ) -> widget::text_editor::Style {
        match status {
            widget::text_editor::Status::Active => widget::text_editor::Style {
                background: self.get_bg(Color::ExtraDark, true),
                border: self.get_border(Color::Dark, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::Light, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::Dark, true),
            },
            widget::text_editor::Status::Hovered => widget::text_editor::Style {
                background: self.get_bg(Color::ExtraDark, true),
                border: self.get_border(Color::SecondDark, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::Light, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::Dark, true),
            },
            widget::text_editor::Status::Focused => widget::text_editor::Style {
                background: self.get_bg(Color::Dark, true),
                border: self.get_border(Color::SecondDark, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::Light, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::SecondDark, true),
            },
            widget::text_editor::Status::Disabled => widget::text_editor::Style {
                background: self.get_bg(Color::SecondDark, true),
                border: self.get_border(Color::Mid, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::Light, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::Dark, true),
            },
        }
    }

    pub fn style_text_editor_flat_extra_dark(
        &self,
        status: widget::text_editor::Status,
    ) -> widget::text_editor::Style {
        let border = Border {
            color: self.get(Color::ExtraDark, true),
            width: 0.0,
            radius: iced::border::Radius::new(0.0),
        };
        match status {
            widget::text_editor::Status::Active | widget::text_editor::Status::Hovered => {
                widget::text_editor::Style {
                    background: self.get_bg(Color::ExtraDark, true),
                    border,
                    icon: self.get(Color::Light, true),
                    placeholder: self.get(Color::Light, true),
                    value: self.get(Color::White, true),
                    selection: self.get(Color::Dark, true),
                }
            }
            widget::text_editor::Status::Focused => widget::text_editor::Style {
                background: self.get_bg(Color::ExtraDark, true),
                border,
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::Light, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::SecondDark, true),
            },
            widget::text_editor::Status::Disabled => widget::text_editor::Style {
                background: self.get_bg(Color::ExtraDark, true),
                border,
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::Light, true),
                value: self.get(Color::SecondLight, true),
                selection: self.get(Color::Dark, true),
            },
        }
    }
}
