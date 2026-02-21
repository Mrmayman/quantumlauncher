use iced::widget;

use super::{color::Color, styles::LauncherTheme};

#[derive(Default, Clone, Copy)]
#[allow(unused)]
pub enum StyleScrollable {
    #[default]
    Round,
    FlatExtraDark,
    FlatDark,
}

#[derive(Default, Clone, Copy)]
#[allow(unused)]
pub enum StyleButton {
    #[default]
    Round,
    RoundDark,
    Flat,
    FlatDark,
    FlatExtraDark,
    /// top right, top left,
    /// bottom right, bottom left
    SemiDark([bool; 4]),
    SemiDarkBorder([bool; 4]),
    SemiExtraDark([bool; 4]),
}

pub trait IsFlat {
    fn is_flat(&self) -> bool;
    fn get_4_sides(&self) -> [bool; 4] {
        [false; 4]
    }
}

impl IsFlat for StyleButton {
    fn is_flat(&self) -> bool {
        match self {
            StyleButton::Round | StyleButton::RoundDark => false,
            StyleButton::Flat
            | StyleButton::FlatDark
            | StyleButton::FlatExtraDark
            | StyleButton::SemiDark(_)
            | StyleButton::SemiDarkBorder(_)
            | Self::SemiExtraDark(_) => true,
        }
    }

    fn get_4_sides(&self) -> [bool; 4] {
        match self {
            StyleButton::Round
            | StyleButton::RoundDark
            | StyleButton::Flat
            | StyleButton::FlatDark
            | StyleButton::FlatExtraDark => [false; 4],
            StyleButton::SemiDark(n) | StyleButton::SemiDarkBorder(n) | Self::SemiExtraDark(n) => {
                *n
            }
        }
    }
}

impl IsFlat for StyleScrollable {
    fn is_flat(&self) -> bool {
        match self {
            Self::Round => false,
            Self::FlatExtraDark | Self::FlatDark => true,
        }
    }
}

impl widget::container::Catalog for LauncherTheme {
    type Class<'a> = widget::container::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> <Self as widget::container::Catalog>::Class<'a> {
        Box::new(Self::style_container_normal)
    }

    fn style(
        &self,
        style: &widget::container::StyleFn<'_, LauncherTheme>,
    ) -> widget::container::Style {
        style(self)
    }
}

// Uncomment this and comment the other impl below this
// to have a gradient skeumorphic look for the buttons
//
// I disabled this because even though it looks decent
// it doesn't fit with the rest of the launcher, and
// all the other widgets look bad with this skeumorphic
// aesthetic.
/*
impl widget::button::Catalog for LauncherTheme {
    type Class<'a> = StyleButton;

    fn active(&self, style: &Self::Class) -> widget::button::Style {
        let color = match style {
            StyleButton::Round | StyleButton::Flat => Color::SecondDark,
            StyleButton::FlatDark => Color::Dark,
            StyleButton::FlatExtraDark => Color::Black,
        };
        widget::button::Style {
            background: Some(if let StyleButton::Round = style {
                iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0)
                        .add_stop(0.0, self.get(Color::SecondDark))
                        .add_stop(1.0, self.get(Color::Mid)),
                ))
            } else {
                self.get_bg(color, true)
            }),
            text_color: self.get(Color::White),
            border: self.get_border_style(style, color, true),
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Class) -> widget::button::Style {
        let color = match style {
            StyleButton::Round | StyleButton::Flat => Color::Mid,
            StyleButton::FlatDark => Color::Mid,
            StyleButton::FlatExtraDark => Color::SecondDark,
        };
        widget::button::Style {
            background: Some(if let StyleButton::Round = style {
                iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0)
                        .add_stop(0.0, self.get(Color::Mid))
                        .add_stop(1.0, self.get(Color::SecondLight)),
                ))
            } else {
                self.get_bg(color, true)
            }),
            text_color: self.get(
                match style {
                    StyleButton::Round | StyleButton::Flat => Color::Dark,
                    StyleButton::FlatDark | StyleButton::FlatExtraDark => Color::White,
                },
                true,
            ),
            border: self.get_border_style(style, color, true),
            ..Default::default()
        }
    }

    fn pressed(&self, style: &Self::Class) -> widget::button::Style {
        widget::button::Style {
            background: Some(if let StyleButton::Round = style {
                iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0)
                        .add_stop(0.0, self.get(Color::SecondLight))
                        .add_stop(1.0, self.get(Color::Mid)),
                ))
            } else {
                self.get_bg(Color::White)
            }),
            text_color: self.get(Color::Dark),
            border: self.get_border_style(style, Color::White, true),
            ..Default::default()
        }
    }

    fn disabled(&self, style: &Self::Class) -> widget::button::Style {
        widget::button::Style {
            background: Some(self.get_bg(
                match style {
                    StyleButton::Round | StyleButton::Flat => Color::SecondDark,
                    StyleButton::FlatDark => Color::Dark,
                    StyleButton::FlatExtraDark => Color::Black,
                },
                true,
            )),
            text_color: self.get(Color::SecondLight),
            border: self.get_border_style(style, Color::SecondDark, true),
            ..Default::default()
        }
    }
}
*/

