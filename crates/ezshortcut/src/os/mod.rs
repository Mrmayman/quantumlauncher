use cfg_if::cfg_if;

cfg_if!(
    if #[cfg(target_os = "windows")] {
        mod windows;
        pub use windows::*;
        /// `.lnk`/`.app`/`.desktop` extension
        pub const EXTENSION: &str = ".lnk";
        /// `lnk`/`app`/`desktop` extension (no period)
        pub const EXTENSION_S: &str = "lnk";
    } else if #[cfg(target_os = "macos")] {
        mod macos;
        pub use macos::*;
        /// `.lnk`/`.app`/`.desktop` extension
        pub const EXTENSION: &str = ".app";
        /// `lnk`/`app`/`desktop` extension (no period)
        pub const EXTENSION_S: &str = "app";
    } else if #[cfg(target_family = "unix")] {
        mod unix;
        pub use unix::*;
        /// `.lnk`/`.app`/`.desktop` extension
        pub const EXTENSION: &str = ".desktop";
        /// `lnk`/`app`/`desktop` extension (no period)
        pub const EXTENSION_S: &str = "desktop";
    } else {
        mod stub;
        pub use stub::*;
        pub const EXTENSION: &str = "";
        pub const EXTENSION_S: &str = "";
    }
);
