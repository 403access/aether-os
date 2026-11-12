# Services: audio, device policy and journal limits.
#
# The Aether session units (aetherd, aether-index, aether-shell) are added in
# P1/P3 once their binaries are packaged. Until then the session is started
# manually from tty1 (see os/README.md); there is no display manager.
{
  config,
  lib,
  pkgs,
  ...
}: {
  services.dbus.enable = true;

  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
    jack.enable = false;
    wireplumber.enable = true;

    extraConfig.pipewire."92-aether" = {
      "context.properties" = {
        "default.clock.rate" = 48000;
        "default.clock.quantum" = 1024;
        "default.clock.min-quantum" = 128;
        "default.clock.max-quantum" = 2048;
      };
    };

    wireplumber.extraConfig."50-aether" = {
      "wireplumber.settings" = {
        "device.routes.default-sink-volume" = 0.65;
        "device.routes.default-source-volume" = 0.7;
      };
    };
  };

  # NVMe: let the hardware queues do the scheduling.
  services.udev.extraRules = ''
    ACTION=="add|change", KERNEL=="nvme[0-9]n[0-9]", ATTR{queue/scheduler}="none"
    ACTION=="add|change", KERNEL=="nvme[0-9]n[0-9]", ATTR{queue/nr_requests}="1024"
    ACTION=="add|change", KERNEL=="nvme[0-9]n[0-9]", ATTR{queue/read_ahead_kb}="4096"
  '';

  services.journald.settings.Journal = {
    SystemMaxUse = "500M";
    RuntimeMaxUse = "128M";
    MaxRetentionSec = "2week";
    ForwardToSyslog = "no";
  };

  # No inbound remote access.
  services.openssh.enable = false;
  services.avahi.enable = false;
  services.printing.enable = false;
  services.fwupd.enable = false;

  systemd = {
    # Faster shutdown: 10 s instead of 90 s.
    settings.Manager.DefaultTimeoutStopSec = "10s";
    # No wait-for-network: the shell must paint before the network is up.
    services.NetworkManager-wait-online.enable = false;
  };
}
