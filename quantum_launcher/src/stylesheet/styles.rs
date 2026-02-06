use std::{fmt::Display, str::FromStr};

use iced::widget::container::Style;
use iced::widget::scrollable::Rail;
use iced::{widget, Border};
use ql_core::err;
use serde::{Deserialize, Serialize};

use crate::stylesheet::color::{ADWAITA_DARK, ADWAITA_LIGHT};

use super::{
    color::{Color, BROWN, CATPPUCCIN, HALLOWEEN, PURPLE, SKY_BLUE, TEAL},
    widgets::{IsFlat, StyleButton, StyleScrollable},
};

pub const BORDER_WIDTH: f32 = 1.0;
pub const BORDER_RADIUS: f32 = 8.0;

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum LauncherThemeColor {
    Brown,
    #[serde(rename = "Sky Blue")]
    SkyBlue,
    Catppuccin,
    Teal,
    Halloween,
    Adwaita,
    #[default]
    #[serde(other)]
    Purple,
}

impl LauncherThemeColor {
    pub const ALL: &'static [Self] = &[
        Self::Purple,
        Self::Brown,
        Self::SkyBlue,
        Self::Catppuccin,
        Self::Teal,
        Self::Halloween,
        Self::Adwaita,
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
                LauncherThemeColor::Halloween => "Halloween",
                LauncherThemeColor::Adwaita => "Adwaita",
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
            "Halloween" => LauncherThemeColor::Halloween,
            "Adwaita" => LauncherThemeColor::Adwaita,
            _ => {
                err!("Unknown style: {s:?}");
                LauncherThemeColor::default()
            }
        })
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug, PartialEq, Eq)]
pub enum LauncherThemeLightness {
    Light,
    Dark,
    #[default]
    #[serde(other)]
    Auto,
}

impl LauncherThemeLightness {
    pub const ALL: &[Self] = &[Self::Light, Self::Dark, Self::Auto];
}
impl Display for LauncherThemeLightness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LauncherThemeLightness::Light => "Light",
                LauncherThemeLightness::Dark => "Dark",
                LauncherThemeLightness::Auto => "Auto",
            },
        )
    }
}

#[derive(Clone, Default, Debug)]
pub struct LauncherTheme {
    pub lightness: LauncherThemeLightness,
    pub color: LauncherThemeColor,
    pub alpha: f32,
    pub system_dark_mode: bool,
}

impl LauncherTheme {
    pub fn is_light(&self) -> bool {
        match self.lightness {
            LauncherThemeLightness::Light => true,
            LauncherThemeLightness::Dark => false,
            LauncherThemeLightness::Auto => !self.system_dark_mode,
        }
    }

    pub fn get(&self, color: Color) -> iced::Color {
        let (palette, color) = self.get_base(color);
        palette.get(color)
    }

    fn get_base(&self, mut color: Color) -> (&super::color::Palette, Color) {
        if self.is_light() {
            if let Color::ExtraDark = color {
                color = Color::Dark;
            } else if let Color::Dark = color {
                color = Color::ExtraDark;
            }
        }

        if let LauncherThemeColor::Adwaita = self.color {
            (
                if self.is_light() {
                    &ADWAITA_LIGHT
                } else {
                    &ADWAITA_DARK
                },
                color,
            )
        } else {
            (
                self.get_palette(),
                if self.is_light() {
                    color.invert()
                } else {
                    color
                },
            )
        }
    }

    fn get_palette(&self) -> &super::color::Palette {
        match self.color {
            LauncherThemeColor::Brown => &BROWN,
            LauncherThemeColor::Purple => &PURPLE,
            LauncherThemeColor::SkyBlue => &SKY_BLUE,
            LauncherThemeColor::Catppuccin => &CATPPUCCIN,
            LauncherThemeColor::Teal => &TEAL,
            LauncherThemeColor::Halloween => &HALLOWEEN,
            LauncherThemeColor::Adwaita => unreachable!(),
        }
    }

    pub fn get_bg(&self, color: Color) -> iced::Background {
        let (palette, color) = self.get_base(color);
        palette.get_bg(color)
    }

    pub fn get_border(&self, color: Color) -> Border {
        let (palette, color) = self.get_base(color);
        palette.get_border(color)
    }

    fn get_border_sharp(&self, color: Color) -> Border {
        let (palette, color) = self.get_base(color);
        Border {
            color: palette.get(color),
            width: 0.0,
            radius: 0.0.into(),
        }
    }

