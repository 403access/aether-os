# Disk layout for Aether OS.
#
# Usage on the live installer, from the repository root:
#
#   sudo nix run github:nix-community/disko -- --mode disko --flake .#kestrel
#
# or, with a different device without editing files:
#
#   sudo nix run github:nix-community/disko -- \
#     --mode disko --arg device '"/dev/nvme1n1"' ./os/nixos/disko.nix
#
# THIS WIPES THE DISK. It creates a GPT table with:
#
#   01-esp     1 GiB    vfat  /boot    label AETHER-BOOT
#   02-root    80 GiB   xfs   /        label aether-root
#   03-nix     120 GiB  xfs   /nix     label aether-nix
#   04-scratch 60 GiB   ext4  /scratch label aether-scratch
#   05-home    rest     xfs   /home    label aether-home
#
# Partition names are numbered so the order is explicit (Nix attrsets are
# sorted by name, not declaration order). disko only supports absolute sizes
# or "100%" (the remaining disk), not arbitrary percentages. Reduce the fixed
# sizes if your NVMe is smaller than ~512 GiB.
{ device ? "/dev/nvme0n1" }:
let
  labels = {
    esp = "AETHER-BOOT";
    root = "aether-root";
    nix = "aether-nix";
    scratch = "aether-scratch";
    home = "aether-home";
  };

  partitions = {
    "01-esp" = {
      size = "1G";
      type = "EF00";
      content = {
        type = "filesystem";
        format = "vfat";
        mountpoint = "/boot";
        extraArgs = ["-F" "32" "-n" labels.esp];
        mountOptions = ["umask=0077"];
      };
    };

    "02-root" = {
      size = "80G";
      type = "8300";
      content = {
        type = "filesystem";
        format = "xfs";
        mountpoint = "/";
        extraArgs = ["-L" labels.root "-b" "size=4096"];
        mountOptions = ["noatime" "largeio" "inode64"];
      };
    };

    "03-nix" = {
      size = "120G";
      type = "8300";
      content = {
        type = "filesystem";
        format = "xfs";
        mountpoint = "/nix";
        extraArgs = ["-L" labels.nix];
        mountOptions = ["noatime" "largeio" "inode64"];
      };
    };

    "04-scratch" = {
      size = "60G";
      type = "8300";
      content = {
        type = "filesystem";
        format = "ext4";
        mountpoint = "/scratch";
        extraArgs = ["-L" labels.scratch];
        mountOptions = ["noatime" "nodiratime" "commit=60" "data=writeback"];
      };
    };

    "05-home" = {
      size = "100%";
      type = "8300";
      content = {
        type = "filesystem";
        format = "xfs";
        mountpoint = "/home";
        extraArgs = ["-L" labels.home];
        mountOptions = ["noatime" "largeio" "inode64"];
      };
    };
  };

  sizes = map (partition: partition.size) (builtins.attrValues partitions);
  remaining = builtins.filter (size: size == "100%") sizes;
  sizePattern = "[0-9]+[KMGTP]?";
in
# Guard rails for the two mistakes that make an install fail late: an
# unsupported size expression, or filesystem labels the kernel cannot match.
assert builtins.all (size: size == "100%" || builtins.match sizePattern size != null) sizes;
assert builtins.length remaining == 1;
assert builtins.stringLength labels.esp <= 11; # FAT
assert builtins.all (label: builtins.stringLength label <= 12) [
  labels.root
  labels.nix
  labels.home
]; # XFS
assert builtins.stringLength labels.scratch <= 16; # ext4
{
  disko.devices = {
    disk.main = {
      type = "disk";
      inherit device;
      content = {
        type = "gpt";
        inherit partitions;
      };
    };
  };
}
