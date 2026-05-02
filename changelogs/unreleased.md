# unreleased changelog

# Mod Store

- **Redesigned** with a new UI and expanded features
- Added **category filters** for mods, resource packs, and shaders
- Added **sorting options** for search results
- Improved mod descriptions with cleaner layout, links, and gallery viewer
  - Mods menu: **Right click → Mod Details** now opens the description page directly

> TODO: Add screenshots

# Mod Loaders

- You can now directly install mod loaders from the Create Instance screen

> TODO: More in future

# Discord Rich Presence

- Added support for displaying your status in Discord through Rich Presence
- Customizable, can display any text or info (game version/instance name) of your choice

# UX

- Automatically **generate changelogs** after mod updates, showing version changes
- Added **success notifications** for common actions
  (e.g., installing mod loaders, importing/exporting presets)
- New launcher behavior options when a game opens: **minimize, close, or do nothing**
  - Now configured globally in Launcher Settings (moved from per-instance settings)
- Improved Launcher Settings page design

# Server Manager

(still experimental, enable with `--enable-server-manager`)

- Now servers and instances are unified in one list
  - The list now reloads in real time if your instances change on disk

# Fixes

- Fixed "system theme" error spam on Raspberry Pi OS (Labwc)
- Fixed launcher auto-updater not supporting `.tar.gz` files
- Fixed Modrinth and CurseForge pages occasionally mixing after selection
- Fixed CurseForge modpack mods being incorrectly stored as Modrinth mods
- Fixed Java binary detection on Linux ARM
- Fixed switching to server manager messing up folder organization
- Fixed Logs tab not being updated when switching instances
- Fixed one instance's log output showing up in another
- Fixed up/down arrow key instance selection following creation date order
  rather than the order shown in sidebar
- Fixed Optifine for 1.2.5 not installing properly
- Reduced clashing between sidebar resizing and scrollbar in main menu
