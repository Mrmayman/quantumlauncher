use std::process::ExitStatus;
use std::{io::Write, time::Duration};

use ql_core::read_log::Diagnostic;
use ql_core::{err, IntoStringError};

use crate::{attempt, search::search_for_window, set_terminal, Cli};

pub async fn launch(name: String, timeout: f32, cli: &Cli) -> bool {
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

    let Some(pid) = child.child.lock().await.id() else {
        err!("{name}: No PID found");
        return false;
    };
    let verbose = cli.verbose;
    let handle = tokio::task::spawn(async move {
        child
            .read_logs(Vec::new(), (!verbose).then(|| std::sync::mpsc::channel().0))
            .await
    });
    // Ok((ExitStatus::default(), instance, None))

    let timeout_duration = Duration::from_secs_f32(timeout);
    let start_time = tokio::time::Instant::now();

    let sys = sysinfo::System::new_all();

    loop {
        if start_time.elapsed() >= timeout_duration {
            println!("Timeout reached!");
            break;
        }
        tokio::time::sleep(Duration::from_secs_f32(timeout / 30.0)).await;

        if handle.is_finished() {
            return handle_process_exit(handle).await;
        }

        if search_for_window(pid, &sys) {
            return true;
        }

        print!(".");
        _ = std::io::stdout().flush();
    }

    err!("{name}: No window found after waiting");
    false
}

type ProcessExitResult = Option<
    Result<
        (ExitStatus, ql_core::InstanceSelection, Option<Diagnostic>),
        ql_core::read_log::ReadError,
    >,
>;

async fn handle_process_exit(handle: tokio::task::JoinHandle<ProcessExitResult>) -> bool {
    let out = handle.await;
    match out
        .strerr()
        .map(|n| {
            n.expect("stdout/stderr should exist, unless you turned switched off logging")
                .strerr()
        })
        .flatten()
    {
        Ok((code, _, diag)) => {
            if let Some(Diagnostic::MacOSPixelFormat) = diag {
                println!("\nmacOS VM lacks GPU acceleration, test successful");
                return true;
            } else if let Some(diag) = diag {
                err!("\nProcess exited (code: {code})\n    {diag}");
            } else if code.success() {
                err!("\nProcess exited somehow!");
            } else {
                err!("\nProcess exited (code: {code})");
            }
        }
        Err(err) => {
            err!("Instance child process: {err}");
        }
    }
    return false;
}
