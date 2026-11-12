# Storage layout for the reference machine.
#
# XFS on the hot paths, ext4 for disposable scratch, tmpfs for the index.
# Labels are stable across device reordering; adjust them to your install.
{
  config,
  lib,
  pkgs,
  ...
}: {
  fileSystems."/" = {
    device = "/dev/disk/by-label/aether-root";
    fsType = "xfs";
    options = ["noatime" "largeio" "inode64"];
  };

  fileSystems."/nix" = {
    device = "/dev/disk/by-label/aether-nix";
    fsType = "xfs";
    options = ["noatime" "largeio" "inode64"];
  };

  fileSystems."/home" = {
    device = "/dev/disk/by-label/aether-home";
    fsType = "xfs";
    options = ["noatime" "largeio" "inode64"];
  };

  fileSystems."/scratch" = {
    device = "/dev/disk/by-label/aether-scratch";
    fsType = "ext4";
    # commit=60 + writeback are acceptable only because this volume is
    # disposable. Never use these options for / or /home.
    options = ["noatime" "nodiratime" "commit=60" "data=writeback"];
  };

  fileSystems."/var/index" = {
    device = "tmpfs";
    fsType = "tmpfs";
    options = ["size=2G" "mode=0700" "nosuid" "nodev"];
  };

  # The archive is mounted manually or via a systemd mount unit; it is not
  # required for boot.
  fileSystems."/mnt/archive" = lib.mkIf false {
    device = "/dev/disk/by-label/archive";
    fsType = "xfs";
    options = ["ro" "noatime"];
  };

  boot.swraid.enable = false;
  boot.extraModprobeConfig = "options nvme_core io_timeout=30";
}
