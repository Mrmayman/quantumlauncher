# unreleased changelog

# Mod Store

- **Redesigned** with a new UI and expanded features
- Added **category filters** for mods, resource packs, and shaders
- Added **sorting options** for search results
- Improved mod descriptions with cleaner layout, links, and gallery viewer
  - Mods menu: **Right click → Mod Details** now opens the description page directly

> TODO: Add screenshots

# UX

- Automatically **generate changelogs** after mod updates, showing version changes
- Added **success notifications** for common actions
  (e.g., installing mod loaders, importing/exporting presets)
- New launcher behavior options when a game opens: **minimize, close, or do nothing**
  - Now configured globally in Launcher Settings (moved from per-instance settings)
- Improved Launcher Settings page design

# Fixes

- Fixed "system theme" error spam on Raspberry Pi OS (Labwc)
- Fixed launcher auto-updater not supporting `.tar.gz` files
- Fixed Modrinth and CurseForge pages occasionally mixing after selection
- Fixed CurseForge modpack mods being incorrectly stored as Modrinth mods
- Fixed Java binary detection on Linux ARM
