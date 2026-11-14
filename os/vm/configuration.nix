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
# Built as a qcow2 EFI image by NixOS's built-in image support (see flake.nix:
# packages.aarch64-linux.vm-image, image target `qemu-efi`). The image module
# supplies the root filesystem, ESP and systemd-boot; do not duplicate them
# here.
{
  pkgs,
  lib,
  modulesPath,
  ...
}: {
  imports = [
    ../common/base.nix
    # virtio storage/net/console drivers. nixos-generators' qcow-efi format
    # imported this; NixOS's built-in image modules do not, so the image would
    # otherwise not find its root disk under QEMU.
    "${modulesPath}/profiles/qemu-guest.nix"
  ];

  system.stateVersion = "26.11";

  networking.hostName = "aether-vm";
  time.timeZone = "UTC";

  i18n.inputMethod = {
    enable = true;
    type = "fcitx5";
  };

  # Small, sparse disk. Grow with qemu-img if you need more.
  virtualisation.diskSize = 4096;
  # The VM may run under TCG when nested KVM is unavailable. Override QEMU
  # memory/cores at launch (`QEMU_OPTS="-m 2048 -smp 2"`) if boot is slow.

  # The upstream image modules do not add a serial console; keep the behaviour
  # the old nixos-generators qcow-efi format provided. The console= parameters
  # also make systemd spawn a getty on the serial line.
  boot.kernelParams = ["console=tty0" "console=ttyAMA0,115200" "loglevel=4"];
  boot.loader.timeout = 0;

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
    aether-shell-drm
    cage
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

  # System index service for the VM user. The compositor target would point
  # AETHER_INDEX_SOCKET at the same path.
  systemd.user.services.aether-index = {
    description = "Aether system index service";
    wantedBy = ["default.target"];
    serviceConfig = {
      ExecStart = "${pkgs.aether-index-service}/bin/aether-index-service";
      Environment = [
        "AETHER_INDEX_SOCKET=%t/aether-index.sock"
        "AETHER_INDEX_ROOTS=%h"
        "AETHER_INDEX_LIMIT=5000"
      ];
      Restart = "on-failure";
    };
  };

  # Native DRM bring-up on the VM's virtio-gpu. Runs on tty7 with a logind
  # session so libseat can open /dev/dri/card0, then reports the result on the
  # serial console. This is VM-only validation for the same backend the
  # reference install runs.
  systemd.services.aether-vm-drm = {
    description = "Aether native DRM boot check";
    wantedBy = ["multi-user.target"];
    after = ["systemd-user-sessions.service" "systemd-logind.service"];
    serviceConfig = {
      PAMName = "login";
      TTYPath = "/dev/tty7";
      StandardInput = "tty";
      StandardOutput = "journal";
      TimeoutStartSec = 300;
    };
    script = ''
      {
        echo "AETHER_VM_DRM_BEGIN"
        echo "AETHER_VM_DRM_UID=$(id -u)"
        ls -l /dev/dri || true
        ${pkgs.systemd}/bin/loginctl session-status 2>&1 | head -20 || true
      } >>/dev/ttyAMA0
      ${pkgs.kbd}/bin/chvt 7 || true
      sleep 3
      export XDG_RUNTIME_DIR=/run/user/$(id -u)
      export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ pkgs.libGL pkgs.mesa pkgs.libgbm ]}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
      export LIBGL_DRIVERS_PATH="${pkgs.mesa}/lib/dri"
      export GBM_BACKENDS_PATH="${pkgs.mesa}/lib/gbm"
      # virtio-gpu under TCG; use the software rasterizer for composition.
      export MESA_LOADER_DRIVER_OVERRIDE=llvmpipe
      export LIBGL_ALWAYS_SOFTWARE=1
      mkdir -p "$XDG_RUNTIME_DIR" 2>/dev/null || true
      timeout -k 10 480 ${pkgs.aether-shell-drm}/bin/aether-shell-drm --frames 1 >>/dev/ttyAMA0 2>&1
      echo "AETHER_VM_DRM_STATUS=$?" >>/dev/ttyAMA0
    '';
  };

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
