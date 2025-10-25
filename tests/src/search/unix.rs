use ql_core::err;

pub fn search_for_window(pid: u32, sys: &sysinfo::System) -> bool {
    match duct::cmd("xdotool", &["search", "--pid", pid.to_string().as_str()])
        .stdout_capture()
        .run()
    {
        Ok(n) => {
            if kill_proc(pid, sys, n) {
                println!();
                return true;
            }
        }

        Err(_) => match duct::cmd("xdotool", ["search", "--classname", "Minecraft*"])
            .stdout_capture()
            .run()
        {
            Ok(n) => {
                if kill_proc(pid, sys, n) {
                    println!();
                    return true;
                }
            }
            Err(err) if err.to_string().contains("exited with code") => {}
            Err(err) => {
                err!("{err:?}");
            }
        },
    }
    false
}

fn kill_proc(pid: u32, sys: &sysinfo::System, n: std::process::Output) -> bool {
    if String::from_utf8_lossy(&n.stdout)
        .lines()
        .map(|n| n.trim())
        .any(|n| !n.is_empty())
    {
        for (proc_pid, proc) in sys.processes() {
            if proc_pid.as_u32() == pid {
                proc.kill();
                return true;
            }
        }
    }
    false
}
