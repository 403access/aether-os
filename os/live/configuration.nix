# Aether live ISO.
#
# This is the full Aether closure on a bootable image: the native DRM session,
# shell surfaces, index, audio and terminal start from the ISO. It is not an
# installer and it has no persistence unless you add a data partition.
#
# The live image is built for x86_64-linux and requires an x86_64 Linux builder:
#   nix build .#live-iso
#
# It reuses the installed system's services/users/hardware modules but overrides
# the disk-backed filesystems and bootloader. Keep target-only policy out of the
# installed configuration; this module is the live and disposable counterpart.
{
  config,
  lib,
  pkgs,
  modulesPath,
  ...
}: {
  imports = [
    ../common/base.nix
    ../nixos/hardware/kabylake-7700k.nix
    ../nixos/services.nix
    ../nixos/users.nix
    (modulesPath + "/installer/cd-dvd/installation-cd-minimal.nix")
  ];

  system.stateVersion = "26.11";

  # The ISO module owns `/` and `/nix`; only override the disk-backed paths that
  # came from the installed-system modules.
  fileSystems."/boot" = lib.mkForce {
    device = "tmpfs";
    fsType = "tmpfs";
    options = ["size=64M" "mode=0755"];
  };

  networking = {
    hostName = "aether-live";
    networkmanager.enable = true;
    firewall.enable = false;
  };

  services.getty.autologinUser = lib.mkForce "oliver";

  # A live system is an explicit, disposable environment. Keep it writable but
  # do not pretend a reboot preserves state.
  environment.etc."aether/README".text = ''
    Aether live session. Changes under / are lost on reboot.
    The compositor runs on tty7; the console is tty1.
    Install for persistence: see /docs/getting-started/install.
  '';

  environment.systemPackages = with pkgs; [
    bun
    git
    ripgrep
    fd
    jq
    curl
    htop
    helix
    tmux
    pciutils
    usbutils
    alsa-utils
    pavucontrol
    wireplumber
    wl-clip-persist
    cage
    aether-index
    aether-audio
    aether-shell-drm
    aether-shell-smithay
    aether-session
    aether-terminal
    intel-gpu-tools
    vulkan-tools
    libva-utils
    aether-scene
  ];

  fonts.packages = with pkgs; [inter dejavu_fonts];

  nixpkgs.config.allowUnfree = true;

  security = {
    sudo.enable = true;
    protectKernelImage = true;
    lockKernelModules = false;
  };

  zramSwap.enable = false;
  swapDevices = [];

  # USB/ISO boot knobs. The minimal CD supplies the initrd and squashfs root.
  isoImage = {
    makeEfiBootable = true;
    makeUsbBootable = true;
    squashfsCompression = "zstd -Xcompression-level 6";
  };
}
