# Hardware facts for the reference machine: Intel Core i7-7700K (Kaby Lake),
# HD Graphics 630, NVMe root, analog + USB + Bluetooth audio.
#
# Every value here is a claim about the machine. If a device is different on
# your build, change this file and update the hardware dossier in the docs.
{
  config,
  lib,
  pkgs,
  modulesPath,
  ...
}: {
  imports = [
    (modulesPath + "/installer/scan/not-detected.nix")
  ];

  boot.initrd = {
    availableKernelModules = [
      "nvme"
      "xhci_pci"
      "usbhid"
      "usb_storage"
      "sd_mod"
      "xfs"
      "ext4"
      "aesni_intel"
      "cryptd"
    ];
    kernelModules = ["i915"];
  };

  boot.kernelModules = [
    "kvm-intel"
    "coretemp"
    "i915"
    "snd_hda_intel"
    "btusb"
  ];

  boot.extraModprobeConfig = ''
    # Keep HDA codecs powered; the idle budget assumes no runtime PM churn.
    options snd_hda_intel power_save=0 power_save_controller=N
  '';

  hardware = {
    cpu.intel.updateMicrocode = true;
    enableRedistributableFirmware = true;
    graphics = {
      enable = true;
      extraPackages = with pkgs; [
        intel-media-driver
        intel-vaapi-driver
        libvdpau-va-gl
      ];
    };
    bluetooth.enable = true;
  };

  # No battery, no lid, no backlight: desktop.
  powerManagement.cpuFreqGovernor = "performance";

  services = {
    # No thermald: this is a desktop board without DPTF. It exits on most
    # Kaby Lake desktops and leaves a failed unit in the first-boot checklist.
    fstrim.enable = true;
    irqbalance.enable = true;
  };

  nix.settings.system-features = ["nixos-test" "benchmark" "big-parallel" "kvm"];

  # 32–64 GiB assumed; the agent budget is configured explicitly in aetherd.
  fileSystems."/boot" = {
    device = "/dev/disk/by-label/AETHER-BOOT";
    fsType = "vfat";
    options = ["fmask=0077" "dmask=0077"];
  };
}
