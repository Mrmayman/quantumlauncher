<!-- <div align="center"> -->
<img align="left" style="width: 140px" width="140" src="https://github.com/Mrmayman/quantumlauncher/raw/main/assets/icon/ql_logo.png" />

<b>QuantumLauncher</b> ([Website](https://mrmayman.github.io/quantumlauncher) | [Discord](https://discord.gg/bWqRaSXar5) | [Changelogs](https://github.com/Mrmayman/quantumlauncher/tree/main/changelogs/))

![GPL3 License](https://img.shields.io/github/license/Mrmayman/quantumlauncher)
![Downloads](https://img.shields.io/github/downloads/Mrmayman/quantumlauncher/total)
[![Made with iced](https://iced.rs/badge.svg)](https://github.com/iced-rs/iced)
[![Discord Online](https://img.shields.io/discord/1280474064540012619?label=&labelColor=6A7EC2&logo=discord&logoColor=ffffff&color=7389D8)](https://discord.gg/bWqRaSXar5)
[![Matrix Server](https://img.shields.io/matrix/quantumgroup:matrix.org)](https://matrix.to/#/#quantumgroup:matrix.org)

A simple, powerful, cross platform Minecraft launcher.
<br><br>

![Quantum Launcher running RL Craft modpack](https://github.com/Mrmayman/quantumlauncher/raw/main/quantum_launcher.png)

# Features

| | |
|---|:--|
| <img src="https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/lightweight.png"> | <h2>Incredibly Lightweight</h2> Uses minimal CPU and RAM! Lighter than the vanilla launcher, common alternatives and even Task Manager! |
| <img src="https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/mod_store.png"> | <h2>Built-in mod store</h2> Install your favorite <b>mods</b> and <b>mod loaders</b> from the comfort of one window.<br><br>Isolate your game versions with instances, so clashes never happen! |
| <img src="https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/old_mc.png"> | <h2>Support for old Minecraft versions (via Omniarchive)</h2> Includes skin and sound fixes, and adds rare versions to the list |
| <img src="https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/mod_manage.png"> | Manage hundreds of mods conveniently!<br>Package and share them with friends. |

# Downloads and Building

Download stable versions from [the website](https://mrmayman.github.io/quantumlauncher/#downloads), or from [Releases](http://github.com/Mrmayman/quantumlauncher/releases/latest)

Or, compile the launcher to get the latest experimental version:

```sh
git clone https://github.com/Mrmayman/quantumlauncher.git
cd quantum-launcher
cargo run --release
```

You can omit the `--release` flag for faster compile times, but slightly worse performance and MUCH larger build file
size.

# Why QuantumLauncher?

- QuantumLauncher provides a feature rich, flexible, simple
  and lightweight experience with plenty of modding features.

What about the others? Well...

- The official Minecraft launcher is slow, unstable, buggy and frustrating to use,
  with barely any modding features
- Legacy Launcher lacks *many* features
- TLauncher is suspected to be malware

# File Locations

- **Windows**: `C:/Users/YOUR_USERNAME/AppData/Roaming/QuantumLauncher/`
  - You probably won't see the `AppData` folder (hidden). Press Windows + R and paste this path, and hit enter
- **macOS**: `/Users/YOURNAME/Library/Application Support/QuantumLauncher/`
- **Linux/BSD**: `~/.local/share/QuantumLauncher/` (`~` refers to your home directory)

Structure:

- Instances located at `QuantumLauncher/instances/YOUR_INSTANCE/`
  - `.minecraft` located at `YOUR_INSTANCE/.minecraft/`
- Logs in `QuantumLauncher/logs/`

<br>

# More info

- **MSRV** (Minimum Supported Rust Version): `1.88.0`
  - Any mismatch is considered a bug, please report if found
- [**Roadmap/Plans**](docs/ROADMAP.md)
- [**Contributing**](CONTRIBUTING.md)
- [**Test Suite**](tests/README.md)

# Licensing and Credits

- Most of this launcher is licensed under the **GNU General Public License v3**
- Some assets have additional licensing ([more info](assets/README.md))

> Many parts of the launcher were inspired by
> <https://github.com/alexivkin/minecraft-launcher/>.
> Massive shoutout!

# Notes

This launcher supports offline mode, but it's at your own risk.
I am not responsible for any issues caused.
You should buy the game, but if you can't, feel free to use this launcher
until you eventually get the means (like me).

If anyone has any issues/complaints, just open an issue in the repo.
