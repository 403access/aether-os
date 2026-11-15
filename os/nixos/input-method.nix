# Input methods are opt-in.
#
# fcitx5 plus its addons is the largest optional process in an otherwise
# minimal session, and every GTK/Qt client that sees GTK_IM_MODULE or
# QT_IM_MODULE loads the corresponding input-method module into its own
# address space. The reference operator does not need one by default.
#
# Enable it for CJK/IME input by adding this to configuration.nix and
# rebuilding:
#
#   aether.inputMethod.enable = true;
#
# Then measure the attributed footprint with scripts/memory-report.sh before
# and after the rebuild (see architecture/memory-footprint.md).
{
  config,
  lib,
  ...
}: {
  options.aether.inputMethod.enable = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = ''
      Enable fcitx5 as the session input method. Off by default because it is
      the largest optional process in an otherwise minimal session; enable it
      for CJK/IME input.
    '';
  };

  config.i18n.inputMethod.enable = config.aether.inputMethod.enable;
  config.i18n.inputMethod.type = lib.mkIf config.aether.inputMethod.enable "fcitx5";
}