    fn get_border_style(&self, style: &impl IsFlat, color: Color) -> Border {
        let sides = style.get_4_sides();
        if sides.into_iter().any(|n| n) {
            let mut b = self.get_border(color);
            b.radius = iced::border::Radius {
                top_left: radius(sides[0]),
                top_right: radius(sides[1]),
                bottom_right: radius(sides[2]),
                bottom_left: radius(sides[3]),
            };
            b
        } else if style.is_flat() {
            self.get_border_sharp(color)
        } else {
            self.get_border(color)
        }
    }

    fn style_scrollable_active(&self, style: StyleScrollable) -> widget::scrollable::Style {
        let border = self.get_border_style(
            &style,
            match style {
                StyleScrollable::Round | StyleScrollable::FlatDark => Color::SecondDark,
                StyleScrollable::FlatExtraDark => Color::Dark,
            },
        );
        let rail = Rail {
            background: Some(self.get_bg(Color::ExtraDark)),
            border,
            scroller: widget::scrollable::Scroller {
                color: self.get(Color::SecondDark),
                border: self.get_border_style(&style, Color::Mid),
            },
        };
        widget::scrollable::Style {
            container: Style {
                text_color: None,
                background: match style {
                    StyleScrollable::Round | StyleScrollable::FlatDark => None,
                    StyleScrollable::FlatExtraDark => Some(self.get_bg(Color::ExtraDark)),
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
                StyleScrollable::Round | StyleScrollable::FlatDark => Color::SecondDark,
                StyleScrollable::FlatExtraDark => Color::Dark,
            },
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
            background: Some(self.get_bg(Color::ExtraDark)),
            border,
            scroller: widget::scrollable::Scroller {
                color: if hovered {
                    self.get(Color::Mid)
                } else {
                    blend_colors(self.get(Color::SecondDark), self.get(Color::Mid))
                },
                border: self.get_border_style(&style, Color::Light),
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
        );
        let rail_v = Rail {
            background: Some(self.get_bg(Color::ExtraDark)),
            border,
            scroller: widget::scrollable::Scroller {
                color: if is_vertical_scrollbar_dragged {
                    self.get(Color::White)
                } else {
                    blend_colors(self.get(Color::Mid), self.get(Color::SecondDark))
                },
                border: self.get_border_style(&style, Color::Light),
            },
        };
        let rail_h = Rail {
            background: Some(self.get_bg(Color::Dark)),
            border,
            scroller: widget::scrollable::Scroller {
                color: self.get(if is_horizontal_scrollbar_dragged {
                    Color::White
                } else {
                    Color::Mid
                }),
                border: self.get_border_style(&style, Color::Light),
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
                StyleScrollable::FlatExtraDark => Some(self.get_bg(Color::ExtraDark)),
            },
            border,
            shadow: iced::Shadow::default(),
        }
    }

    pub fn style_rule(&self, color: Color, thickness: u16) -> widget::rule::Style {
        widget::rule::Style {
            color: self.get(color),
            width: thickness,
            radius: 0.into(),
            fill_mode: widget::rule::FillMode::Full,
        }
    }

    pub fn style_container_normal(&self) -> Style {
        Style {
            border: self.get_border(Color::SecondDark),
            ..Default::default()
        }
    }

    pub fn style_container_selected_flat_button(&self) -> Style {
        Style {
            border: self.get_border_sharp(Color::Mid),
            background: Some(self.get_bg(Color::SecondDark)),
            text_color: None,
            ..Default::default()
        }
    }

    pub fn style_container_selected_flat_button_semi(&self, radii: [bool; 4]) -> Style {
        Style {
            border: Border {
                radius: get_radius_semi(radii),
                width: 1.0,
                color: self.get(Color::SecondDark),
            },
            background: Some(self.get_bg(Color::Dark)),
            text_color: None,
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
                    color: self.get(Color::Mid),
                    width,
                    radius: radius.into(),
                }
            },
            background: Some(self.get_bg(color)),
            ..Default::default()
        }
    }

    pub fn style_container_bg_semiround(
        &self,
        radii: [bool; 4],
        color: Option<(Color, f32)>,
    ) -> Style {
        Style {
            border: {
                Border {
                    color: self.get(Color::Mid),
                    width: 0.0,
                    radius: get_radius_semi(radii),
                }
            },
            background: Some(
                color.map_or(self.get_bg_color(), |(c, a)| self.get_bg(c).scale_alpha(a)),
            ),
            ..Default::default()
        }
    }

    pub fn style_container_bg(&self, radius: f32, color: Option<Color>) -> Style {
        Style {
            border: {
                Border {
                    color: self.get(Color::Mid),
                    width: 0.0,
                    radius: radius.into(),
                }
            },
            background: Some(color.map_or(self.get_bg_color(), |n| self.get_bg(n))),
            ..Default::default()
        }
    }

