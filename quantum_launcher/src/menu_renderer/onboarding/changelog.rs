use iced::widget;

use crate::{
    config::LauncherConfig,
    menu_renderer::{
        get_mode_selector, onboarding::IMG_MANAGE_MODS, settings::get_theme_selector, Element,
    },
    state::Message,
};

#[allow(unused)]
pub fn changelog<'a>(config: &LauncherConfig) -> Element<'a> {
    const FS: u16 = 14;
    let cmd = if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    };

    widget::column![
        widget::text("Welcome to QuantumLauncher v0.5.0!").size(40),
        "Happy new year by the way!",

        get_mode_selector(config),

        widget::container(widget::column![
            "TLDR;",
            widget::text("- Mod loaders: OptiFine + Forge together, plus legacy Fabric support").size(14),
            widget::text("- UI & themes: major overhauls, polish, keyboard navigation and new themes").size(14),
            widget::text("- Experimental: early Server Manager and MultiMC/PrismLauncher import").size(14),
            widget::text("- Power user features, technical improvements").size(14),
            widget::text("- Lots of fixes and improvements").size(14),
        ].spacing(5)).padding(10),

        widget::text("Mod Loaders").size(32),
        widget::column![
            "- You can now install OptiFine and Forge together!",
            "- Added CLI commands to manage loaders: `quantum_launcher loader install/info/uninstall`",
            "Added alternate fabric implementations for versions without official Fabric support:",
            widget::text("- Legacy Fabric (1.3-1.13)").size(14),
            widget::text("- OrnitheMC (b1.7-1.13)").size(14),
            widget::text("- Babric and Cursed Legacy (b1.7.3)").size(14),
        ].spacing(5),

        widget::rule::horizontal(1),
        widget::text("UX").size(32),

        widget::column![
        "- Export mods as a shareable text list with optional links!",
        "- Write instance-specific notes for coordinates, todo lists, etc!",
        "- Many small UX improvements and polish",
        "- The launcher will now remember the last-selected instance (toggleable)",
        ].spacing(5),

        widget::text("Themes").size(32),
        widget::column![
            "- Added Auto light/dark mode (syncs with system)",
            "- Added themes:",
            widget::text("    - \"Adwaita\" greyish theme (GNOME-inspired)").size(14),
            widget::text("    - \"Halloween\" orange/amber theme (thanks @Sreehari425)").size(14),
        ].spacing(5),

        get_theme_selector(),

        widget::text("Create Instance").size(32),
        widget::column![
            "Overhauled the Create Instance screen, now with:",
            "- Sidebar to view versions",
            "- Filters for release/snapshot/beta/... (thanks @Sreehari425)",
            "- Search bar",
            "- Auto-filling version and name by default",
        ].spacing(5),

        widget::text("Mod Menu").size(32),
        widget::column![
            "Overhauled the mod menu, now with:",
            widget::text("- Icons and Search!").size(14),
            widget::text("- Easy bulk-selection (ctrl-a, shift/ctrl+click)").size(14),
            widget::text("- Better aesthetics and layout").size(14),
            "Also:",
            widget::text("- Drastically improved mod description page rendering in the store, now with fewer visual glitches/missing items").size(14),
            widget::text("- Added option to include/exclude configuration in mod presets (thanks @Sreehari425)").size(14),
        ].spacing(5),

        widget::image(IMG_MANAGE_MODS.clone()).height(400),

        widget::text("Keyboard Navigation").size(32),

        widget::column![
            widget::text!("- {cmd}/Alt + 1/2/3 to switch tabs in main screen").size(14),
            widget::text!("- {cmd} + , to open settings").size(14),

            widget::Space::with_height(5),
            "Creating instance:",
            widget::text!("- {cmd} + N to open screen").size(14),
            widget::text!("- {cmd} + F to search versions (and `Enter` to select)").size(14),
            widget::text!("- {cmd} + Enter to confirm").size(14),
        ].spacing(5),

        widget::rule::horizontal(1),
        widget::text("Experiments").size(32),

        widget::text("A few experimental features have been enabled for users to try out.\nPlease try them and report any bugs or feedback!"),

        widget::text("1) Server Manager").size(20),
        widget::column![
            widget::text("Enabled a *sneak-peek* of the server manager.\nYou can use it to create and host your own servers!"),

            widget::container(widget::column![
                widget::text("WARNING: Very buggy and incomplete").size(14),
                widget::text("You will face frustration if you try and daily-drive this right now").size(14)
            ]).padding(10),

            widget::text("- Enable it through the CLI flag: --enable-server-manager").size(14),
            widget::rich_text![
                widget::span("- For others to join, you'll need "),
                widget::span("PLAYIT.GG").link(Message::CoreOpenLink("https://playit.gg".to_owned())),
                widget::span(" or port forwarding (no auto-setup yet)")
            ].size(14),
            widget::text("- Supports mod loaders, mod store, sending commands, viewing logs").size(14),
            widget::text("- More features coming soon!").size(14),
        ].spacing(5),

        widget::text("2) Importing from MultiMC").size(20),
        widget::column![
            widget::text("You can now import `.zip` instances from MultiMC and PrismLauncher!"),
            widget::text("- Enable it through the CLI flag: --enable-mmc-import").size(14),
            widget::text("- Go to New instance > Import from MultiMC").size(14),
            widget::text("- Still experimental, may not work for all instances").size(14),
            widget::text("- If you find incompatibilities, please report them on GitHub/Discord! (upload your instance if possible)").size(14),
        ].spacing(5),

        widget::rule::horizontal(1),
        widget::text("Technical").size(32),
        widget::column![
            widget::text("- Added pre-launch prefix commands (eg: `prime-run`, `mangohud`, `gamemoderun`, etc)").size(14),
            widget::text("- Added global Java arguments and prefixes").size(14),
            widget::text("- Added custom jar override and custom main class options").size(14),
            widget::text("- File location on linux has moved from `~/.config` to `~/.local/share` (with auto-migration)").size(14),
            widget::text("- Added option to redownload libraries and assets").size(14),
            widget::text("- Added warning for mistakenly downloading Windows 32-bit build").size(14),
            widget::text("- Added option to input arguments with spaces").size(14),
        ].spacing(5),

        widget::rule::horizontal(1),
        widget::text("Fixes").size(32),
        widget::column![
            widget::text("- CurseForge mods without a loader can now be installed").size(14),
            widget::text("- Instances from newer launcher versions can be opened in v0.4.1").size(14),
            widget::text("- Backspace no longer kills running instances without Ctrl").size(14),
            widget::Space::with_height(5),
            widget::text("- Fixed the game log being a single-line mess").size(14),
            widget::text("- Fixed crash with \"Better Discord Rich Presence\" mod").size(14),
            widget::text("- Fixed launcher panic when launching the game").size(14),
            widget::text("- Fixed NeoForge 1.21.1 and Forge 1.21.5 crash (reinstall loader to apply)").size(14),
            widget::text("- Fixed forge installer error: \"Processor failed, invalid outputs\"").size(14),
            widget::text("- Fixed wrong link used for \"Open Website\" in auto-update screen").size(14),
            widget::text("- Fixed confusing error on entering wrong password in ely.by login").size(14),
            widget::Space::with_height(5),
            "Platform:",
            widget::text("- Added warning if xrandr isn't installed").size(14),
            widget::text("- Added colored terminal output for Windows").size(14),
            widget::text("- Improved ARM support for Linux and macOS, for 1.21 and above").size(14),
            widget::text("- Fixed \"java binary not found\" macOS error").size(14),
            widget::text("- Fixed \"SSLHandshakeException\" crash on Windows").size(14),
            widget::text("- Fixed some versions (eg: 1.12.2) being undownloadable on macOS").size(14)
        ].spacing(5),

        widget::Space::with_height(10),
        widget::container(widget::text("By the way, I've been busy with my life a lot lately.\nSorry for delaying many promised features.").size(12)).padding(10),
        widget::Space::with_height(10),
        widget::text("Ready to experience your new launcher now? Hit continue!").size(20),
    ]
    .padding(10)
    .spacing(10)
    .into()
}
