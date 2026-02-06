# unreleased changelog

- Added quick uninstall button to Mod Store

# Technical

- Usernames in paths are now censored in logs
  - eg: `C:\Users\YOUR_NAME` or `/home/YOUR_NAME` ->
    `C:\Users\[REDACTED]` or `/home/[REDACTED]`
  - Use `--no-redact-info` CLI flag to temporarily disable this

# Fixes

- Fixed many concurrent downloading bugs with CurseForge
- Fixed account login being broken for new users

## Logging
- Overhauled log viewer code, now with text selection and better scrolling
- Fixed game crash reports not showing in logs
- Added warning when running in macOS VM
