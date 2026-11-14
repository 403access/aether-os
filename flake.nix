{
  description = "Aether OS — a single-target Linux workstation for the i7-7700K";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {self, nixpkgs, rust-overlay}:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default self.overlays.default ];
        config.allowUnfree = true;
      };

      # Linux-only Smithay backend dependencies. Kept out of the default shell
      # and workspace so macOS/CI never try to build a Wayland compositor.
      mkSmithayShell = shellSystem: let
        shellPkgs = import nixpkgs {
          system = shellSystem;
          config.allowUnfree = true;
        };
      in
        shellPkgs.mkShell {
          packages = with shellPkgs; [
            bun
            cargo
            rustc
            clippy
            rustfmt
            pkg-config
            wayland
            libxkbcommon
            libglvnd
            libGL
            mesa
            libx11
            libxcursor
            libxi
            libxrandr
            libxinerama
            libxext
            libxcb
            xvfb
            weston
            wayland-utils
            wl-clipboard
            wl-clip-persist
            xdotool
            imagemagick
          ];
          shellHook = ''
            export LD_LIBRARY_PATH="${shellPkgs.lib.makeLibraryPath [
              shellPkgs.wayland
              shellPkgs.libxkbcommon
              shellPkgs.libglvnd
              shellPkgs.libGL
              shellPkgs.mesa
              shellPkgs.libx11
              shellPkgs.libxcursor
              shellPkgs.libxi
              shellPkgs.libxrandr
              shellPkgs.libxinerama
              shellPkgs.libxext
              shellPkgs.libxcb
            ]}:$LD_LIBRARY_PATH"
            export LIBGL_DRIVERS_PATH="${shellPkgs.mesa}/lib/dri"
            export __EGL_VENDOR_LIBRARY_FILENAMES="${shellPkgs.mesa}/share/glvnd/egl_vendor.d/50_mesa.json"
            export MESA_LOADER_DRIVER_OVERRIDE=llvmpipe
            export LIBGL_ALWAYS_SOFTWARE=1
          '';
        };

      # Keep local build artefacts out of the package source.
      sceneSource = nixpkgs.lib.cleanSourceWith {
        src = ./os/compositor;
        filter = path: _type: builtins.baseNameOf path != "target";
      };

      indexSource = nixpkgs.lib.fileset.toSource {
        root = ./.;
        fileset = nixpkgs.lib.fileset.unions [
          ./apps/indexer-service
          ./packages/indexer
          ./packages/protocol
          ./tsconfig.json
          ./package.json
        ];
      };
    in {
      overlays.default = final: prev: {
        aether-scene = final.rustPlatform.buildRustPackage {
          pname = "aether-scene";
          version = "0.1.0";
          src = sceneSource;
          cargoLock.lockFile = ./os/compositor/Cargo.lock;
          cargoBuildFlags = ["--workspace"];
          installPhase = ''
            mkdir -p $out/bin
            cp target/release/aether-scene-demo $out/bin/aether-scene
            cp target/release/aether-shell-preview $out/bin/aether-shell-preview
          '';
          meta = with final.lib; {
            description = "Aether compositor scene graph, damage tracking and frame pacing";
            license = licenses.mit;
            platforms = ["x86_64-linux"];
          };
        };

        aether-index-service = final.stdenvNoCC.mkDerivation {
          pname = "aether-index-service";
          version = "0.1.0";
          src = indexSource;
          nativeBuildInputs = [final.bun];
          dontConfigure = true;
          buildPhase = ''
            runHook preBuild
            export HOME=$TMPDIR
            bun build apps/indexer-service/src/main.ts --compile --outfile aether-index-service
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            install -Dm755 aether-index-service $out/bin/aether-index-service
            runHook postInstall
          '';
          meta = with final.lib; {
            description = "Aether system index service (Unix socket)";
            license = licenses.mit;
            platforms = ["x86_64-linux" "aarch64-linux"];
          };
        };
      };

      # The installed system: boots from disk after the install guide.
      nixosConfigurations.kestrel = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          # Make the Aether overlay (aether-scene, future daemons) visible to
          # the NixOS pkgs instance, not just to flake packages.
          { nixpkgs.overlays = [ self.overlays.default ]; }
          ./os/nixos/configuration.nix
          ./os/nixos/hardware/kabylake-7700k.nix
        ];
      };

      # The live installer: a minimal NixOS environment with git + disko.
      nixosConfigurations.installer = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [./os/nixos/installer.nix];
      };

      # Disk layout consumed by: disko --flake .#kestrel
      diskoConfigurations.kestrel = import ./os/nixos/disko.nix (import ./os/nixos/disk.nix);

      packages.${system} = {
        default = self.packages.${system}.iso;
        iso = self.nixosConfigurations.installer.config.system.build.isoImage;
        installer-iso = self.packages.${system}.iso;
        aether-scene = pkgs.aether-scene;
        index-service = pkgs.aether-index-service;
      };

      # ---------------------------------------------------------------------
      # Virtual-machine target. This is a DEVELOPMENT HARNESS, deliberately
      # separate from os/nixos (the i7-7700K install target). It does not
      # import the reference hardware, kernel fragment, storage layout or
      # users; VM-only settings must never leak into kestrel.
      # ---------------------------------------------------------------------
      packages.aarch64-linux = let
        vmPkgs = import nixpkgs {
          system = "aarch64-linux";
          overlays = [self.overlays.default];
          config.allowUnfree = true;
        };
        vm = nixpkgs.lib.nixosSystem {
          system = "aarch64-linux";
          modules = [
            { nixpkgs.overlays = [ self.overlays.default ]; }
            ./os/vm/configuration.nix
          ];
        };
      in {
        # NixOS's built-in image support: `qemu-efi` is the upstream
        # replacement for nixos-generators' `qcow-efi` (qcow2 + ESP +
        # systemd-boot).
        vm-image = vm.config.system.build.images.qemu-efi;
        index-service = vmPkgs.aether-index-service;
        default = vm.config.system.build.images.qemu-efi;
      };

      devShells.aarch64-linux = {
        smithay = mkSmithayShell "aarch64-linux";
      };

      devShells.${system} = {
        smithay = mkSmithayShell system;
        default = pkgs.mkShell {
        packages = with pkgs; [
          bun
          nodejs_22
          rust-bin.stable.latest.default
          cargo
          clippy
          rustfmt
          nixpkgs-fmt
          disko
        ];
          shellHook = ''
            echo "Aether dev shell"
            echo "  bun install && bun run verify"
            echo "  nix build .#iso"
            echo "  cargo test --manifest-path os/compositor/Cargo.toml"
          '';
        };
      };

      formatter.${system} = pkgs.nixpkgs-fmt;
    };
}
