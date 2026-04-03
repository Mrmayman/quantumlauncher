# unreleased changelog


# Mod Store

- Redesigned, now with a new look and many features
- Added category filters:
  - Filter mods, resource packs and shaders by various search categories!
- Mod Descriptions: now with cleaner UI, links and gallery viewer
  - Mods menu: Right click -> Mod Details now takes you directly to description page

TODO: Add screenshots

# Import/export

- You can now import instances from:
  - MultiMC/PrismLauncher (experimental)
  - Modrinth/Curseforge modpacks
  - Official Minecraft Launcher, TL Legacy, TLauncher, GDLauncher, ATLauncher (TODO)
- Export instances to:
  - QuantumLauncher instance format (experimental)
  - MultiMC/PrismLauncher (TODO)
  - Modrinth/Curseforge modpacks (TODO)

# UX

- You can now automatically create changelogs after updating mods,
  showing which versions changed.
- Added success notification messages for common tasks like installing/uninstalling mod loaders,
  importing/exporting presets, etc.

# Fixes

- Fixed "system theme" error spam on Raspberry Pi OS, LXDE, Openbox, etc
- Fixed launcher auto-updater not supporting `.tar.gz` files (only `.zip`)
- Fixed Modrinth pages sometimes appearing after selecting Curseforge,
  and vice versa
- Fixed mods installed through Curseforge modpacks internally being
  stored as Modrinth mods
