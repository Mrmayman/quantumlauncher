use std::ffi::c_void;

use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation::base::{CFRelease, CFTypeRef};
use core_foundation::dictionary::{CFDictionaryGetValue, CFDictionaryRef};
use core_foundation::number::{CFNumberGetValue, CFNumberRef, kCFNumberSInt32Type};
use core_graphics::window::{
    CGWindowListCopyWindowInfo, kCGWindowListOptionAll, kCGWindowNumber, kCGWindowOwnerPID,
};

use crate::search::kill_proc;

/// Returns `true` if a window owned by `pid` exists; if so, kills the process.
pub fn search_for_window(pid: u32, sys: &sysinfo::System) -> bool {
    let all_windows = get_all_windows_with_pids();

    let wins: Vec<u32> = all_windows
        .into_iter()
        .filter(|&(_, window_pid)| window_pid == pid)
        .map(|(window_id, _)| window_id)
        .collect();

    if wins.is_empty() {
        return false;
    }

    kill_proc(pid, sys);
    true
}

/// Enumerates every window currently known to the window server and returns
/// `(window_id, owning_pid)` pairs.
///
/// This is the direct analogue of the Windows `EnumWindows` +
/// `GetWindowThreadProcessId` combination.
fn get_all_windows_with_pids() -> Vec<(u32, u32)> {
    let mut windows_info: Vec<(u32, u32)> = Vec::new();

    // Safety: The CFArray returned by CGWindowListCopyWindowInfo follows the
    // "Copy" naming rule, so we own a reference and must release it. All
    // CFNumber / CFStringRef values inside the array are borrowed only for the
    // lifetime of the array and are read-only here.
    unsafe {
        let window_list = CGWindowListCopyWindowInfo(kCGWindowListOptionAll, 0);
        if window_list.is_null() {
            return windows_info;
        }

        let count = CFArrayGetCount(window_list);
        for i in 0..count {
            let dict = CFArrayGetValueAtIndex(window_list, i) as CFDictionaryRef;
            if dict.is_null() {
                continue;
            }

            let pid_ref = CFDictionaryGetValue(dict, kCGWindowOwnerPID as *const c_void);
            if pid_ref.is_null() {
                continue;
            }

            let mut process_id: i32 = 0;
            let ok = CFNumberGetValue(
                pid_ref as CFNumberRef,
                kCFNumberSInt32Type,
                &mut process_id as *mut i32 as *mut c_void,
            );
            if !ok || process_id == 0 {
                continue;
            }

            let wid_ref = CFDictionaryGetValue(dict, kCGWindowNumber as *const c_void);
            let mut window_id: u32 = 0;
            if !wid_ref.is_null() {
                let _ = CFNumberGetValue(
                    wid_ref as CFNumberRef,
                    kCFNumberSInt32Type,
                    &mut window_id as *mut u32 as *mut c_void,
                );
            }

            windows_info.push((window_id, process_id as u32));
        }

        CFRelease(window_list as CFTypeRef);
    }

    windows_info
}
