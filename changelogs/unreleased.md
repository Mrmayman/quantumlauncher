# unreleased

# Mods

## Loaders

- You can now install OptiFine and Forge together!

### Fabric

Support for a few alternate implementations of Fabric were added:
- Legacy Fabric (1.3 - 1.13)
- OrnitheMC (b1.7 - 1.13)
- Babric (b1.7.3)
- Cursed Legacy (b1.7.3)

These are for versions where Fabric isn't officially supported.
If multiple are available you can choose between them.

- Added extra Quilt support via OrnitheMC for older versions
- Made fabric installer faster

---

## UX

- Overhauled the mod list, now with **icons**, bulk-selection
  (shift/ctrl-click and ctrl-a), and better aesthetics.
- Presets, text exporting (explained below), and recommended mods
  are now under a hamburger menu.

## Text export

- Export mods as a text list for easy sharing.
- List includes mods, optional links, and instance details.
- Anyone can view and manually install mods, regardless of their launcher.

---

## Keyboard Navigation

- `Ctrl/Cmd/Alt 1/2/3` to switch tabs in main screen
- `Ctrl N` to create new instance
- `Ctrl ,` to open settings

## File location (linux)

- Files moved from `~/.config` to `~/.local/share`.
- Auto-migration and symbolic links ensure compatibility with older launcher versions.

# Fixes

- Colored terminal output on Windows.
- CurseForge mods without a loader can now be installed.
- Instances from newer launcher versions can be opened in v0.4.1.
- Backspace no longer kills running instances without Ctrl.

- Fixed crash with "Better Discord Rich Presence" mod.
- Fixed launcher panics when launching the game.
- Fixed "java binary not found" macOS error.
- Fixed NeoForge 1.21.1 crash (reinstall NeoForge to apply)
