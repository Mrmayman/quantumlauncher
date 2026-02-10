use cfg_if::cfg_if;

cfg_if!(
    if #[cfg(target_os = "windows")] {
        mod windows;
        pub use windows::*;
        pub const EXTENSION: &str = ".lnk";
    } else if #[cfg(target_os = "macos")] {
        mod macos;
        pub use macos::*;
        pub const EXTENSION: &str = ".app";
    } else if #[cfg(target_family = "unix")] {
        mod unix;
        pub use unix::*;
        pub const EXTENSION: &str = ".desktop";
    } else {
        mod stub;
        pub use stub::*;
        pub const EXTENSION: &str = "";
    }
);
