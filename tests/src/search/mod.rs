#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::search_for_window;

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
mod unix;
#[cfg(all(target_family = "unix", not(target_os = "macos")))]
pub use unix::search_for_window;

#[cfg(target_os = "macos")]
pub fn search_for_window(pid: u32, sys: &sysinfo::System) -> bool {
    todo!("macOS support not added yet to test suite!")
}
