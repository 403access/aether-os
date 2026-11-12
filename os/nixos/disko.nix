# Disk layout for Aether OS.
#
# Usage on the live installer:
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
#   01-esp     1 GiB        vfat  /boot   label AETHER-BOOT
#   02-root    25% of disk  xfs   /       label aether-root
#   03-nix     30% of disk  xfs   /nix    label aether-nix
#   04-scratch 10% of disk  ext4  /scratch label aether-scratch
#   05-home    remaining    xfs   /home   label aether-home
#
# Partition names are numbered so the order is explicit (Nix attrsets are
# sorted by name, not declaration order). Adjust the percentages if your NVMe
# is smaller than ~512 GiB or if you want a larger home.
{
  device ? "/dev/nvme0n1",
}: {
  disko.devices = {
    disk.main = {
      type = "disk";
      inherit device;
      content = {
        type = "gpt";
        partitions = {
          "01-esp" = {
            size = "1G";
            type = "EF00";
            content = {
              type = "filesystem";
              format = "vfat";
              mountpoint = "/boot";
              extraArgs = ["-F" "32" "-n" "AETHER-BOOT"];
              mountOptions = ["umask=0077"];
            };
          };

          "02-root" = {
            size = "25%";
            type = "8300";
            content = {
              type = "filesystem";
              format = "xfs";
              mountpoint = "/";
              extraArgs = ["-L" "aether-root" "-b" "size=4096"];
              mountOptions = ["noatime" "largeio" "inode64"];
            };
          };

          "03-nix" = {
            size = "30%";
            type = "8300";
            content = {
              type = "filesystem";
              format = "xfs";
              mountpoint = "/nix";
              extraArgs = ["-L" "aether-nix"];
              mountOptions = ["noatime" "largeio" "inode64"];
            };
          };

          "04-scratch" = {
            size = "10%";
            type = "8300";
            content = {
              type = "filesystem";
              format = "ext4";
              mountpoint = "/scratch";
              extraArgs = ["-L" "aether-scratch"];
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
              extraArgs = ["-L" "aether-home"];
              mountOptions = ["noatime" "largeio" "inode64"];
            };
          };
        };
      };
    };
  };
}
