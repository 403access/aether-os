# Live installer ISO.
#
# This is a normal NixOS minimal live environment with the tools the Aether
# install guide needs: git, disko, helix, rsync and the hardware probes. It does
# NOT contain the Aether system closure — the installer fetches the flake (or
# reads a copy from USB) and installs `nixosConfigurations.kestrel`.
{ modulesPath, pkgs, lib, ... }: {
  imports = [
    (modulesPath + "/installer/cd-dvd/installation-cd-minimal.nix")
  ];

  nix.settings = {
    experimental-features = ["nix-command" "flakes"];
    trusted-users = ["root" "nixos"];
  };

  environment.systemPackages = with pkgs; [
    git
    curl
    rsync
    disko
    helix
    tmux
    pciutils
    usbutils
    hwinfo
    smartmontools
    nvme-cli
  ];

  # The minimal CD already enables NetworkManager; keep it explicit so the
  # install guide can rely on `nmtui`/`nmcli`.
  networking.networkmanager.enable = lib.mkDefault true;

  # Faster live boot and a smaller image.
  isoImage.squashfsCompression = "zstd -Xcompression-level 6";

  # Make it obvious that this is the Aether installer, not a generic ISO.
  boot.kernelParams = ["console=tty0" "quiet" "loglevel=4"];
}
