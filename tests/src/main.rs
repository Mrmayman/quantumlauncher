use std::{fmt::Display, path::PathBuf, process::exit};

use clap::Parser;
use ql_core::{do_jobs, eeprintln, print::LogConfig, ListEntry, Loader, LAUNCHER_DIR};
use ql_instances::DownloadError;

use crate::version::{Version, VERSIONS_LWJGL2, VERSIONS_LWJGL3};

mod launch;
mod search;
mod version;

#[derive(clap::Parser)]
#[command(
    long_about = "A test suite that launches different versions of Minecraft with different mod loader configurations."
)]
#[command(author = "Mrmayman")]
struct Cli {
    #[arg(long)]
    #[arg(help = "Test a specific version")]
    specific: Option<String>,
    #[arg(short, long)]
    #[arg(help = "Whether to reuse existing test files instead of redownloading them")]
    existing: bool,
    #[arg(
        long,
        help = "Only tests legacy LWJGL2-based versions (1.12.2 and below)"
    )]
    skip_lwjgl3: bool,
    #[arg(long)]
    #[arg(help = "How long to wait for a window, per instance, before giving up (default: 60).")]
    timeout: Option<f32>,
    #[arg(short, long)]
    #[arg(help = "See all the logs to diagnose issues")]
    verbose: bool,
}

impl Cli {
    fn get_versions(&self) -> impl Iterator<Item = &Version> {
        VERSIONS_LWJGL2.iter().chain(
            (!self.skip_lwjgl3)
                .then_some(VERSIONS_LWJGL3.iter())
                .into_iter()
                .flatten(),
        )
    }
}

fn attempt<T, E: Display>(r: Result<T, E>) -> T {
    match r {
        Ok(n) => n,
        Err(err) => {
            eeprintln!("\nERROR: {err}");
            exit(1);
        }
    }
}

#[tokio::main]
#[allow(unreachable_code)]
async fn main() {
    set_terminal(true);
    setup_dir();
    let cli = Cli::parse();

    if !cli.existing {
        if let Some(name) = &cli.specific {
            let path = LAUNCHER_DIR.join("instances").join(name);
            _ = tokio::fs::remove_dir_all(&path).await;
            attempt(create_instance(name.to_owned()).await);
        } else {
            attempt(
                do_jobs(cli.get_versions().map(|version| async {
                    let path = LAUNCHER_DIR.join("instances").join(version.0);
                    _ = tokio::fs::remove_dir_all(&path).await;
                    create_instance(version.0.to_owned()).await?;
                    Ok::<(), DownloadError>(())
                }))
                .await,
            );
        }
    }

    #[cfg(any(
        feature = "simulate_linux_arm64",
        feature = "simulate_macos_arm64",
        feature = "simulate_linux_arm32",
    ))]
    return;

    let mut fails = Vec::new();

    if let Some(name) = &cli.specific {
        try_version(name, &[], &mut fails, &cli).await;
    } else {
        for Version(name, loaders) in cli.get_versions() {
            try_version(name, loaders, &mut fails, &cli).await;
        }
    }

    if !fails.is_empty() {
        println!("\nTEST FAILURES:");
        for (name, loader) in fails {
            if let Some(loader) = loader {
                println!("{name}: {loader:?}");
            } else {
                println!("{name} (vanilla)");
            }
        }
    }
}

async fn try_version<'a>(
    name: &'a str,
    loaders: &[Loader],
    fails: &mut Vec<(&'a str, Option<Loader>)>,
    cli: &Cli,
) {
    let instance = ql_core::InstanceSelection::new(name, false);
    attempt(ql_mod_manager::loaders::uninstall_loader(instance.clone()).await);
    set_terminal(cli.verbose);
    if !launch::launch(name.to_owned(), cli.timeout.unwrap_or(60.0), cli).await {
        fails.push((name, None));
    }
    for loader in loaders {
        println!("(Loader: {loader:?})");
        if let Err(err) =
            ql_mod_manager::loaders::install_specified_loader(instance.clone(), *loader, None, None)
                .await
        {
            eeprintln!("{err}");
            fails.push((name, Some(*loader)));
            continue;
        }

        println!("Done");
        if !launch::launch((*name).to_owned(), cli.timeout.unwrap_or(60.0), cli).await {
            fails.push((name, Some(*loader)));
        }
        set_terminal(cli.verbose);
        attempt(ql_mod_manager::loaders::uninstall_loader(instance.clone()).await);
    }
}

fn setup_dir() {
    let new_dir = PathBuf::from(file!())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("QuantumLauncher");
    let logs_dir = new_dir.join("logs");
    _ = std::fs::remove_dir_all(&logs_dir);
    unsafe {
        std::env::set_var("QL_DIR", new_dir);
    }
}

async fn create_instance(version: String) -> Result<(), DownloadError> {
    match ql_instances::create_instance(version.clone(), ListEntry::new(version), None, false).await
    {
        Ok(_) | Err(DownloadError::InstanceAlreadyExists(_)) => Ok(()),
        Err(err) => Err(err),
    }
}

fn set_terminal(terminal: bool) {
    ql_core::print::set_config(LogConfig {
        terminal,
        file: false,
    })
}
