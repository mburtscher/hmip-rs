# syntax=docker/dockerfile:1
#
# Generic image for every plugin in plugins/<name>. Use scripts/build-plugin.sh rather than
# calling this directly:
#   docker buildx build --platform linux/arm64 --build-arg PLUGIN=fronius ... -f docker/plugin.Dockerfile .
#
# The binary is cross-compiled for aarch64 on the build host (no emulation) and ends up alone in
# a `scratch` image.

FROM --platform=$BUILDPLATFORM ghcr.io/rust-cross/cargo-zigbuild:latest AS build
ARG PLUGIN
ARG TARGET=aarch64-unknown-linux-musl
WORKDIR /src
RUN rustup target add "$TARGET"
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo zigbuild --release --locked --package "$PLUGIN" --target "$TARGET" \
 && cp "target/$TARGET/release/$PLUGIN" /plugin

FROM scratch
ARG VERSION
ARG HCU_METADATA
COPY --from=build /plugin /plugin
# Persistent plugin data; the HCU keeps /data across updates and restarts.
WORKDIR /data
ENTRYPOINT ["/plugin"]
LABEL de.eq3.hmip.plugin.metadata="${HCU_METADATA}" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.source="https://github.com/mburtscher/HomematicIP.Net" \
      org.opencontainers.image.licenses="MIT"
