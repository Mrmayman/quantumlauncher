# unreleased changelog

# TLDR;
- Added instance folders!
- Added shortcuts; launch the game from your taskbar in one click!
- Many UX improvements and fixes

---

- Added instance folders, with drag & drop and renaming

# Shortcuts

- Launch instances with a single click; no launcher required!
- Create one-click shortcuts for:
  - Desktop
  - Start Menu / Applications Menu (Windows/Linux)
  - Applications (macOS)
  - Other custom locations

# UX

- Improved new-user Welcome screen with keyboard navigation,
  better layout and more guidance
- Higher memory allocation values (upto 32 GB) now supported in Edit tab
  - Also added manual input, alongside slider

## Mods

- Redesigned Mod Store, with category filters and quick uninstall!
- Added a more visible toggler to mods list for enabling/disabling mods
- Disabled mods now stay disabled when updating

# Technical

- Mod update checking is no longer automatic
  - Now trigger it manually through "... -> Check for Updates"
  - We understand this is a UX regression, but
    this significantly reduces network usage and "error code 504" issues
- Usernames are now redacted in log paths
  - eg: `C:\Users\YOUR_NAME` -> `C:\Users\[REDACTED]`
  - Disable temporarily with `--no-redact-info` flag
- When launching headlessly (`quantum_launcher launch <INSTANCE> <USERNAME>`),
  you can now use `--show-progress` to get desktop notifications on account login progress or errors
  - Especially useful for shortcuts/scripts

## Java

- Select launcher-provided Java versions in addition to custom paths
- Improved Java installer with expanded platform support
  - Minecraft 1.20.5–1.21.11 now runs on many 32-bit systems
- Platforms without Mojang Java now use **Azul Zulu** instead of Amazon Corretto

# Fixes

- Fixed modrinth "error code 504" (caused by automatic update checks)
- Fixed context menus not closing after a click
- Fixed many CurseForge concurrent downloading issues
- Fixed QMP presets added via "Add File" in Mods menu, to not install all mods
- Fixed account login persistence for new users
- Fixed post-1.21.11 versions (eg: snapshots) not launching on Linux ARM
- Fixed unnecessary Java redownloads on some ARM systems

## Logging

- Overhauled log viewer: text selection, better scrolling, fewer bugs
- Fixed missing crash reports in logs
- Added warning when running in macOS VM
