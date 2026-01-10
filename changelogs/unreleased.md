# unreleased changelog

# Sidebar

- Added instance folders
- You can now drag and reorder instances/folders around

# UX

- Added quick uninstall button to Mod Store
- Improved new-user Welcome screen with keyboard navigation,
  better layout and more guidance

# Technical

- Usernames in paths are now censored in logs
  - eg: `C:\Users\YOUR_NAME` or `/home/YOUR_NAME` ->
    `C:\Users\[REDACTED]` or `/home/[REDACTED]`
  - Use `--no-redact-info` CLI flag to temporarily disable this

# Fixes

- Fixed context menus not closing after a click
- Fixed many concurrent downloading bugs with CurseForge
- Fixed littleskin OTP login being broken for new users
