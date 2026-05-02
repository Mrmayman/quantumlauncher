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
icon_define!(clone, '\u{e905}');
icon_define!(close, '\u{e906}');
icon_define!(compass, '\u{e907}');
icon_define!(cross, '\u{e908}');
icon_define!(deselectall, '\u{e909}');
icon_define!(discord, '\u{e90a}');
icon_define!(arrow_down, '\u{e90b}');
icon_define!(download, '\u{e90c}');
icon_define!(edit, '\u{e90d}');
icon_define!(fav, '\u{e90e}');
icon_define!(file, '\u{e90f}');
icon_define!(file_download, '\u{e910}');
icon_define!(file_gear, '\u{e911}');
icon_define!(file_info, '\u{e912}');
icon_define!(file_jar, '\u{e913}');
icon_define!(file_zip, '\u{e914}');
icon_define!(filter, '\u{e915}');
icon_define!(floppydisk, '\u{e916}');
icon_define!(folder, '\u{e917}');
icon_define!(gear, '\u{e918}');
icon_define!(github, '\u{e919}');
icon_define!(globe, '\u{e91a}');
icon_define!(hammer, '\u{e91b}');
icon_define!(layers, '\u{e91c}');
icon_define!(lines, '\u{e91d}');
icon_define!(matrix, '\u{e91e}');
icon_define!(maximize, '\u{e91f}');
icon_define!(minimize, '\u{e920}');
icon_define!(mode_dark, '\u{e921}');
icon_define!(mode_light, '\u{e922}');
icon_define!(new, '\u{e923}');
icon_define!(options, '\u{e924}');
icon_define!(paintbrush, '\u{e925}');
icon_define!(pin, '\u{e926}');
icon_define!(play, '\u{e927}');
icon_define!(qm, '\u{e928}');
icon_define!(refresh, '\u{e929}');
icon_define!(search, '\u{e92a}');
icon_define!(selectall, '\u{e92b}');
icon_define!(server, '\u{e92c}');
icon_define!(shortcut, '\u{e92d}');
icon_define!(sort, '\u{e92e}');
icon_define!(sort_ascend, '\u{e92f}');
icon_define!(sort_descend, '\u{e930}');
icon_define!(toggleoff, '\u{e931}');
icon_define!(toggleon, '\u{e932}');
icon_define!(tweak, '\u{e933}');
icon_define!(types, '\u{e934}');
icon_define!(unfav, '\u{e935}');
icon_define!(arrow_up, '\u{e936}');
icon_define!(upload, '\u{e937}');
icon_define!(version_cancel, '\u{e938}');
icon_define!(version_download, '\u{e939}');
icon_define!(version_tick, '\u{e93a}');
icon_define!(version_warn, '\u{e93b}');
icon_define!(warn, '\u{e93c}');
icon_define!(winsize, '\u{e93d}');
icon_define!(wrench, '\u{e93e}');

