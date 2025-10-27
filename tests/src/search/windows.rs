use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM},
    UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId},
};

pub fn search_for_window(pid: u32, sys: &sysinfo::System) -> bool {
    let all_windows = get_all_windows_with_pids();
    let wins: Vec<HWND> = all_windows
        .into_iter()
        .filter(|&(_, window_pid)| window_pid == pid)
        .map(|(hwnd, _)| hwnd)
        .collect();

    if wins.is_empty() {
        return false;
    }

    for (proc_pid, proc) in sys.processes() {
        if proc_pid.as_u32() == pid {
            _ = proc.kill_and_wait();
            return true;
        }
    }
    false
}

fn get_all_windows_with_pids() -> Vec<(HWND, u32)> {
    let mut windows_info = Vec::new();
    let ptr = &mut windows_info as *mut _;
    let l_param = ptr as isize;

    // Safety: The pointer passed as l_param is valid for the lifetime of the call.
    unsafe {
        EnumWindows(Some(enum_windows_callback), LPARAM(l_param))
            .expect("Failed to enumerate windows");
    }
    windows_info
}

extern "system" fn enum_windows_callback(hwnd: HWND, l_param: LPARAM) -> BOOL {
    let window_pids = unsafe { &mut *(l_param.0 as *mut Vec<(HWND, u32)>) };

    let mut process_id: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id as *mut u32));
    }

    if process_id != 0 {
        window_pids.push((hwnd, process_id));
    }

    BOOL(1) // continue enumeration
}
