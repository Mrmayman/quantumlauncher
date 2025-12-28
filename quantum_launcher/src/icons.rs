//! All the icons to be shown in the launcher's UI.
//! For example, play, delete, etc.
//!
//! The icons are designed by [Aurlt](https://github.com/Aurlt).
//!
//! # How this works
//! Internally, the icons are stored as a font,
//! where each character is an icon. When showing an
//! icon, a `widget::text` object is made with the icon font
//! and the special character that corresponds to the icon.

use crate::stylesheet::styles::LauncherTheme;
use paste::paste;

const ICON_FONT: iced::Font = iced::Font::with_name("QuantumLauncher");

pub fn icon<'a>(codepoint: char) -> iced::widget::Text<'a, LauncherTheme> {
    iced::widget::text(codepoint).font(ICON_FONT)
}

pub fn icon_with_size<'a>(codepoint: char, size: u16) -> iced::widget::Text<'a, LauncherTheme> {
    iced::widget::text(codepoint).font(ICON_FONT).size(size)
}

macro_rules! icon_define {
    ($name:ident, $unicode:expr) => {
        paste! {
            #[allow(dead_code)]
            pub fn $name<'a>() -> iced::widget::Text<'a, LauncherTheme> {
                icon($unicode)
            }

            #[allow(dead_code)]
            pub fn [<$name _s>]<'a>(size: u16) -> iced::widget::Text<'a, LauncherTheme> {
                icon_with_size($unicode, size)
            }
        }
    };
}

icon_define!(back, '\u{e900}');
icon_define!(bin, '\u{e901}');
icon_define!(chatbox, '\u{e902}');
icon_define!(checkmark, '\u{e903}');
icon_define!(clock, '\u{e904}');
icon_define!(close, '\u{e905}');
icon_define!(cross, '\u{e906}');
icon_define!(deselectall, '\u{e907}');
icon_define!(discord, '\u{e908}');
icon_define!(arrow_down, '\u{e909}');
icon_define!(download, '\u{e90a}');
icon_define!(edit, '\u{e90b}');
icon_define!(fav, '\u{e90c}');
icon_define!(file, '\u{e90d}');
icon_define!(file_download, '\u{e90e}');
icon_define!(file_gear, '\u{e90f}');
icon_define!(file_info, '\u{e910}');
icon_define!(file_jar, '\u{e911}');
icon_define!(file_zip, '\u{e912}');
icon_define!(filter, '\u{e913}');
icon_define!(floppydisk, '\u{e914}');
icon_define!(folder, '\u{e915}');
icon_define!(gear, '\u{e916}');
icon_define!(github, '\u{e917}');
icon_define!(globe, '\u{e918}');
icon_define!(lines, '\u{e919}');
icon_define!(maximize, '\u{e91a}');
icon_define!(minimize, '\u{e91b}');
icon_define!(mode_dark, '\u{e91c}');
icon_define!(mode_light, '\u{e91d}');
icon_define!(new, '\u{e91e}');
icon_define!(paintbrush, '\u{e91f}');
icon_define!(pin, '\u{e920}');
icon_define!(play, '\u{e921}');
icon_define!(qm, '\u{e922}');
icon_define!(refresh, '\u{e923}');
icon_define!(search, '\u{e924}');
icon_define!(selectall, '\u{e925}');
icon_define!(sort, '\u{e926}');
icon_define!(sort_ascend, '\u{e927}');
icon_define!(sort_descend, '\u{e928}');
icon_define!(toggleoff, '\u{e929}');
icon_define!(toggleon, '\u{e92a}');
icon_define!(tweak, '\u{e92b}');
icon_define!(unfav, '\u{e92c}');
icon_define!(arrow_up, '\u{e92d}');
icon_define!(upload, '\u{e92e}');
icon_define!(version_cancel, '\u{e92f}');
icon_define!(version_download, '\u{e930}');
icon_define!(version_tick, '\u{e931}');
icon_define!(version_warn, '\u{e932}');
icon_define!(warn, '\u{e933}');
icon_define!(win_size, '\u{e934}');
