# Aether VM target (development only)
#
# SEPARATION POLICY
# -----------------
# This file is for booting Aether in a local virtual machine. It is intentionally
# NOT imported by os/nixos/configuration.nix, and none of these settings belong
# in nixosConfigurations.kestrel:
#   * no i7-7700K kernel fragment
#   * no XFS/ext4/tmpfs reference disk layout
#   * no PipeWire / agent supervisord
#   * a development user, password login, passwordless sudo and no firewall
#
# Shared code belongs in packages/ or os/compositor/, never in this directory.
#
# Built as a qcow2 EFI image by nixos-generators (see flake.nix:
# packages.aarch64-linux.vm-image). The qcow-efi format supplies the root
# filesystem, ESP, GRUB and serial consoles; do not duplicate them here.
{
  pkgs,
  lib,
  ...
}: {
  imports = [../common/base.nix];

  system.stateVersion = "26.11";

  networking.hostName = "aether-vm";
  time.timeZone = "UTC";

  # Small, sparse disk. Grow with qemu-img if you need more.
  virtualisation.diskSize = 4096;

  # ttyAMA0 is added by the qcow-efi format; tty0 is for a graphical console.
  boot.kernelParams = ["console=tty0" "loglevel=4"];

  # --- development account -------------------------------------------------
  # VM only. The reference machine uses a locked root and a hashed operator
  # password; do not copy this block into os/nixos.
  users.mutableUsers = true;
  users.users.aether = {
    isNormalUser = true;
    description = "Aether VM";
    extraGroups = ["wheel"];
    initialPassword = "aether";
  };
  security.sudo.wheelNeedsPassword = false;
  services.getty.autologinUser = "aether";

  # --- tooling -------------------------------------------------------------
  environment.systemPackages = with pkgs; [
    helix
    tmux
    htop
    pciutils
    usbutils
    curl
    git
  ];

  documentation.enable = false;
  services.openssh = {
    enable = true;
    settings = {
      PasswordAuthentication = true;
      PermitRootLogin = "no";
    };
  };
  networking.firewall.enable = false; # user-mode NAT only; not the install target

  nix.settings.trusted-users = ["root" "aether"];

  # A deterministic marker for headless smoke tests. It only writes to the
  # serial console; it does not power off or modify the system.
  systemd.services.aether-vm-report = {
    wantedBy = ["multi-user.target"];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
    };
    script = ''
      {
        echo "AETHER_VM_READY arch=$(uname -m) kernel=$(uname -r)"
        echo "AETHER_VM_LSM=$(cat /sys/kernel/security/lsm 2>/dev/null || echo unknown)"
        echo "AETHER_VM_USER=$(id -un)"
      } > /dev/ttyAMA0 2>/dev/null || true
    '';
  };

  environment.etc."aether-vm/README".text = ''
    Aether OS development VM.

    This image is a boot harness, not the i7-7700K installation target.
    Build: scripts/vm-build.sh
    Boot:  scripts/vm-run.sh
    Smoke: scripts/vm-smoke.sh

    Login: aether / aether (VM only, passwordless sudo).

    The reference install lives in os/nixos and is installed with
    scripts/install.sh. Do not copy VM settings there.
  '';
}
