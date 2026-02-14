# unreleased changelog

- Added quick uninstall button to Mod Store

# Technical

- Higher memory allocation values (upto 32 GB)
  are now supported in Edit Instance screen
  - You can also manually enter a number instead of using the slider
- Usernames in paths are now censored in logs
  - eg: `C:\Users\YOUR_NAME` or `/home/YOUR_NAME` ->
    `C:\Users\[REDACTED]` or `/home/[REDACTED]`
  - Use `--no-redact-info` CLI flag to temporarily disable this

## Java

- In addition to custom Java paths, you can now choose
  different launcher-provided Java versions as well
- The java installer has been improved with better platform support
  - For example, you can now run Minecraft 1.20.5 to 1.21.11
    on many 32-bit systems
- For platforms without Mojang-provided Java,
  we now use Azul Zulu instead of Amazon Corretto

# Fixes

- Fixed many concurrent downloading bugs with CurseForge
- Fixed account login being broken for new users
- Fixed versions after 1.21.11 (eg: snapshots) not launching on Linux ARM
- Fixed Java being frequently redownloaded on some ARM systems

## Logging
- Overhauled log viewer code, now with text selection,
  better scrolling, and fewer bugs
- Fixed game crash reports not showing in logs
- Added warning when running in macOS VM
