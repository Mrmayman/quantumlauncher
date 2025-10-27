use std::{fmt::Display, path::PathBuf, process::exit};

use clap::Parser;
use ql_core::{do_jobs, print::LogConfig, InstanceSelection, ListEntry};
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
    #[arg(short, long)]
    existing: bool,
    #[arg(
        long,
        help = "Only tests legacy LWJGL2-based versions (1.12.2 and below)"
    )]
    skip_lwjgl3: bool,
    #[arg(long)]
    timeout: Option<f32>,
    #[arg(short, long)]
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
            eprintln!("\nERROR: {err}");
            exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    set_terminal(true);
    setup_dir();
    let cli = Cli::parse();

    if !cli.existing {
        attempt(
            do_jobs(
                cli.get_versions()
                    .map(|version| create_instance(version.0.to_owned())),
            )
            .await,
        );
    }

    let mut fails = Vec::new();

    for Version(name, loaders) in cli.get_versions() {
        let instance = InstanceSelection::new(name, false);
        attempt(ql_mod_manager::loaders::uninstall_loader(instance.clone()).await);
        set_terminal(cli.verbose);
        if !launch::launch((*name).to_owned(), cli.timeout.unwrap_or(60.0)).await {
            fails.push((*name, None));
        }
        for loader in *loaders {
            println!("(Loader: {loader:?})");
            attempt(
                ql_mod_manager::loaders::install_specified_loader(
                    instance.clone(),
                    *loader,
                    None,
                    None,
                )
                .await,
            );

            println!("Done");
            if !launch::launch((*name).to_owned(), cli.timeout.unwrap_or(60.0)).await {
                fails.push((*name, Some(*loader)));
            }
            set_terminal(cli.verbose);
            attempt(ql_mod_manager::loaders::uninstall_loader(instance.clone()).await);
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

fn setup_dir() {
    let new_dir = PathBuf::from(file!())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("QuantumLauncher");
    std::env::set_var("QL_DIR", new_dir);
}

async fn create_instance(version: String) -> Result<(), DownloadError> {
    match ql_instances::create_instance(
        version.clone(),
        ListEntry {
            name: version,
            is_classic_server: false,
        },
        None,
        false,
    )
    .await
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
