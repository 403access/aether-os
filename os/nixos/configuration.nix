# Aether OS — system-wide configuration.
#
# This file deliberately contains no desktop environment. The session is a
# single compositor started from tty1; everything else is a service it owns.
{
  config,
  lib,
  pkgs,
  ...
}: {
  imports = [
    ../common/base.nix
    ./hardware/kabylake-7700k.nix
    ./kernel.nix
    ./filesystem.nix
    ./services.nix
    ./users.nix
  ];

  system.stateVersion = "26.11";

  nix = {
    settings = {
      auto-optimise-store = true;
      trusted-users = ["root" "oliver"];
    };
    gc = {
      automatic = true;
      dates = "weekly";
      options = "--delete-older-than 30d";
    };
  };

  boot = {
    loader = {
      systemd-boot = {
        enable = true;
        configurationLimit = 5;
      };
      efi.canTouchEfiVariables = true;
      timeout = 1;
    };
    tmp.useTmpfs = true;
    tmp.cleanOnBoot = true;
    supportedFilesystems = ["xfs" "ext4" "vfat"];
  };

  # The operator is the only interactive user; no display manager.
  services.getty.autologinUser = "oliver";

  networking = {
    hostName = "kestrel";
    networkmanager.enable = true;
    firewall.enable = true;
    # No inbound services by default.
    firewall.allowedTCPPorts = [];
    firewall.allowedUDPPorts = [];
  };

  time.timeZone = "Europe/Berlin";

  i18n.inputMethod = {
    enable = true;
    type = "fcitx5";
  };
  console.font = "Lat2-Terminus16";

  environment = {
    systemPackages = with pkgs; [
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
      aether-shell-smithay
      intel-gpu-tools
      vulkan-tools
      libva-utils
      aether-scene
    ];
    variables = {
      EDITOR = "hx";
    };
    etc."aether/README".text = ''
      Aether OS configuration lives in /nix/var/nix/profiles/system.
      Rebuild with: nixos-rebuild switch --flake /etc/aether#kestrel
    '';
  };

  # Locally built packages are already tuned; keep the store from rebuilding
  # everything with generic flags.
  nixpkgs.config.allowUnfree = true;

  security = {
    sudo.enable = true;
    protectKernelImage = true;
    lockKernelModules = false; # needed for agent Landlock/seccomp development
  };

  zramSwap.enable = false;
  swapDevices = [];
}
