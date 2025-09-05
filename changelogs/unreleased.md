# unreleased

# Mods

## Loaders

Support for a few alternate implementations of Fabric were added:
- Legacy Fabric (1.3 - 1.13)
- OrnitheMC (b1.7 - 1.13)
- Babric (b1.7.3)
- Cursed Legacy (b1.7.3)

These are for versions where Fabric isn't officially supported.
If multiple are available you can choose between them.

## UX
- (TODO) Presets and text exporting (explained below) are now grouped together
  under a hamburger menu for easier access
- Disabled mods are now tinted darker
- (TODO) Mod selection no longer uses clunky checkboxes, but rather proper list items
- Ctrl-A can now select all mods and jarmods
## Text export
- You can now export your mods as a text list, making it easy to share with others.
- The list includes mods, and can optionally include links and instance details.
- Anyone can view the list and manually install mods, regardless of their launcher.

# Other

## File location (linux)
- Files moved from `~/.config` to `~/.local/share`
- There will be auto-migration, and symbolic links so older launcher versions still work seamlessly

# Fixes
- CurseForge mods that don't specify a loader
  can now be installed
- You can now open instances created in newer versions, in v0.4.1
- Fixed crash with "Better Discord Rich Presence" mod