    fn get_bg_color(&self) -> iced::Background {
        let c = if let LauncherThemeColor::Adwaita = self.color {
            self.get(Color::Dark)
        } else {
            blend_colors(self.get(Color::Dark), self.get(Color::ExtraDark))
        };
        iced::Background::Color(c.scale_alpha(self.alpha))
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
        let text_color = text_color.map(|n| self.get(n));
        match status {
            widget::checkbox::Status::Active { is_checked } => widget::checkbox::Style {
                background: if is_checked {
                    self.get_bg(Color::Light)
                } else {
                    self.get_bg(Color::Dark)
                },
                icon_color: if is_checked {
                    self.get(Color::Dark)
                } else {
                    self.get(Color::Light)
                },
                border: self.get_border(Color::Mid),
                text_color,
            },
            widget::checkbox::Status::Hovered { is_checked } => widget::checkbox::Style {
                background: if is_checked {
                    self.get_bg(Color::White)
                } else {
                    self.get_bg(Color::SecondDark)
                },
                icon_color: if is_checked {
                    self.get(Color::SecondDark)
                } else {
                    self.get(Color::White)
                },
                border: self.get_border(Color::Mid),
                text_color,
            },
            widget::checkbox::Status::Disabled { is_checked } => widget::checkbox::Style {
                background: if is_checked {
                    self.get_bg(Color::SecondLight)
                } else {
                    self.get_bg(Color::ExtraDark)
                },
                icon_color: if is_checked {
                    self.get(Color::ExtraDark)
                } else {
                    self.get(Color::SecondLight)
                },
                border: self.get_border(Color::SecondDark),
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
                    StyleButton::FlatDark
                    | StyleButton::RoundDark
                    | StyleButton::SemiDark(_)
                    | StyleButton::SemiDarkBorder(_) => Color::Dark,
                    StyleButton::FlatExtraDark | StyleButton::SemiExtraDark(_) => Color::ExtraDark,
                };
                widget::button::Style {
                    background: Some(self.get_bg(color)),
                    text_color: self.get(Color::White),
                    border: if let StyleButton::Round = style {
                        Border {
                            radius: BORDER_RADIUS.into(),
                            ..Default::default()
                        }
                    } else if let StyleButton::SemiDarkBorder(n) = style {
                        Border {
                            radius: get_radius_semi(n),
                            width: BORDER_WIDTH,
                            color: self.get(Color::SecondDark),
                        }
                    } else {
                        self.get_border_style(&style, color)
                    },
                    ..Default::default()
                }
            }
            widget::button::Status::Hovered => {
                let color = match style {
                    StyleButton::Round
                    | StyleButton::RoundDark
                    | StyleButton::Flat
                    | StyleButton::FlatDark
                    | StyleButton::SemiDark(_)
                    | StyleButton::SemiDarkBorder(_) => Color::Mid,
                    StyleButton::FlatExtraDark | StyleButton::SemiExtraDark(_) => Color::Dark,
                };
                widget::button::Style {
                    background: Some(self.get_bg(color)),
                    text_color: self.get(match style {
                        StyleButton::Round | StyleButton::Flat => Color::Dark,
                        _ => Color::White,
                    }),
                    border: self.get_border_style(&style, color),
                    ..Default::default()
                }
            }
            widget::button::Status::Pressed => widget::button::Style {
                background: Some(self.get_bg(Color::SecondLight)),
                text_color: self.get(Color::Dark),
                border: self.get_border_style(&style, Color::White),
                ..Default::default()
            },
            widget::button::Status::Disabled => {
                let color = match style {
                    StyleButton::Flat | StyleButton::Round | StyleButton::RoundDark => Color::Dark,
                    StyleButton::FlatDark
                    | StyleButton::SemiDark(_)
                    | StyleButton::SemiDarkBorder(_)
                    | StyleButton::SemiExtraDark(_) => Color::ExtraDark,
                    StyleButton::FlatExtraDark => Color::SecondDark,
                };
                widget::button::Style {
                    background: Some(self.get_bg(color)),
                    text_color: self.get(Color::ExtraDark),
                    border: if let StyleButton::Round = style {
                        let (palette, color) = self.get_base(Color::SecondDark);
                        iced::Border {
                            color: palette.get(color),
                            width: 0.5,
                            radius: BORDER_RADIUS.into(),
                        }
                    } else {
                        self.get_border_style(&style, color)
                    },
                    ..Default::default()
                }
            }
        }
    }

    pub fn style_radio(&self, status: widget::radio::Status, color: Color) -> widget::radio::Style {
        match status {
            widget::radio::Status::Active { is_selected } => widget::radio::Style {
                background: self.get_bg(Color::Dark),
                dot_color: self.get(if is_selected {
                    Color::SecondLight
                } else {
                    Color::ExtraDark
                }),
                border_width: BORDER_WIDTH,
                border_color: self.get(Color::SecondLight),
                text_color: Some(self.get(color)),
            },
            widget::radio::Status::Hovered { is_selected } => widget::radio::Style {
                background: self.get_bg(Color::Dark),
                dot_color: self.get(if is_selected {
                    Color::White
                } else {
                    Color::SecondDark
                }),
                border_width: BORDER_WIDTH,
                border_color: self.get(Color::SecondLight),
                text_color: Some(self.get(color)),
            },
        }
    }

    pub fn style_text(&self, color: Color) -> widget::text::Style {
        widget::text::Style {
            color: Some(self.get(color)),
        }
    }

    pub fn style_text_editor_box(
        &self,
        status: widget::text_editor::Status,
    ) -> widget::text_editor::Style {
        match status {
            widget::text_editor::Status::Active => widget::text_editor::Style {
                background: self.get_bg(Color::ExtraDark),
                border: self.get_border(Color::Dark),
                icon: self.get(Color::Light),
                placeholder: self.get(Color::Light),
                value: self.get(Color::White),
                selection: self.get(Color::Dark),
            },
            widget::text_editor::Status::Hovered => widget::text_editor::Style {
                background: self.get_bg(Color::ExtraDark),
                border: self.get_border(Color::SecondDark),
                icon: self.get(Color::Light),
                placeholder: self.get(Color::Light),
                value: self.get(Color::White),
                selection: self.get(Color::Dark),
            },
            widget::text_editor::Status::Focused => widget::text_editor::Style {
                background: self.get_bg(Color::ExtraDark),
                border: self.get_border(Color::SecondDark),
                icon: self.get(Color::Light),
                placeholder: self.get(Color::Light),
                value: self.get(Color::White),
                selection: self.get(Color::SecondDark),
            },
            widget::text_editor::Status::Disabled => widget::text_editor::Style {
                background: self.get_bg(Color::SecondDark),
                border: self.get_border(Color::Mid),
                icon: self.get(Color::Light),
                placeholder: self.get(Color::Light),
                value: self.get(Color::White),
                selection: self.get(Color::Dark),
            },
        }
    }

    pub fn style_text_editor_flat_extra_dark(
        &self,
        status: widget::text_editor::Status,
    ) -> widget::text_editor::Style {
        let border = Border {
            color: self.get(Color::ExtraDark),
            width: 0.0,
            radius: iced::border::Radius::new(0.0),
        };
        match status {
            widget::text_editor::Status::Active | widget::text_editor::Status::Hovered => {
                widget::text_editor::Style {
                    background: self.get_bg(Color::ExtraDark),
                    border,
                    icon: self.get(Color::Light),
                    placeholder: self.get(Color::Light),
                    value: self.get(Color::White),
                    selection: self.get(Color::Dark),
                }
            }
            widget::text_editor::Status::Focused => widget::text_editor::Style {
                background: self.get_bg(Color::ExtraDark),
                border,
                icon: self.get(Color::Light),
                placeholder: self.get(Color::Light),
                value: self.get(Color::White),
                selection: self.get(Color::SecondDark),
            },
            widget::text_editor::Status::Disabled => widget::text_editor::Style {
                background: self.get_bg(Color::ExtraDark),
                border,
                icon: self.get(Color::Light),
                placeholder: self.get(Color::Light),
                value: self.get(Color::SecondLight),
                selection: self.get(Color::Dark),
            },
        }
    }
}

fn get_radius_semi(radii: [bool; 4]) -> iced::border::Radius {
    let [tl, tr, bl, br] = radii;
    iced::border::Radius::new(0.0)
        .top_left(radius(tl))
        .top_right(radius(tr))
        .bottom_left(radius(bl))
        .bottom_right(radius(br))
}

fn radius(t: bool) -> f32 {
    if t {
        BORDER_RADIUS
    } else {
        0.0
    }
}

fn blend_colors(color1: iced::Color, color2: iced::Color) -> iced::Color {
    // Calculate the average of each RGBA component
    let r = (color1.r + color2.r) / 2.0;
    let g = (color1.g + color2.g) / 2.0;
    let b = (color1.b + color2.b) / 2.0;
    let a = (color1.a + color2.a) / 2.0;

    // Return a new Color with the blended RGBA values
    iced::Color::from_rgba(r, g, b, a)
}
