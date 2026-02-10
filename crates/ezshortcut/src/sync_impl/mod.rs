use cfg_if::cfg_if;

cfg_if!(
    if #[cfg(target_os = "windows")] {
        mod windows;
        pub use windows::*;
    } else if #[cfg(target_os = "macos")] {
        mod macos;
        pub use macos::*;
    } else if #[cfg(target_family = "unix")] {
        mod unix;
        pub use unix::*;
    } else {
        mod stub;
        pub use stub::*;
    }
);
