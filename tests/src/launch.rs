use std::{io::Write, time::Duration};

use ql_core::err;

use crate::search::search_for_window;
use crate::{attempt, set_terminal};

pub async fn launch(name: String, timeout: f32) -> bool {
    print!("Testing {name} ");
    _ = std::io::stdout().flush();
    let child = attempt(
        ql_instances::launch(
            name.clone(),
            "test".to_owned(),
            None,
            None,
            None,
            Vec::new(),
        )
        .await,
    );
    set_terminal(true);

    let Some(pid) = child.child.lock().unwrap().id() else {
        err!("{name}: No PID found");
        return false;
    };

    let timeout_duration = Duration::from_secs_f32(timeout);
    let start_time = tokio::time::Instant::now();

    let sys = sysinfo::System::new_all(); // sys needs to be mutable to refresh

    loop {
        if start_time.elapsed() >= timeout_duration {
            println!("Timeout reached!");
            break;
        }
        tokio::time::sleep(Duration::from_secs_f32(timeout / 30.0)).await;

        if search_for_window(pid, &sys) {
            return true;
        }

        print!(".");
        _ = std::io::stdout().flush();
    }

    err!("{name}: No window found after waiting");
    false
}
