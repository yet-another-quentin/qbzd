# ─────────────────────────────────────────────
# Stage 1 — build
# ─────────────────────────────────────────────
FROM rust:1-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libasound2-dev \
    libdbus-1-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY crates/ ./crates/

RUN cd crates && cargo build --release -p qbzd

# ─────────────────────────────────────────────
# Stage 2 — runtime
# ─────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    # Audio — PipeWire + PulseAudio compat layer + ALSA
    pipewire \
    pipewire-pulse \
    pipewire-alsa \
    wireplumber \
    alsa-utils \
    # D-Bus (MPRIS + zbus device reservation + dbus-launch)
    dbus \
    dbus-x11 \
    # CA certificates (HTTPS vers Qobuz)
    ca-certificates \
    # pw-metadata + pactl utilisés par le backend PipeWire
    pipewire-bin \
    pulseaudio-utils \
    && rm -rf /var/lib/apt/lists/*

# Utilisateur dédié — membre du groupe audio pour accès /dev/snd
RUN groupadd -r qbzd \
    && useradd -r -g qbzd -G audio -m -d /var/lib/qbzd -s /sbin/nologin qbzd

COPY --from=builder /build/crates/target/release/qbzd /usr/local/bin/qbzd
COPY docker/entrypoint.sh /entrypoint.sh
COPY docker/qbzd.toml /etc/qbz/qbzd.toml.default

RUN chmod +x /entrypoint.sh \
    && mkdir -p /var/lib/qbzd \
    && chown qbzd:qbzd /var/lib/qbzd \
    && mkdir -p /run/user/$(id -u qbzd) \
    && chown qbzd:qbzd /run/user/$(id -u qbzd) \
    && chmod 700 /run/user/$(id -u qbzd)

# Données persistantes (tokens, cache, db)
VOLUME ["/var/lib/qbzd"]

# API HTTP
EXPOSE 8182

USER qbzd
ENTRYPOINT ["/entrypoint.sh"]
