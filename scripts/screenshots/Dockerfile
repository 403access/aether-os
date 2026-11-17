# Linux capture environment for the Smithay Xvfb screenshot smokes.
#
# Build context: repository root.
#   docker build -f scripts/screenshots/Dockerfile -t aether-shell-capture:local .
#
# scripts/capture-shell-screenshots.sh builds this image on demand. Keeping the
# build inside Linux makes the screenshots reproducible on macOS, where the
# compositor cannot run natively.
FROM rust:1.99.1-trixie

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config build-essential git ca-certificates \
    libwayland-dev wayland-protocols libxkbcommon-dev libudev-dev libseat-dev \
    libinput-dev libgbm-dev libegl1-mesa-dev libgles2-mesa-dev \
    libx11-dev libxcursor-dev libxi-dev libxrandr-dev libxinerama-dev libxext-dev \
    libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libpixman-1-dev libdbus-1-dev libsystemd-dev \
    xvfb xdotool imagemagick xauth \
    libxkbcommon-x11-0 libgl1-mesa-dri mesa-utils \
    fonts-inter-variable fonts-dejavu-core \
    weston foot wl-clipboard wayland-utils \
    python3 curl unzip \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY os/compositor /src/os/compositor
COPY scripts/check-explorer-screenshot.sh /src/scripts/check-explorer-screenshot.sh
RUN chmod +x /src/scripts/check-explorer-screenshot.sh

RUN cargo build --manifest-path /src/os/compositor/aether-shell-smithay/Cargo.toml

# Bun and the JavaScript indexer for the full-text palette captures. The
# indexer imports the workspace protocol package directly, so link just those
# two packages instead of installing the whole workspace.
COPY apps/indexer-service /src/apps/indexer-service
COPY packages/indexer /src/packages/indexer
COPY packages/protocol /src/packages/protocol
RUN curl -fsSL https://bun.sh/install | bash -s "bun-v1.4.4" \
    && ln -sf /root/.bun/bin/bun /usr/local/bin/bun \
    && mkdir -p /src/node_modules/@aether \
    && ln -s /src/packages/protocol /src/node_modules/@aether/protocol \
    && ln -s /src/packages/indexer /src/node_modules/@aether/indexer

# Debian's foot is older than the version the repo config targets: it uses the
# flat [colors] section, has no [colors-light] split and no cursor colour pair.
# Translate the capture copy in place so the terminal matches the Water palette
# instead of falling back to the default scheme. Keep this after the Rust build
# so config-only changes reuse the cached cargo layers.
RUN awk '/^\[colors-light\]/{skip=1;next} /^\[csd\]/{skip=0} !skip' \
      /src/os/compositor/aether-shell-smithay/foot/foot.ini \
      | sed -e 's/^\[colors-dark\]/[colors]/' -e '/^cursor=/d' \
      > /tmp/foot.ini \
    && mv /tmp/foot.ini /src/os/compositor/aether-shell-smithay/foot/foot.ini

# Keep the container identity out of terminal captures.
RUN printf 'export PS1="aether@kestrel ~ %% "\n' > /root/.bashrc \
    && printf '[ -f /root/.bashrc ] && . /root/.bashrc\n' > /root/.bash_profile

ENV AETHER_FONT=/usr/share/fonts/truetype/inter-vf/InterVariable.ttf
ENV BIN=/src/os/compositor/aether-shell-smithay/target/debug/aether-shell-smithay
ENV LIBGL_ALWAYS_SOFTWARE=1
ENV MESA_LOADER_DRIVER_OVERRIDE=llvmpipe

CMD ["bash"]