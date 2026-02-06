Command: `cargo run -p tests`

This launches an instance, and closes the window if successful.
It scans for a window for `timeout` seconds.

Cli Flags (optional):

- `--help`: See help message
- `--existing`: Whether to reuse existing Minecraft files instead of redownloading them
- `--timeout <SECONDS>`: How long to wait for a window before giving up (default: 60).
  - Increase this for slower systems
- `--verbose`: See all the logs to diagnose issues
- **Selection**:
  - `--specific <VERSION>`: Only test one version
  - `--skip-lwjgl3`: Only tests legacy LWJGL2-based versions (1.12.2 and below).
    Useful for less supported platforms like FreeBSD
  - `--skip-loaders` (TODO): Only test vanilla Minecraft, skipping mod loaders

# Supports

- Windows
- macOS
- Linux/FreeBSD (X11/XWayland)

# TODO

- Test different fabric backends
- Add `--include`/`--exclude` flags to select specific versions to test
