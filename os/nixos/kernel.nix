# Custom kernel for the reference machine.
#
# `kestrel.config` is a config fragment, not a full .config. It is appended to
# the nixpkgs defconfig and then olddefconfig runs. Unknown symbols are ignored
# (and logged) so a kernel bump does not require a mechanical rewrite.
{
  pkgs,
  lib,
  ...
}: let
  fragment = builtins.readFile ./kernel/kestrel.config;

  kernel = pkgs.linux.override {
    extraConfig = fragment;
    ignoreConfigErrors = true;
  };
in {
  boot.kernelPackages = lib.mkForce (pkgs.linuxPackagesFor kernel);

  boot.kernelParams = [
    "quiet"
    "loglevel=3"
    "mitigations=auto,nosmt=off"
    "nowatchdog"
    "nmi_watchdog=0"
    "zswap.enabled=0"
    "split_lock_detect=off"
    "intel_pstate=active"
    "audit=1"
    "lsm=landlock,yama,bpf"
  ];

  boot.kernel.sysctl = {
    "kernel.kptr_restrict" = 2;
    "kernel.dmesg_restrict" = 1;
    "kernel.yama.ptrace_scope" = 1;
    "kernel.unprivileged_bpf_disabled" = 1;
    "kernel.perf_event_paranoid" = 2;
    "kernel.unprivileged_userns_clone" = 1; # required for agent sandboxes
    "vm.unprivileged_userfaultfd" = 0;
    "vm.swappiness" = 10;
  };

  # Keep the build honest: if the fragment cannot be applied, fail loudly.
  warnings = lib.optional
    (lib.versionOlder kernel.version "6.6")
    "Aether expects Linux 6.6+ for Landlock ABI v3; update nixpkgs.";
}
