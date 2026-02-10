use cfg_if::cfg_if;

cfg_if!(
    if #[cfg(target_os = "windows")] {
        mod stub;
        pub use stub::*;
    } else if #[cfg(target_os = "macos")] {
        mod stub;
        pub use stub::*;
    } else if #[cfg(target_family = "unix")] {
        mod unix;
        pub use unix::*;
    } else {
        mod stub;
        pub use stub::*;
    }
);
