# Services: audio, device policy and journal limits.
#
# The compositor session and aether-index daemon are packaged and enabled here.
# aetherd (P5) is future work; there is no display manager, the native session
# starts on tty7 and falls back to cage.
{
  config,
  lib,
  pkgs,
  ...
}: {
  services.dbus.enable = true;

  # QuickSync encode/decode for the remote-display fallback. The software H.264
  # path still works if the iHD driver is unavailable; the shell labels it.
  hardware.graphics.enable = true;
  hardware.graphics.extraPackages = [pkgs.intel-media-driver];
  environment.sessionVariables = {
    LIBVA_DRIVER_NAME = "iHD";
  };

  # Bluetooth transport for headsets and speakers. WirePlumber owns the
  # profile/codec choice; aether-audio only reports and switches routes.
  hardware.bluetooth.enable = true;
  hardware.bluetooth.powerOnBoot = true;

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
        "default.clock.min-quantum" = 64;
        "default.clock.max-quantum" = 2048;
        "link.max-buffers" = 16;
      };
    };

    wireplumber.extraConfig."50-aether" = {
      "wireplumber.settings" = {
        "device.routes.default-sink-volume" = 0.65;
        "device.routes.default-source-volume" = 0.7;
      };
      # Prefer quality codecs when the device advertises them. SBC remains the
      # compatibility floor; the active codec is reported by aether-audio.
      "monitor.bluez.properties" = {
        "bluez5.enable-sbc-xq" = true;
        "bluez5.enable-msbc" = true;
        "bluez5.enable-hw-volume" = true;
        "bluez5.codecs" = ["ldac" "aptx_hd" "aptx" "aac" "sbc"];
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

  # DRM session. aether-shell-drm is the native single-process backend: it
  # owns seat0, libinput and KMS page-flips directly. If it cannot start
  # (no seat access, unknown GPU, crash) the wrapper falls back to cage
  # running the winit adapter fullscreen. Policy: ADR-0017.
  systemd.services.aether-session = {
    description = "Aether session (native DRM, cage fallback)";
    wantedBy = ["multi-user.target"];
    after = ["systemd-user-sessions.service"];
    serviceConfig = {
      User = "oliver";
      # Smithay dlopens libEGL/libgbm; give it the store paths directly so it
      # does not depend on the login shell's graphics environment.
      Environment = [
        "AETHER_FONT=${pkgs.inter}/share/fonts/truetype/InterVariable.ttf"
        "LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [ pkgs.libGL pkgs.mesa pkgs.libgbm ]}"
        "LIBGL_DRIVERS_PATH=${pkgs.mesa}/lib/dri"
        "GBM_BACKENDS_PATH=${pkgs.mesa}/lib/gbm"
      ];
      PAMName = "login";
      TTYPath = "/dev/tty7";
      StandardInput = "tty";
      StandardOutput = "journal";
      ExecStart = "${pkgs.aether-session}/bin/aether-session";
      Restart = "on-failure";
      RestartSec = 2;
    };
  };

  # System index daemon: inotify ingestion, Unix-socket queries and a ring of
  # change events. The shell queries the socket asynchronously.
  systemd.user.services.aether-index = {
    description = "Aether system index daemon";
    wantedBy = ["default.target"];
    serviceConfig = {
      ExecStart = "${pkgs.aether-index}/bin/aether-index";
      Environment = [
        "AETHER_INDEX_SOCKET=%t/aether-index.sock"
        "AETHER_INDEX_RING=%t/aether-index.ring"
        "AETHER_INDEX_ROOTS=%h"
        "AETHER_INDEX_LIMIT=20000"
      ];
      Restart = "on-failure";
      RestartSec = 2;
    };
  };

  # Remote display signaling is opt-in and local-only: the HTTP listener binds
  # 127.0.0.1 and the firewall gains no inbound rule. Start it for a session
  # with `systemctl --user start aether-remote`; the shell reads the published
  # status document through `Super+R`.
  systemd.user.services.aether-remote = {
    description = "Aether remote display signaling (local-only, opt-in)";
    serviceConfig = {
      ExecStart = "${pkgs.aether-remote}/bin/aether-remote serve --http 127.0.0.1:7443 --udp 127.0.0.1:0 --status %t/aether/remote-session.json";
      Environment = [
        "LIBVA_DRIVER_NAME=iHD"
        "AETHER_REMOTE_STATUS=%t/aether/remote-session.json"
      ];
      RuntimeDirectory = "aether";
      Restart = "on-failure";
      RestartSec = 2;
      NoNewPrivileges = true;
      PrivateTmp = true;
      ProtectSystem = "strict";
      ProtectHome = "read-only";
      ReadWritePaths = ["%t/aether"];
      RestrictAddressFamilies = ["AF_INET" "AF_INET6" "AF_UNIX"];
    };
    # Not wantedBy default. The host starts it explicitly for a session.
    wantedBy = [];
  };

  # Chat backend: the built-in chat drawer talks to this process, which owns
  # the model loop and the Aether MCP client. No model ships with the OS; set
  # AETHER_CHAT_ENDPOINT in ~/.config/aether/chat.env to point at a local
  # OpenAI-compatible endpoint. Without it the drawer reports the missing
  # configuration instead of failing silently.
  systemd.user.services.aether-chat = {
    description = "Aether chat backend (model loop + MCP client)";
    wantedBy = ["default.target"];
    partOf = ["graphical-session.target"];
    after = ["graphical-session.target"];
    serviceConfig = {
      ExecStart = "${pkgs.aether-mcp}/bin/aether-chat";
      Environment = [
        "AETHER_CHAT_SOCKET=%t/aether-chat.sock"
        "AETHER_MCP_BIN=${pkgs.aether-mcp}/bin/aether-mcp"
        "AETHER_CHAT_MODEL=deepseek-chat"
      ];
      EnvironmentFile = "-%h/.config/aether/chat.env";
      Restart = "on-failure";
      RestartSec = 2;
    };
  };

  # Keep clipboard data after the source client exits. wlr-data-control is
  # advertised by the compositor; the helper re-owns the selection.
  systemd.user.services.aether-clip-persist = {
    description = "Aether clipboard persistence";
    wantedBy = ["default.target"];
    serviceConfig = {
      ExecStart = "${pkgs.wl-clip-persist}/bin/wl-clip-persist --clipboard both";
      Environment = ["WAYLAND_DISPLAY=aether-0"];
      Restart = "on-failure";
      RestartSec = 2;
    };
  };

  systemd = {
    # Faster shutdown: 10 s instead of 90 s.
    settings.Manager.DefaultTimeoutStopSec = "10s";
    # No wait-for-network: the shell must paint before the network is up.
    services.NetworkManager-wait-online.enable = false;
  };
}
