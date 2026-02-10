# unreleased changelog

- Added a Quick Uninstall button to the Mod Store

# Shortcuts

- Launch instances with a single click; no launcher required!
- Create one-click shortcuts for:
  - Desktop
  - Start Menu / Applications Menu (Windows/Linux)
  - Applications (macOS)
  - Other custom locations

# Technical

- Higher memory allocation values (upto 32 GB)
  are now supported in Edit Instance screen
  - Manual input now supported, alongside slider
- Usernames are now redacted in log paths
  - eg: `C:\Users\YOUR_NAME` -> `C:\Users\[REDACTED]`
  - Disable temporarily with `--no-redact-info` flag

## Java

- Select launcher-provided Java versions in addition to custom paths
- Improved Java installer with expanded platform support
  - Minecraft 1.20.5–1.21.11 now runs on many 32-bit systems
- Platforms without Mojang Java now use **Azul Zulu** instead of Amazon Corretto

# Fixes

- Fixed many CurseForge concurrent downloading issues
- Fixed account login persistence for new users
- Fixed post-1.21.11 versions (eg: snapshots) not launching on Linux ARM
- Fixed unnecessary Java redownloads on some ARM systems

## Logging

- Overhauled log viewer: text selection, better scrolling, fewer bugs
- Fixed missing crash reports in logs
- Added warning when running in macOS VM
