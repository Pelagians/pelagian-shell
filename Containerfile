# syntax=docker/dockerfile:1.7
ARG RUST_IMAGE=rust:1.98.0-bookworm
# Same exact Selkies base currently pinned by Grotto. See docs/reference-runtime.md
# for the upstream-Labwc provenance decision still required before release.
ARG SELKIES_BASE_IMAGE=ghcr.io/linuxserver/baseimage-selkies:debiantrixie@sha256:ac7fd6d182238b4a99e66554c5e75be48a714e2a0c9da81bd18e171ff9ba3dd5
FROM ${RUST_IMAGE} AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --locked

FROM ${SELKIES_BASE_IMAGE}
ARG VERSION=0.1.0
ARG REVISION=unknown

LABEL org.opencontainers.image.title="Pelagian Shell" \
    org.opencontainers.image.description="Selkies/Labwc workspace substrate for Pelagian streamed workloads" \
    org.opencontainers.image.source="https://github.com/Pelagians/pelagian-shell" \
    org.opencontainers.image.revision="${REVISION}" \
    org.opencontainers.image.version="${VERSION}"

ENV TITLE="Pelagian Shell" \
    START_DOCKER=false \
    PIXELFLUX_WAYLAND=true \
    AUTO_GPU=true \
    RESTART_APP=false \
    SELKIES_DESKTOP=false \
    PELORUS=false \
    PELAGIAN_LAYOUTD_STATE=/config/.local/state/pelagian-shell/layoutd-status.json

RUN set -eux; \
    apt-get update; \
    apt-get install --no-install-recommends -y wmctrl x11-utils util-linux; \
    rm -rf /var/lib/apt/lists/*

COPY --from=build /src/target/release/pelagian-shellctl /usr/local/bin/pelagian-shellctl
COPY --from=build /src/target/release/pelagian-layoutd /usr/local/bin/pelagian-layoutd
COPY config/ /usr/share/pelagian-shell/
COPY labwc/rc.xml /defaults/labwc.xml
COPY ["labwc/theme/Pelagian Shell/", "/usr/share/themes/Pelagian Shell/"]
COPY session/autostart_wayland /defaults/autostart_wayland
COPY session/autostart /defaults/autostart
COPY session/20-pelagian-shell-config /custom-cont-init.d/20-pelagian-shell-config
COPY theme/ /usr/share/pelagian-shell/theme/
COPY wine/pelagian-shell.reg /usr/share/pelagian-shell/wine/pelagian-shell.reg
COPY wine/apply-defaults.sh /usr/local/bin/pelagian-shell-apply-wine-defaults

RUN set -eux; \
    chmod 0755 \
        /defaults/autostart \
        /defaults/autostart_wayland \
        /custom-cont-init.d/20-pelagian-shell-config \
        /usr/local/bin/pelagian-shell-apply-wine-defaults; \
    sh -n /defaults/autostart; \
    sh -n /defaults/autostart_wayland; \
    sh -n /custom-cont-init.d/20-pelagian-shell-config; \
    test -x /usr/local/bin/pelagian-shellctl; \
    test -x /usr/local/bin/pelagian-layoutd; \
    test -x /lsiopy/bin/selkies; \
    command -v labwc; \
    command -v flock; \
    command -v wmctrl; \
    command -v xprop; \
    command -v xwininfo; \
    command -v xmessage; \
    PELAGIAN_SHELL_DATA_DIR=/usr/share/pelagian-shell \
        PELAGIAN_SHELL_ETC_DIR=/etc/pelagian-shell \
        /usr/local/bin/pelagian-shellctl config show >/dev/null; \
    /usr/local/bin/pelagian-layoutd status

EXPOSE 3001
VOLUME ["/config"]
