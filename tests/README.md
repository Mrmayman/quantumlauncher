Command: `cargo run -p tests`

This launches an instance, and closes the window if successful.
It scans for a window for `timeout` seconds.

Warning: **highly experimental**

Cli Flags (optional):

- `--help`: See help message
- `--existing`: Whether to reuse existing test files instead of redownloading them
- `--timeout`: How long to wait for a window before giving up (default: 60).
- `--verbose`: See all the logs to diagnose issues
- `--skip-lwjgl3`: Only tests legacy LWJGL2-based versions (1.12.2 and below).
  Useful for less supported platforms like FreeBSD.
- `--skip-loaders` (TODO): Only test vanilla Minecraft, skipping mod loaders

# Supports

- Windows
- X11 (or XWayland under Wayland) environments like **Linux, FreeBSD**, etc.

If you're interested in helping me out,
consider porting the `src/search` module to macOS (osascript?).

# TODO

- Add macOS support
- Test different fabric backends
- Also have a basic testing system for other platforms using
  crate features `simulate_*_*`, by creating instance and using
  `ls` and `file` to verify correct natives in
  `INSTANCE/libraries/natives/`
