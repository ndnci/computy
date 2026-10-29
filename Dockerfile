# syntax=docker/dockerfile:1
FROM node:22-bookworm-slim AS node

FROM rust:1.99.1-bookworm AS toolchain
COPY --from=node /usr/local/ /usr/local/
RUN apt-get update && apt-get install -y --no-install-recommends \
    libwebkit2gtk-4.1-dev build-essential curl file libssl-dev \
    libayatana-appindicator3-dev librsvg2-dev patchelf ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace
ENV CARGO_BUILD_JOBS=2

FROM toolchain AS desktop-qa
RUN apt-get update && apt-get install -y --no-install-recommends \
    xvfb xauth dbus-x11 webkit2gtk-driver fonts-noto-core fonts-noto-cjk python3-dbus python3-gi \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install tauri-driver --version 2.0.6 --locked
RUN useradd --uid 1000 --create-home computy
USER computy

FROM toolchain AS boot-qa
RUN apt-get update && apt-get install -y --no-install-recommends \
    qemu-system-x86 ovmf xorriso grub-pc-bin grub-efi-amd64-bin mtools dosfstools \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --uid 1000 --create-home computy
USER computy

FROM toolchain AS sources
COPY package.json package-lock.json ./
RUN npm ci
COPY . .

FROM sources AS verify
RUN cargo test --locked --manifest-path native/Cargo.toml
RUN cargo build --locked --release --manifest-path native/Cargo.toml
RUN npm test && npm run build
RUN cargo check --locked --manifest-path src-tauri/Cargo.toml

FROM sources AS desktop-build
RUN npm run package -- --bundles deb

FROM node AS dev
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace
COPY --from=verify /workspace/node_modules ./node_modules
COPY --from=verify /workspace/native/target/release/computy-cli /usr/local/bin/computy-cli
COPY . .
ENV COMPUTY_ENGINE_PATH=/usr/local/bin/computy-cli COMPUTY_DATA_DIR=/data COMPUTY_CONTAINER=1
RUN mkdir /data && chown node:node /data
USER node
EXPOSE 1420
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]
