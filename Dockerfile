# syntax=docker/dockerfile:1
FROM node:22-bookworm-slim AS node

# Native GPUI desktop toolchain. Xvfb, sway (headless) and Mesa's software
# Vulkan driver (lavapipe) render the interface for visual regression captures.
FROM rust:1.100.0-bookworm AS gpui
COPY --from=node /usr/local/ /usr/local/
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev libfontconfig1-dev libfreetype6-dev libasound2-dev \
    libgbm-dev libvulkan-dev libwayland-dev libx11-dev libx11-xcb-dev libxcb1-dev libxcb-xkb-dev \
    libxkbcommon-dev libxkbcommon-x11-dev libxcursor-dev libxrandr-dev libxinerama-dev libxi-dev \
    libxrender-dev libdbus-1-dev mesa-vulkan-drivers vulkan-tools xvfb xauth x11-utils \
    imagemagick dbus-x11 fonts-noto-core fonts-noto-cjk dpkg-dev fakeroot file sway grim \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy
ENV CARGO_HOME=/cargo
WORKDIR /workspace

FROM gpui AS boot-qa
RUN apt-get update && apt-get install -y --no-install-recommends \
    qemu-system-x86 ovmf xorriso grub-pc-bin grub-efi-amd64-bin mtools dosfstools \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --uid 1000 --create-home computy
USER computy

FROM gpui AS sources
COPY package.json package-lock.json ./
RUN npm ci
COPY . .

FROM sources AS verify
RUN cargo test --locked --manifest-path native/Cargo.toml
RUN cargo test --locked --manifest-path desktop/Cargo.toml
RUN cargo build --locked --release --manifest-path native/Cargo.toml
RUN npm test

FROM sources AS desktop-build
RUN scripts/package-deb.sh
