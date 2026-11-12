# Accounts. One interactive user, one service user, a reserved UID range for
# agents. Agents are never interactive and never own files outside their jail.
{
  config,
  lib,
  pkgs,
  ...
}: {
  users.mutableUsers = true;

  users.users.oliver = {
    isNormalUser = true;
    description = "Operator";
    home = "/home/oliver";
    shell = pkgs.bash;
    # Change immediately after first boot with `passwd`.
    initialPassword = "aether";
    extraGroups = [
      "wheel"
      "audio"
      "video"
      "input"
      "render"
      "networkmanager"
      "aether-agents"
    ];
  };

  users.users.aether = {
    isSystemUser = true;
    group = "aether";
    home = "/var/lib/aether";
    createHome = true;
    description = "Aether supervisor and index";
  };

  users.groups.aether = {};
  users.groups.aether-agents = {gid = 4000;};

  # Agent UIDs are allocated from 4001..4095 by aetherd at spawn time. They are
  # not declared here because they are ephemeral; the group is the invariant.

  security.sudo = {
    enable = true;
    wheelNeedsPassword = true;
    execWheelOnly = true;
  };

  # No passwordless sudo. Debugging a broken agent profile is not a reason to
  # weaken the host.
  security.sudo.extraConfig = ''
    Defaults  env_reset, timestamp_timeout=5
  '';
}
