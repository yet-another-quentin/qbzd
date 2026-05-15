#!/bin/bash
set -e

# ── XDG_RUNTIME_DIR ──────────────────────────────────────────────────────────
# PipeWire et D-Bus en ont besoin. En container il n'est pas créé
# automatiquement par systemd-logind.
# Préférer /run/user/<uid> (pré-créé dans le Dockerfile), sinon /tmp
_default_runtime="/run/user/$(id -u)"
if [ ! -d "$_default_runtime" ]; then
    _default_runtime="/tmp/runtime-qbzd"
fi
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$_default_runtime}"
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"

# ── D-Bus session ─────────────────────────────────────────────────────────────
# Requis pour MPRIS. Le bus system (PipeWire mod.rt) n'est pas démarré :
# son absence dégrade seulement la priorité RT de PipeWire, sans impact
# sur la lecture audio.
if [ -z "$DBUS_SESSION_BUS_ADDRESS" ]; then
    DBUS_SOCKET="$XDG_RUNTIME_DIR/bus"
    dbus-daemon \
        --session \
        --address="unix:path=$DBUS_SOCKET" \
        --fork \
        --print-pid > /dev/null 2>&1 || true
    export DBUS_SESSION_BUS_ADDRESS="unix:path=$DBUS_SOCKET"
fi

# ── Audio backend ─────────────────────────────────────────────────────────────
AUDIO_BACKEND="${AUDIO_BACKEND:-pipewire}"

if [ "$AUDIO_BACKEND" = "pipewire" ]; then
    # Vérifie si PipeWire tourne déjà (socket hôte monté en volume).
    if [ ! -S "$XDG_RUNTIME_DIR/pipewire-0" ]; then
        echo "[entrypoint] Démarrage de PipeWire..."
        pipewire &
        PIPEWIRE_PID=$!

        # PulseAudio compat (fournit pactl + sink PULSE_SINK)
        pipewire-pulse &

        # Session manager
        wireplumber &

        # Attendre que le socket PipeWire soit prêt
        for i in $(seq 1 20); do
            [ -S "$XDG_RUNTIME_DIR/pipewire-0" ] && break
            sleep 0.2
        done

        if [ ! -S "$XDG_RUNTIME_DIR/pipewire-0" ]; then
            echo "[entrypoint] ERREUR : socket PipeWire absent après 4s" >&2
            exit 1
        fi
        echo "[entrypoint] PipeWire prêt."
    else
        echo "[entrypoint] Socket PipeWire hôte détecté, pas de démarrage local."
    fi
fi

# ── Config ────────────────────────────────────────────────────────────────────
# Copie la config par défaut si aucune config utilisateur n'existe.
CONFIG_DIR="/var/lib/qbzd/config"
CONFIG_FILE="$CONFIG_DIR/qbzd.toml"
mkdir -p "$CONFIG_DIR"

if [ ! -f "$CONFIG_FILE" ]; then
    cp /etc/qbz/qbzd.toml.default "$CONFIG_FILE"
    echo "[entrypoint] Config initiale copiée dans $CONFIG_FILE"
fi

# ── Lancement ─────────────────────────────────────────────────────────────────
echo "[entrypoint] Démarrage de qbzd..."
exec qbzd \
    --config "$CONFIG_FILE" \
    --data-dir /var/lib/qbzd \
    "$@"
