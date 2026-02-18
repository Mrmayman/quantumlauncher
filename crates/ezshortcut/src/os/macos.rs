use tokio::{fs, process::Command};

use crate::{make_filename_safe, Shortcut};
use std::path::{Path, PathBuf};

pub fn get_menu_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("Applications"))
}

fn refresh_applications() {
    tokio::task::spawn(async {
        _ = Command::new("mdimport").arg("/Applications/").spawn();
    });
}

pub async fn create(shortcut: &Shortcut, path: impl AsRef<Path>) -> std::io::Result<()> {
    create_inner(shortcut, path.as_ref()).await?;
    Ok(())
}

async fn create_inner(shortcut: &Shortcut, path: &Path) -> std::io::Result<PathBuf> {
    let path = if path.extension().is_some_and(|n| n == ".app") {
        path.to_owned()
    } else {
        path.join(shortcut.get_filename())
    };

    fs::create_dir_all(&path).await?;
    let contents = path.join("Contents");
    let exec_dir = contents.join("MacOS");
    fs::create_dir_all(&contents).await?;
    fs::create_dir_all(&exec_dir).await?;
    fs::create_dir_all(&contents.join("Resources")).await?;

    let exec_path = exec_dir.join("ql_shortcut");

    let script = format!(
        r#"#!/usr/bin/env sh

exec {:?} {}
"#,
        shortcut.exec,
        shortcut.get_formatted_args()
    );
    fs::write(&exec_path, &script).await?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&exec_path, std::fs::Permissions::from_mode(0o755)).await?;

    let info_plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
    "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>ql_shortcut</string>
    <key>CFBundleIdentifier</key>
    <string>io.github.Mrmayman.QLShortcut_{}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
"#,
        shortcut.name,
        make_filename_safe(&shortcut.name, true),
    );
    // TODO: Before the </dict> you could have
    // ```
    // <key>CFBundleIconFile</key>
    // <string>ql_logo</string>
    // ```
    // with icon in Contents/Resources/ql_logo.icns
    // May need iconutil and sips for this

    let info_path = contents.join("Info.plist");
    fs::write(&info_path, &info_plist).await?;

    _ = Command::new("xattr").arg("-rc").arg(&path).spawn();

    Ok(path)
}

pub async fn create_in_applications(shortcut: &Shortcut) -> std::io::Result<()> {
    let path = Path::new("/Applications");
    let shortcut_path = create_inner(shortcut, path).await?;
    _ = Command::new("open").arg("-R").arg(&shortcut_path).spawn();
    refresh_applications();
    Ok(())
}
