# syntax=docker/dockerfile:1.7
ARG RUST_IMAGE=rust:1.98.0-bookworm
# Selkies owns streaming; Shell replaces its Labwc binary with the pinned
# control endpoint used by pelagian-layoutd.
ARG SELKIES_BASE_IMAGE=ghcr.io/linuxserver/baseimage-selkies:debiantrixie@sha256:ac7fd6d182238b4a99e66554c5e75be48a714e2a0c9da81bd18e171ff9ba3dd5
FROM ${RUST_IMAGE} AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --locked

FROM ${SELKIES_BASE_IMAGE} AS labwc-builder
ARG LABWC_COMMIT=f0dbad27fcf6e388cad1aa32448a1d548da68990
COPY labwc/ipc-control.patch /tmp/ipc-control.patch
RUN set -eux; \
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get build-dep --no-install-recommends -y labwc; \
    DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y hwdata; \
    git clone https://github.com/labwc/labwc.git /tmp/labwc; \
    cd /tmp/labwc; \
    git checkout "$LABWC_COMMIT"; \
    git apply --unidiff-zero /tmp/ipc-control.patch; \
    meson setup build --prefix=/usr --libdir=lib/x86_64-linux-gnu -Dxwayland=enabled -Dnls=enabled; \
    ninja -C build; \
    ninja -C build install

FROM ${SELKIES_BASE_IMAGE}
ARG VERSION=0.1.0
ARG REVISION=unknown

RUN set -eux; \
    apt-get update; \
    DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y \
        gir1.2-gtk-3.0 \
        python3-gi \
        wlr-randr; \
    rm -rf /var/lib/apt/lists/*

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
    PELORUS=false

COPY --from=build /src/target/release/pelagian-shellctl /usr/local/bin/pelagian-shellctl
COPY --from=build /src/target/release/pelagian-layoutd /usr/local/bin/pelagian-layoutd
COPY --from=labwc-builder /usr/bin/labwc /usr/bin/labwc
COPY config/ /usr/share/pelagian-shell/
COPY labwc/rc.xml /defaults/labwc.xml
COPY ["labwc/theme/Pelagian Shell/", "/usr/share/themes/Pelagian Shell/"]
COPY session/startwm_wayland.sh /defaults/startwm_wayland.sh
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
        /defaults/startwm_wayland.sh \
        /custom-cont-init.d/20-pelagian-shell-config \
        /usr/local/bin/pelagian-shell-apply-wine-defaults; \
    sh -n /defaults/autostart; \
    sh -n /defaults/autostart_wayland; \
    sh -n /defaults/startwm_wayland.sh; \
    sh -n /custom-cont-init.d/20-pelagian-shell-config; \
    test -x /usr/local/bin/pelagian-shellctl; \
    test -x /usr/local/bin/pelagian-layoutd; \
    test -x /lsiopy/bin/selkies; \
    command -v labwc; \
    command -v wlr-randr; \
    PELAGIAN_SHELL_DATA_DIR=/usr/share/pelagian-shell \
        PELAGIAN_SHELL_ETC_DIR=/etc/pelagian-shell \
        /usr/local/bin/pelagian-shellctl config show >/dev/null; \
    /usr/local/bin/pelagian-layoutd status

EXPOSE 3001
VOLUME ["/config"]
