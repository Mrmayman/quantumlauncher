Plans for the future of QuantumLauncher.

# Platforms

| Architectures | Windows | macOS | Linux | FreeBSD |
|:---:|:---:|:---:|:---:|:---:|
| x86_64        | ✅      | 🟢    | ✅    | 🟠¹     |
| i686          | 🟡      |      | 🔴    | 🔴      |
| aarch64       | 🟡      | 🟢    | 🟢    | 🔴      |
| arm32         |        |      | 🟠    | 🔴      |

- ✅: **Perfect** support
- 🟢: **Decent** support
- 🟡: **Unstable**, may have bugs
- 🟠: 1.16.5 and below
- 🟠¹: 1.12.2 and below
- 🔴: Not supported

> Not all platforms are well-supported due to lack of
> development resources. If you can, consider
> contributing and helping out.
> 
> Bug reports, PRs, anything is welcome!

Future plans:

- Haiku
- Solaris
- Redox OS
- Android (unlikely)

# Instances

- Import MultiMC/PrismLauncher instances
- Migrate from other launchers
- Package QuantumLauncher instances (WIP)
- Upgrading instances to a newer Minecraft version

# Mods

- Filters in Mod store
- OptiFabric support
- Modpack UI/UX improvements

# Misc

- Integrate Java 25 (massive performance improvements)
- Full controller, keyboard-navigation support in UI
- Plugin system in lua ([abandoned implementation here](https://github.com/Mrmayman/quantumlauncher/blob/16e02b1e36a736fadb3214b84de908eb21635a55/plugins/README.md), scrapped due to complexity)

---

# Servers

The server manager is highly incomplete and under
active development, so it's temporarily disabled.

This will allow you to setup and host Minecraft servers
across the internet from a single click. Think Aternos
but local and ad-free.

- [x] Create/delete/run Minecraft servers
- [x] Editing basic server settings (RAM, Java, Args)
- [ ] Editing `server.properties`
- [ ] Editing NBT config files
- [ ] Plugin store
- [ ] [playit.gg](https://playit.gg) integration
- [ ] Version-control based world rollback system
- [ ] Detect `world/session.lock` error and auto-fix it

## Loaders

- [x] Paper
- [ ] Spigot
- [ ] Bukkit
- [ ] Bungeecoord
- [ ] [Combining mod-loaders and plugin-loaders](https://github.com/LeStegii/server-software/blob/master/java/MODS+PLUGINS.md)

---

# Command-Line interface

- [x] `list-instances`, `-l`
- [x] `list-available-versions`, `-a`
- [x] `create NAME VERSION`
- [x] `launch INSTANCE USERNAME`
- [ ] `loader install/uninstall/info`
- [ ] Mod installation features from CLI
- [ ] Preset, modpack features from CLI
