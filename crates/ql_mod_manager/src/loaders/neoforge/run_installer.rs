use std::{path::Path, sync::mpsc::Sender};

use ql_core::{no_window, pt, GenericProgress, IntoIoError, CLASSPATH_SEPARATOR};
use ql_java_handler::{get_java_binary, JavaVersion};
use tokio::process::Command;

use crate::loaders::{
    forge::{ForgeInstallError, ForgeInstallProgress},
    neoforge::{send_progress, INSTALLER_NAME},
    FORGE_INSTALLER_JAVA,
};

pub async fn compile_and_run_installer(
    neoforge_dir: &Path,
    j_progress: Option<&Sender<GenericProgress>>,
    f_progress: Option<&Sender<ForgeInstallProgress>>,
    is_server: bool,
) -> Result<(), ForgeInstallError> {
    send_progress(f_progress, ForgeInstallProgress::P4CompilingInstaller);

    write_source_file(neoforge_dir, is_server).await?;
    compile_installer(neoforge_dir, f_progress, j_progress).await?;
    run_installer(neoforge_dir, f_progress, is_server).await?;

    Ok(())
}

async fn run_installer(
    neoforge_dir: &Path,
    f_progress: Option<&Sender<ForgeInstallProgress>>,
    is_server: bool,
) -> Result<(), ForgeInstallError> {
    pt!("Running Installer");
    send_progress(f_progress, ForgeInstallProgress::P5RunningInstaller);

    let java_path = get_java_binary(JavaVersion::Java21, "java", None).await?;
    let mut command = Command::new(&java_path);
    no_window!(command);
    command
        .args([
            "-cp",
            &format!(
                "forge/{INSTALLER_NAME}{CLASSPATH_SEPARATOR}{INSTALLER_NAME}{CLASSPATH_SEPARATOR}forge/{CLASSPATH_SEPARATOR}."
            ),
            "ForgeInstaller",
        ])
        .current_dir(if is_server {
            neoforge_dir
                .parent()
                .map_or(neoforge_dir.join(".."), |n| n.to_owned())
        } else {
            neoforge_dir.to_owned()
        });

    let output = command.output().await.path(java_path)?;
    if !output.status.success() {
        return Err(ForgeInstallError::InstallerError(
            String::from_utf8(output.stdout)?,
            String::from_utf8(output.stderr)?,
        ));
    }
    Ok(())
}

async fn compile_installer(
    neoforge_dir: &Path,
    f_progress: Option<&Sender<ForgeInstallProgress>>,
    j_progress: Option<&Sender<GenericProgress>>,
) -> Result<(), ForgeInstallError> {
    pt!("Compiling Installer");
    send_progress(f_progress, ForgeInstallProgress::P4CompilingInstaller);
    let javac_path = get_java_binary(JavaVersion::Java21, "javac", j_progress).await?;
    let mut command = Command::new(&javac_path);
    command
        .args(["-cp", INSTALLER_NAME, "ForgeInstaller.java", "-d", "."])
        .current_dir(neoforge_dir);
    no_window!(command);

    let output = command.output().await.path(javac_path)?;
    if !output.status.success() {
        return Err(ForgeInstallError::CompileError(
            String::from_utf8(output.stdout)?,
            String::from_utf8(output.stderr)?,
        ));
    }
    Ok(())
}

async fn write_source_file(neoforge_dir: &Path, is_server: bool) -> Result<(), ForgeInstallError> {
    let java_source_file = FORGE_INSTALLER_JAVA
        .replace("CLIENT", if is_server { "SERVER" } else { "CLIENT" })
        .replace("new File(\".\")", "new File(\".\"), a -> true");
    let source_path = neoforge_dir.join("ForgeInstaller.java");
    tokio::fs::write(&source_path, java_source_file)
        .await
        .path(source_path)?;
    Ok(())
}
