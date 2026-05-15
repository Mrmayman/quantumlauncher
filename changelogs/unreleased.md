# unreleased changelog

# Mod Store

- Redesigned, now with a new look and many features
- Added category filters:
  - Filter mods, resource packs and shaders by various search categories!
- Mod Descriptions: now with cleaner UI, links and gallery viewer
  - Mods menu: Right click -> Mod Details now takes you directly to description page

TODO: Add screenshots

# Discord Rich Presence

- Added support for displaying your status in Discord through Rich Presence
- Customizable, can display any text or info (game version/instance name) of your choice

# UX

- You can now automatically create changelogs after updating mods,
  showing which versions changed.
- Added success notification messages for common tasks like installing/uninstalling mod loaders,
  importing/exporting presets, etc.
- You can now choose to minimize the launcher after a game opens (new), or close it, or do nothing.
  - Configured in launcher settings (instead of per-instance settings).
- Added option to minimize launcher on game open
- Improved design of Launcher Settings page

# Server Manager

(still experimental, enable with `--enable-server-manager`)

- Now servers and instances are unified in one list
  - The list now reloads in real time if your instances change on disk

# Fixes

- Fixed "system theme" error spam on Raspberry Pi OS, LXDE, Openbox, etc
- Fixed launcher auto-updater not supporting `.tar.gz` files (only `.zip`)
- Fixed Modrinth pages sometimes appearing after selecting Curseforge,
  and vice versa
- Fixed mods installed through Curseforge modpacks internally being
  stored as Modrinth mods
- Fixed Java binary not being found on Linux ARM
- Fixed switching to server manager messing up folder organization
- Fixed Logs tab not being updated when switching instances
- Fixed one instance's log output showing up in another
- Fixed up/down arrow key instance selection following creation date order
  rather than the order shown in sidebar
- Reduced clashing between sidebar resizing and scrollbar in main menu
