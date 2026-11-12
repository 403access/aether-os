{
  description = "Aether OS — a single-target Linux workstation for the i7-7700K";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixos-generators = {
      url = "github:nix-community/nixos-generators";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, nixos-generators}:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default self.overlays.default ];
        config.allowUnfree = true;
      };

      # Keep local build artefacts out of the package source.
      sceneSource = nixpkgs.lib.cleanSourceWith {
        src = ./os/compositor;
        filter = path: _type: builtins.baseNameOf path != "target";
      };
    in {
      overlays.default = final: prev: {
        aether-scene = final.rustPlatform.buildRustPackage {
          pname = "aether-scene";
          version = "0.1.0";
          src = sceneSource;
          cargoLock.lockFile = ./os/compositor/Cargo.lock;
          cargoBuildFlags = ["--package" "aether-scene" "--bin" "aether-scene-demo"];
          installPhase = ''
            mkdir -p $out/bin
            cp target/release/aether-scene-demo $out/bin/aether-scene
          '';
          meta = with final.lib; {
            description = "Aether compositor scene graph, damage tracking and frame pacing";
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
      };

      # ---------------------------------------------------------------------
      # Virtual-machine target. This is a DEVELOPMENT HARNESS, deliberately
      # separate from os/nixos (the i7-7700K install target). It does not
      # import the reference hardware, kernel fragment, storage layout or
      # users; VM-only settings must never leak into kestrel.
      # ---------------------------------------------------------------------
      packages.aarch64-linux = {
        vm-image = nixos-generators.nixosGenerate {
          system = "aarch64-linux";
          format = "qcow-efi";
          modules = [./os/vm/configuration.nix];
        };
        default = self.packages.aarch64-linux.vm-image;
      };

      devShells.${system}.default = pkgs.mkShell {
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

      formatter.${system} = pkgs.nixpkgs-fmt;
    };
}