impl widget::button::Catalog for LauncherTheme {
    type Class<'a> = widget::button::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|n, status| n.style_button(status, StyleButton::default()))
    }

    fn style(
        &self,
        style: &widget::button::StyleFn<'_, LauncherTheme>,
        status: widget::button::Status,
    ) -> widget::button::Style {
        style(self, status)
    }
}

impl widget::text::Catalog for LauncherTheme {
    type Class<'a> = widget::text::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|n| n.style_text(Color::White))
    }

    fn style(&self, style_fn: &<Self as widget::text::Catalog>::Class<'_>) -> widget::text::Style {
        style_fn(self)
    }
}

impl widget::pick_list::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::pick_list::Catalog>::Class<'a> {}

    fn style(&self, (): &(), status: widget::pick_list::Status) -> widget::pick_list::Style {
        match status {
            widget::pick_list::Status::Active | widget::pick_list::Status::Opened => {
                widget::pick_list::Style {
                    text_color: self.get(Color::Light),
                    placeholder_color: self.get(Color::SecondLight),
                    handle_color: self.get(Color::Light),
                    background: self.get_bg(Color::Dark),
                    border: self.get_border(Color::SecondDark),
                }
            }
            widget::pick_list::Status::Hovered => widget::pick_list::Style {
                text_color: self.get(Color::Light),
                placeholder_color: self.get(Color::SecondLight),
                handle_color: self.get(Color::Light),
                background: self.get_bg(Color::SecondDark),
                border: self.get_border(Color::SecondDark),
            },
        }
    }
}

impl widget::overlay::menu::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::overlay::menu::Catalog>::Class<'a> {}

    fn style(&self, (): &()) -> iced::overlay::menu::Style {
        iced::overlay::menu::Style {
            text_color: self.get(Color::White),
            background: self.get_bg(Color::SecondDark),
            border: self.get_border(Color::Mid),
            selected_text_color: self.get(Color::Dark),
            selected_background: self.get_bg(Color::SecondLight),
        }
    }
}

impl widget::scrollable::Catalog for LauncherTheme {
    type Class<'a> = widget::scrollable::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> <Self as widget::scrollable::Catalog>::Class<'a> {
        Box::new(Self::style_scrollable_round)
    }

    fn style(
        &self,
        style: &widget::scrollable::StyleFn<'_, LauncherTheme>,
        status: widget::scrollable::Status,
    ) -> widget::scrollable::Style {
        style(self, status)
    }
}

impl widget::text_input::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::text_input::Catalog>::Class<'a> {}

    fn style(&self, (): &(), status: widget::text_input::Status) -> widget::text_input::Style {
        match status {
            widget::text_input::Status::Active => widget::text_input::Style {
                background: self.get_bg(Color::ExtraDark),
                border: self.get_border(Color::SecondDark),
                icon: self.get(Color::Light),
                placeholder: self.get(Color::Mid),
                value: self.get(Color::White),
                selection: self.get(Color::Light),
            },
            widget::text_input::Status::Hovered => widget::text_input::Style {
                background: self.get_bg(Color::Dark),
                border: self.get_border(Color::Mid),
                icon: self.get(Color::Light),
                placeholder: self.get(Color::Mid),
                value: self.get(Color::White),
                selection: self.get(Color::Light),
            },
            widget::text_input::Status::Focused => widget::text_input::Style {
                background: self.get_bg(Color::Dark),
                border: self.get_border(Color::SecondLight),
                icon: self.get(Color::Light),
                placeholder: self.get(Color::Mid),
                value: self.get(Color::White),
                selection: self.get(Color::Light),
            },
            widget::text_input::Status::Disabled => widget::text_input::Style {
                background: self.get_bg(Color::ExtraDark),
                border: self.get_border(Color::Dark),
                icon: self.get(Color::Light),
                placeholder: self.get(Color::Mid),
                value: self.get(Color::White),
                selection: self.get(Color::Light),
            },
        }
    }
}

impl widget::progress_bar::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::progress_bar::Catalog>::Class<'a> {}

    fn style(&self, (): &()) -> widget::progress_bar::Style {
        widget::progress_bar::Style {
            background: self.get_bg(Color::SecondDark),
            bar: self.get_bg(Color::Light),
            border: self.get_border(Color::Mid),
        }
    }
}

