<div align="center">

# <img src="https://github.com/Mrmayman/quantumlauncher/raw/main/assets/icon/ql_logo.png" style="height: 1.4em; vertical-align: middle;" /> QuantumLauncher

## [Website](https://mrmayman.github.io/quantumlauncher) | [Discord](https://discord.gg/bWqRaSXar5) | [Changelogs](https://github.com/Mrmayman/quantumlauncher/tree/main/changelogs/)

![GPL3 License](https://img.shields.io/github/license/Mrmayman/quantumlauncher)
![Downloads](https://img.shields.io/github/downloads/Mrmayman/quantumlauncher/total)
![Discord Online](https://img.shields.io/discord/1280474064540012619?label=&labelColor=6A7EC2&logo=discord&logoColor=ffffff&color=7389D8)
[![Made with iced](https://iced.rs/badge.svg)](https://github.com/iced-rs/iced)

A minimalistic Minecraft launcher for Windows, macOS and Linux.

![Quantum Launcher running a Minecraft Instance](https://github.com/Mrmayman/quantumlauncher/raw/main/quantum_launcher.png)

QuantumLauncher offers a lightweight and responsive experience.
It's designed to be simple and easy to use, with a focus on performance and features.

# Features

## Lightweight and responsive

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/lightweight.png)

## Install Fabric, Forge, NeoForge, Quilt, or OptiFine with ease

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/install_loader.png)

## Built-in mod store to download your favorite mods!

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/mod_store.png)

## Isolate your different game versions with instances!

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/new.png)

## Full support for old Minecraft versions (via Omniarchive). Includes skin and sound fixes!

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/old_mc.png)

## Manage your hundreds of mods conveniently!

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/mod_manage.png)

## Make your launcher yours

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/screenshots/themes.png)
<br><br>

</div>

# Downloads and Building

You can download the stable version from the website linked above, or from the *Releases* button

Or, you can compile the launcher to get the latest experimental version (with potentially broken and untested features).
To compile the launcher:

```sh
git clone https://github.com/Mrmayman/quantumlauncher.git
cd quantum-launcher
cargo run --release
```

You can omit the `--release` flag for faster compile times, but *slightly* worse performance and MUCH larger build file
size.

# Why QuantumLauncher?

- QuantumLauncher provides a feature rich, flexible, simple
  and lightweight experience with plenty of modding features.

What about the others? Well...

- The official Minecraft launcher is slow, unstable, buggy and frustrating to use,
  with barely any modding features.
- Prism Launcher is a great launcher overall, but it does not support
  offline accounts. Same for MultiMC.
- Legacy Launcher isn't as feature rich as this
- TLauncher is *suspected* to be malware

# File Locations

- On *Windows*, the launcher files are at `C:/Users/YOUR_USERNAME/AppData/Roaming/QuantumLauncher/`
  - You probably won't see the `AppData` folder. Press Windows + R and paste this path, and hit enter
- On *Linux*, the launcher files are at `~/.local/share/QuantumLauncher/`. (`~` refers to your home directory)

Structure:

- Launcher logs are located at `QuantumLauncher/logs/`
- Instances located at `QuantumLauncher/instances/YOUR_INSTANCE/`
- `.minecraft` located at `YOUR_INSTANCE/.minecraft/`

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
