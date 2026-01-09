# unreleased changelog

- Added quick uninstall button to Mod Store
- Usernames in paths are now censored in logs
  - eg: `C:\Users\YOUR_NAME` or `/home/YOUR_NAME` ->
    `C:\Users\[REDACTED]` or `/home/[REDACTED]`
  - Use `--no-redact-info` CLI flag to temporarily disable this

# Fixes

- Fixed concurrent downloading bugs with CurseForge
- Fixed context menus not closing after a click
