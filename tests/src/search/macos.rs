use core_foundation::{array::CFArray, base::TCFType, dictionary::CFDictionary, number::CFNumber};
use core_graphics::window::{kCGNullWindowID, kCGWindowListOptionAll, CGWindowListCopyWindowInfo};

use crate::search::kill_proc;
use std::ffi::c_void;

pub fn search_for_window(pid: u32, sys: &sysinfo::System) -> bool {
    // SAFETY: Quartz returns a retained CFArray
    let window_list =
        unsafe { CGWindowListCopyWindowInfo(kCGWindowListOptionAll, kCGNullWindowID) };

    if window_list.is_null() {
        return false;
    }

    // Wrap raw CFArrayRef
    let windows = unsafe { CFArray::<CFDictionary>::wrap_under_create_rule(window_list) };

    for window in windows.iter() {
        // kCGWindowOwnerPID is a CFString constant
        let pid_value =
            window.find(unsafe { core_graphics::window::kCGWindowOwnerPID as *const c_void });

        if let Some(pid_value) = pid_value {
            // Value is a CFNumber
            let pid_number: CFNumber =
                unsafe { CFNumber::wrap_under_get_rule((*pid_value) as *const _) };

            if let Some(pid_current) = pid_number.to_i32() {
                if pid_current as u32 == pid {
                    kill_proc(pid, sys);
                    return true;
                }
            }
        }
    }

    false
}
