use std::{fmt::Display, path::PathBuf, process::exit};

use clap::Parser;
use ql_core::{do_jobs, err, print::LogConfig, InstanceSelection, ListEntry};
use ql_instances::DownloadError;

use crate::version::{Version, VERSIONS_LWJGL2, VERSIONS_LWJGL3};

mod launch;
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
    #[arg()]
    timeout: Option<f32>,
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
            err!("{err}");
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

    for Version(name, loaders) in cli.get_versions() {
        let instance = InstanceSelection::new(name, false);
        attempt(ql_mod_manager::loaders::uninstall_loader(instance.clone()).await);

        launch::launch((*name).to_owned(), cli.timeout.unwrap_or(60.0)).await;
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
