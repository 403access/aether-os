# Headless operation is opt-in.
#
# The whole UI is one systemd service: aether-session owns tty7 and the DRM
# seat. Everything else (index, MCP, sandboxes, chat, audio, network) is
# display-independent, so a server install only has to stop pulling that one
# unit. Keep this off for the workstation; enable it on a machine that is
# administered over SSH or a serial console:
#
#   aether.headless.enable = true;
#
# This module intentionally keeps the firewall enabled and SSH password
# authentication disabled. Add a key before relying on remote login:
#
#   users.users.oliver.openssh.authorizedKeys.keys = [ "ssh-ed25519 ..." ];
#
# No headless compositor backend exists: the shell needs a DRM connector or a
# seat. `systemctl start aether-session` still works on demand for a temporary
# local UI, and the remote-display service remains opt-in inside that session.
{
  config,
  lib,
  ...
}: let
  cfg = config.aether.headless;
in {
  options.aether.headless = {
    enable = lib.mkEnableOption ''
      a no-UI server profile: the compositor session is not started at boot
      and the console login no longer auto-logs-in the operator
    '';

    ssh = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Enable OpenSSH for remote administration. The firewall stays enabled and
        only port 22 is opened. Password and keyboard-interactive authentication
        stay disabled; add an authorized key for `oliver` before relying on it.
      '';
    };

    serialConsole = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Add `console=ttyS0,115200` to the kernel command line so systemd spawns
        a getty on the serial port. The local tty0 console remains available.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    # Do not pull the compositor at boot. The unit stays installed and can be
    # started by hand for a temporary local session.
    systemd.services.aether-session.wantedBy = lib.mkForce [];

    # Clipboard persistence is a Wayland client; it has nothing to attach to
    # without the compositor.
    systemd.user.services.aether-clip-persist.wantedBy = lib.mkForce [];

    # A server console should not hand out an operator shell without a
    # password. Log in on tty1 as `oliver` (the installed initial password is
    # documented in first-boot) and change it immediately.
    services.getty.autologinUser = lib.mkForce null;

    # User services (aether-index, aether-chat) must run before and without an
    # interactive login.
    users.users.oliver.linger = true;

    services.openssh = lib.mkIf cfg.ssh {
      # services.nix disables sshd for the workstation ("no inbound remote
      # access"); the profile is an explicit, documented override.
      enable = lib.mkForce true;
      settings = {
        PasswordAuthentication = false;
        KbdInteractiveAuthentication = false;
        PermitRootLogin = "no";
      };
    };

    networking.firewall.allowedTCPPorts =
      lib.mkIf cfg.ssh [22];

    boot.kernelParams =
      lib.mkIf cfg.serialConsole ["console=ttyS0,115200"];
  };
}