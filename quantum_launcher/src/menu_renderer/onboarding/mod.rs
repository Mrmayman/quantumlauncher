use std::sync::LazyLock;

use iced::widget::{self, column, image::Handle};

mod changelog;
mod welcome;

pub use changelog::changelog;

use crate::{menu_renderer::tsubtitle, state::Message, stylesheet::styles::LauncherTheme};

pub static IMG_MANAGE_MODS: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_bytes(include_bytes!("../../../../assets/screenshots/mod_manage.png").as_slice())
});
pub static IMG_LOGO: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_bytes(include_bytes!("../../../../assets/icon/ql_logo.png").as_slice())
});

pub fn x86_warning() -> iced::widget::Container<'static, Message, LauncherTheme> {
    widget::container(
        column![
            widget::text(
                "You downloaded the 32-bit version!\nYou may want the 64-bit version (x86_64)"
            )
            .style(tsubtitle)
            .size(14),
            widget::button("Open Website")
                .on_press(Message::CoreOpenLink(ql_core::WEBSITE.to_owned()))
        ]
        .align_x(iced::Alignment::Center)
        .padding(5)
        .spacing(5),
    )
}
