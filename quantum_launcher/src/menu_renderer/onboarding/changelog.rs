use iced::widget;

use crate::menu_renderer::{onboarding::IMG_MANAGE_MODS, Element};

#[allow(unused)]
pub fn changelog<'a>() -> Element<'a> {
    const FS: u16 = 14;

    widget::column![
        widget::text("Welcome to QuantumLauncher v0.4.3!").size(40),

        widget::text("You can now install OptiFine and Forge together!"),

        widget::column![
            widget::text("Added alternate fabric implementations for versions without official Fabric support:"),
            widget::text("- Legacy Fabric (1.3-1.13)").size(14),
            widget::text("- OrnitheMC (b1.7-1.13)").size(14),
            widget::text("- Babric and Cursed Legacy (b1.7.3)").size(14),
        ].spacing(5),

        widget::horizontal_rule(1),
        widget::text("UX").size(32),

        widget::column![
        "- Export mods as a shareable text list with optional links!",
        "- Write instance-specific notes for coordinates, todo lists, etc!",
        "- Many UI tweaks everywhere",
        ].spacing(5),

        widget::text("Themes").size(32),
        widget::column![
            widget::text("- Added Auto light/dark mode (syncs with system)"),
            widget::text("- Added themes:"),
            widget::text("    - \"Adwaita\" greyish theme (GNOME-inspired)").size(14),
            widget::text("    - \"Halloween\" orange/amber theme (thanks @Sreehari425)").size(14),
        ].spacing(5),

        widget::text("Create Instance").size(32),
        widget::column![
            widget::text("Overhauled the Create Instance screen, now with:"),
            widget::text("- Sidebar to view versions clearer"),
            widget::text("- Filters for release/snapshot/beta/... (thanks @Sreehari425)"),
            widget::text("- Search bar"),
            widget::text("- Auto-filling version and name by default"),
        ],

        widget::text("Mod Menu").size(32),
        widget::column![
            widget::text("Overhauled the mod menu, now with:"),
            widget::text("- Icons and Search!"),
            widget::text("- Easy bulk-selection (ctrl-a, shift/ctrl+click)"),
            widget::text("- Better aesthetics and layout"),
            widget::text("Also:"),
            widget::text("- Added EXPERIMENTAL importing of MultiMC/PrismLauncher instances"),
            widget::text("- Added option to include/exclude configuration in mod presets (thanks @Sreehari425)"),
        ].spacing(5),

        widget::image(IMG_MANAGE_MODS.clone()).height(400),

        widget::horizontal_rule(1),
        widget::text("Keyboard Navigation").size(32),

        widget::column![
            "- \"Ctrl/Cmd/Alt 1/2/3\" to switch tabs in main screen",
            "- \"Ctrl N\" to create new instance",
            "- \"Ctrl ,\" to open settings",
        ].spacing(5),

        widget::horizontal_rule(1),
        widget::text("Technical").size(32),
        widget::column![
            widget::text("- Added pre-launch prefix commands (eg: `prime-run`, `mangohud`, `gamemoderun`, etc)"),
            widget::text("- Added global Java arguments"),
            widget::text("- Added custom jar override support"),
            widget::text("- File location on linux has moved from `~/.config` to `~/.local/share` (with auto-migration)"),
            widget::text("- Added option to redownload libraries and assets"),
            widget::text("- Added warning for mistakenly downloading Windows 32-bit build"),
        ].spacing(5),

        widget::horizontal_rule(1),
        widget::text("Fixes").size(32),
        widget::column![
            widget::text("- Colored terminal output on Windows.").size(14),
            widget::text("- CurseForge mods without a loader can now be installed.").size(14),
            widget::text("- Instances from newer launcher versions can be opened in v0.4.1.").size(14),
            widget::text("- Backspace no longer kills running instances without Ctrl.").size(14),
            widget::text("- Added warning if xrandr isn't installed").size(14),
            widget::text("- Improved ARM support for Linux and macOS, for 1.21 and above").size(14),
            widget::Space::with_height(5),
            widget::text("- Fixed the game log being a single-line mess.").size(14),
            widget::text("- Fixed \"java binary not found\" macOS error.").size(14),
            widget::text("- Fixed crash with \"Better Discord Rich Presence\" mod.").size(14),
            widget::text("- Fixed launcher panic when launching the game.").size(14),
            widget::text("- Fixed NeoForge 1.21.1 and Forge 1.21.5 crash (reinstall loader to apply)").size(14),
            widget::text("- Fixed forge installer error: \"Processor failed, invalid outputs\"").size(14),
            widget::text("- Fixed \"SSLHandshakeException\" crash on Windows.").size(14),
            widget::text("- Fixed wrong link used for \"Open Website\" in auto-update screen.").size(14),
        ].spacing(5),

        widget::Space::with_height(10),
        widget::container(widget::text("By the way, I've been busy with my life a lot lately.\nSorry for the lack of features.").size(12)).padding(10),
        widget::Space::with_height(10),
        widget::text("Ready to experience your new launcher now? Hit continue!").size(20),
    ]
    .padding(10)
    .spacing(10)
    .into()
}
