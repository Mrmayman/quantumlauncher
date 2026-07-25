# Using QuantumLauncher with Nix

QuantumLauncher ships a Nix flake providing packages, a dev shell, a NixOS
module, and a Home Manager module. Non-flake `nix-build` / `nix-shell` usage
is also supported via `default.nix` / `shell.nix`.

## Quick start (with flakes enabled)

```sh
# Run without installing
nix run github:Mrmayman/quantumlauncher

# Build locally
nix build github:Mrmayman/quantumlauncher
./result/bin/quantum_launcher

# Dev shell with the pinned Rust toolchain + all native deps
nix develop github:Mrmayman/quantumlauncher
```


## Without flakes enabled

Uses the `default.nix` / `shell.nix` compat shims (via `flake-compat`):

```sh
git clone https://github.com/Mrmayman/quantumlauncher.git
cd quantumlauncher
nix-build
./result/bin/quantum_launcher

# or, for a dev shell
nix-shell
```

## Package outputs

| Output | Cargo profile | Use case |
|---|---|---|
| `packages.default` | `release-ql` | The tuned release build (LTO, stripped, panic=abort)  |
| `packages.release` | `release-ql` | Alias of `default` |
| `packages.release-dbg` | `release-dbg` | Release build with debug symbols |

Build a specific output with:

```sh
nix build .#release-dbg
```

Supported systems: `x86_64-linux`, `aarch64-linux`, `x86_64-darwin`,
`aarch64-darwin`. Only `x86_64-linux` has been tested so far.

## NixOS module

```nix
{
  inputs.quantumlauncher.url = "github:Mrmayman/quantumlauncher";

  outputs = { self, nixpkgs, quantumlauncher, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      modules = [
        quantumlauncher.nixosModules.default
        {
          programs.quantum-launcher.enable = true;
          # Optional: override the package used
          # programs.quantum-launcher.package = quantumlauncher.packages.x86_64-linux.release-dbg;
        }
      ];
    };
  };
}
```

## Home Manager module

```nix
{
  inputs.quantumlauncher.url = "github:Mrmayman/quantumlauncher";

  outputs = { self, home-manager, quantumlauncher, ... }: {
    homeConfigurations.myuser = home-manager.lib.homeManagerConfiguration {
      modules = [
        quantumlauncher.homeManagerModules.default
        {
          programs.quantum-launcher.enable = true;
        }
      ];
    };
  };
}
```

## Non-NixOS Linux: graphics may need nixGL

If you're using Nix on a non-NixOS distro (Ubuntu, Fedora, Arch, etc. via
the standalone Nix installer), you are likely to hit a crash on wayland
which i faced when running QuantumLauncher. This happens
because the OpenGL/Vulkan libraries QuantumLauncher links against come
from nixpkgs, not your system's actual GPU driver. on NixOS these are
wired together automatically, but on other distros they aren't.

The fix is to wrap the binary with [nixGL](https://github.com/nix-community/nixGL),
which makes it use your host system's real driver instead. Example using
Home Manager (adjust `nixGLIntel` to `nixGLNvidia`/`nixGLMesa`/etc. to match
your GPU):
Note: the above paragraph may not be entirely accurate , this is workaround i found.
feel free to open a pr to correct it :).

```nix
{ pkgs, nixgl, quantumlauncher, ... }:
{
  home.packages = [
    (pkgs.writeShellScriptBin "quantum_launcher" ''
      exec ${nixgl.packages.${pkgs.stdenv.hostPlatform.system}.nixGLIntel}/bin/nixGLIntel \
        ${quantumlauncher.packages.${pkgs.stdenv.hostPlatform.system}.release}/bin/quantum_launcher "$@"
    '')
  ];
}
```



## Known limitations

- Tagged releases are published to the project's binary cache. Development
  builds or commits without cached artifacts will be built locally.
- macOS and `aarch64-linux` builds are untested.
