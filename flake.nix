{
  description = "Aether OS — a single-target Linux workstation for the i7-7700K";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
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
            platforms = ["x86_64-linux"];
          };
        };
      };

      # The installed system: boots from disk after the install guide.
      nixosConfigurations.kestrel = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
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
