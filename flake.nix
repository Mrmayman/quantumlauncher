{
  description = "QuantumLauncher";

  nixConfig = {
    extra-substituters = [
      "https://quantumlauncher.cachix.org"
    ];

    extra-trusted-public-keys = [
      "quantumlauncher.cachix.org-1:8y+ba6VjsH9kr988wfhPEYsUt0rAxat0V6CeXLzdWCg="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-compat = {
      url = "github:NixOS/flake-compat";
      flake = false;
    };

  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              config.allowDeprecatedx86_64Darwin = true;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );

      cargoToml = builtins.fromTOML (builtins.readFile ./quantum_launcher/Cargo.toml);

      commonAttrs = pkgs: {
        pname = cargoToml.package.name;
        version = cargoToml.package.version;
        cargoLock = {
          lockFile = ./Cargo.lock;
          outputHashes = {
            "dark-light-2.0.0" = "sha256-4JPfNSk8MsMMxJPz6M8ju+sMVXlAerct16xwbkEWpYw=";
            "http-cache-1.0.0-alpha.6" = "sha256-ZnrX0bnLARjwYeH4YnhwidAk2st124rGCR40oyFun4U=";
            "http-cache-semantics-3.0.0" = "sha256-D2n73HWEnhaBEZOwTzAOQ6kd2bO9oin5vEZ+zangJ9A=";
          };
        };

        buildAndTestSubdir = "quantum_launcher";

        nativeBuildInputs = with pkgs; [
          pkg-config
          makeWrapper
        ];

        buildInputs =
          with pkgs;
          [ libxkbcommon ]

          ++ lib.optionals stdenv.isLinux [
            libGL
            vulkan-loader
            wayland
            wayland-protocols
            libx11
            libxcursor
            libxi
            libxrandr
          ];

        postInstall = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
          wrapProgram $out/bin/${cargoToml.package.name} \
            --prefix LD_LIBRARY_PATH : "${
              pkgs.lib.makeLibraryPath [
                pkgs.wayland
                pkgs.libxkbcommon
                pkgs.libGL
                pkgs.vulkan-loader
              ]
            }"
        '';

        meta = with pkgs.lib; {
          description = cargoToml.package.description;
          homepage = cargoToml.package.homepage;
          mainProgram = cargoToml.package.name;
          license = licenses.gpl3Only;
        };

      };
    in
    {
      # ==========================================
      # 1. PACKAGES (For nix run, build, and tags)
      # ==========================================
      packages = forAllSystems (
        pkgs:
        let
          pinnedRustPlatform = pkgs.makeRustPlatform {
            cargo = pkgs.rust-bin.stable."1.85.0".default;
            rustc = pkgs.rust-bin.stable."1.85.0".default;
          };
        in
        {
          default = pinnedRustPlatform.buildRustPackage (
            (commonAttrs pkgs)
            // {
              src = pkgs.lib.cleanSource ./.;
              buildPhase = ''
                runHook preBuild

                export CARGO_TARGET_DIR=target

                cargo build \
                  --offline \
                  --frozen \
                  --profile release-ql

                runHook postBuild
              '';

              installPhase = ''
                runHook preInstall
                mkdir -p $out/bin
                cp target/release-ql/${cargoToml.package.name} $out/bin/
                runHook postInstall
              '';
            }
          );

          release = self.packages.${pkgs.stdenv.hostPlatform.system}.default;

          release-dbg = pinnedRustPlatform.buildRustPackage (
            (commonAttrs pkgs)
            // {
              src = pkgs.lib.cleanSource ./.;
              buildPhase = ''
                runHook preBuild

                export CARGO_TARGET_DIR=target

                cargo build \
                  --offline \
                  --frozen \
                  --profile release-dbg

                runHook postBuild
              '';

              installPhase = ''
                runHook preInstall
                mkdir -p $out/bin
                cp target/release-dbg/${cargoToml.package.name} $out/bin/
                runHook postInstall
              '';
            }
          );
        }
      );
      # ==========================================
      # 2. DEVELOPMENT ENVIRONMENT (nix develop)
      # ==========================================
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          nativeBuildInputs = [
            (pkgs.rust-bin.stable."1.85.0".default.override {
              extensions = [
                "rust-src"
                "rust-analyzer"
              ];
            })
            pkgs.pkg-config
          ];

          buildInputs = (commonAttrs pkgs).buildInputs;

          LD_LIBRARY_PATH = pkgs.lib.optionalString pkgs.stdenv.isLinux (
            pkgs.lib.makeLibraryPath [
              pkgs.wayland
              pkgs.libxkbcommon
              pkgs.libGL
              pkgs.vulkan-loader
            ]
          );

        };
      });

      # ==========================================
      # 3. NIXOS MODULE (System-wide configuration)
      # ==========================================
      nixosModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.programs.quantum-launcher;
        in
        {
          options.programs.quantum-launcher = {
            enable = lib.mkEnableOption "the QuantumLauncher Minecraft launcher.";
            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
              description = "The QuantumLauncher package to install.";
            };
          };

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ cfg.package ];
          };
        };

      # ==========================================
      # 4. HOME MANAGER MODULE (User-level configs)
      # ==========================================
      homeManagerModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.programs.quantum-launcher;
        in
        {
          options.programs.quantum-launcher = {
            enable = lib.mkEnableOption "the QuantumLauncher Minecraft launcher.";
            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
              description = "The QuantumLauncher package to install.";
            };
          };

          config = lib.mkIf cfg.enable {
            home.packages = [ cfg.package ];
          };
        };
    };
}
