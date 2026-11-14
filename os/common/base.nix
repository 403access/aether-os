# Shared base — the ONLY module both targets may import
#
# Allowed here: target-agnostic presentation and Nix configuration.
# Forbidden here: hardware, kernel, storage, users, security posture, services,
# VM devices. Those belong to os/nixos (physical) or os/vm (development).
{
  lib,
  pkgs,
  ...
}: {
  i18n.defaultLocale = "en_US.UTF-8";
  console.keyMap = "us";

  nix.settings.experimental-features = ["nix-command" "flakes"];

  # EGL/GBM runtime: /run/opengl-driver supplies the GBM DRI backend and the
  # glvnd vendor files that Mesa's libgbm dlopens. Both the native DRM backend
  # and the cage/winit fallback need this.
  hardware.graphics.enable = true;
}