impl widget::slider::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::slider::Catalog>::Class<'a> {}

    fn style(&self, (): &(), status: widget::slider::Status) -> widget::slider::Style {
        match status {
            widget::slider::Status::Active => widget::slider::Style {
                rail: widget::slider::Rail {
                    backgrounds: (self.get_bg(Color::Mid), self.get_bg(Color::SecondDark)),
                    width: 6.0,
                    border: self.get_border(Color::SecondDark),
                },
                handle: widget::slider::Handle {
                    shape: widget::slider::HandleShape::Circle { radius: 6.0 },
                    background: self.get_bg(Color::SecondLight),
                    border_width: 2.0,
                    border_color: self.get(Color::Light),
                },
            },
            widget::slider::Status::Hovered => widget::slider::Style {
                rail: widget::slider::Rail {
                    backgrounds: (self.get_bg(Color::Light), self.get_bg(Color::Mid)),
                    width: 4.0,
                    border: self.get_border(Color::Mid),
                },
                handle: widget::slider::Handle {
                    shape: widget::slider::HandleShape::Circle { radius: 8.0 },
                    background: self.get_bg(Color::SecondLight),
                    border_width: 2.0,
                    border_color: self.get(Color::White),
                },
            },
            widget::slider::Status::Dragged => widget::slider::Style {
                rail: widget::slider::Rail {
                    backgrounds: (self.get_bg(Color::White), self.get_bg(Color::SecondDark)),
                    width: 6.0,
                    border: self.get_border(Color::Mid),
                },
                handle: widget::slider::Handle {
                    shape: widget::slider::HandleShape::Circle { radius: 12.0 },
                    background: self.get_bg(Color::White),
                    border_width: 2.0,
                    border_color: self.get(Color::White),
                },
            },
        }
    }
}

impl iced::application::DefaultStyle for LauncherTheme {
    fn default_style(&self) -> iced::application::Appearance {
        iced::application::Appearance {
            background_color: iced::Color::TRANSPARENT,
            text_color: self.get(Color::Light),
        }
    }
}

impl widget::checkbox::Catalog for LauncherTheme {
    type Class<'a> = widget::checkbox::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> <Self as widget::checkbox::Catalog>::Class<'a> {
        Box::new(|n, status| n.style_checkbox(status, None))
    }

    fn style(
        &self,
        s: &<Self as widget::checkbox::Catalog>::Class<'_>,
        status: widget::checkbox::Status,
    ) -> widget::checkbox::Style {
        s(self, status)
    }
}

impl widget::text_editor::Catalog for LauncherTheme {
    type Class<'a> = widget::text_editor::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> <Self as widget::text_editor::Catalog>::Class<'a> {
        Box::new(LauncherTheme::style_text_editor_box)
    }

    fn style(
        &self,
        class: &<Self as widget::text_editor::Catalog>::Class<'_>,
        status: widget::text_editor::Status,
    ) -> widget::text_editor::Style {
        class(self, status)
    }
}

impl widget::svg::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::svg::Catalog>::Class<'a> {}

    fn style(&self, (): &(), _: widget::svg::Status) -> widget::svg::Style {
        // Who hovers on an SVG image, huh?
        widget::svg::Style { color: None }
    }
}

impl widget::radio::Catalog for LauncherTheme {
    type Class<'a> = widget::radio::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> <Self as widget::radio::Catalog>::Class<'a> {
        Box::new(|l, s| l.style_radio(s, Color::SecondLight))
    }

    fn style(
        &self,
        c: &<Self as widget::radio::Catalog>::Class<'_>,
        status: widget::radio::Status,
    ) -> widget::radio::Style {
        c(self, status)
    }
}

impl widget::rule::Catalog for LauncherTheme {
    type Class<'a> = widget::rule::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(LauncherTheme::style_rule_default)
    }

    fn style(&self, style: &widget::rule::StyleFn<'_, LauncherTheme>) -> widget::rule::Style {
        style(self)
    }
}

impl widget::combo_box::Catalog for LauncherTheme {}

impl widget::pane_grid::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::pane_grid::Catalog>::Class<'a> {}

    fn style(
        &self,
        (): &<Self as widget::pane_grid::Catalog>::Class<'_>,
    ) -> widget::pane_grid::Style {
        widget::pane_grid::Style {
            hovered_region: widget::pane_grid::Highlight {
                background: self.get_bg(Color::ExtraDark),
                border: iced::Border::default(),
            },
            picked_split: widget::pane_grid::Line {
                color: self.get(Color::SecondLight),
                width: 2.0,
            },
            hovered_split: widget::pane_grid::Line {
                color: self.get(Color::Mid),
                width: 1.0,
            },
        }
    }
}
