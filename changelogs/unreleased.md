# unreleased changelog

# Mod Store

- Cleaned up layout and look of the store
- Added quick uninstall button to Mod Store

# Technical

- Usernames in paths are now censored in logs
  - eg: `C:\Users\YOUR_NAME` or `/home/YOUR_NAME` ->
    `C:\Users\[REDACTED]` or `/home/[REDACTED]`
  - Use `--no-redact-info` CLI flag to temporarily disable this

# Fixes

- Fixed many concurrent downloading bugs with CurseForge
- Fixed littleskin OTP login being broken for new users
- Fixed descriptions of the wrong mod appearing in the store
