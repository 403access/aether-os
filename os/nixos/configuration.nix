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
    ./hardware/kabylake-7700k.nix
    ./kernel.nix
    ./filesystem.nix
    ./services.nix
    ./users.nix
  ];

  system.stateVersion = "25.11";

  nix = {
    settings = {
      experimental-features = ["nix-command" "flakes"];
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
  i18n.defaultLocale = "en_US.UTF-8";
  console = {
    font = "Lat2-Terminus16";
    keyMap = "us";
  };

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
      intel-gpu-tools
      vulkan-tools
      libva-utils
      aether-scene
    ];
    variables = {
      EDITOR = "hx";
      XDG_CACHE_HOME = "/home/oliver/.cache";
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
