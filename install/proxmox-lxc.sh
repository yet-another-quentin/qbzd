#!/usr/bin/env bash
# qbzd — Proxmox LXC installer
#
# Usage (on the Proxmox host, as root):
#   bash <(curl -fsSL https://raw.githubusercontent.com/qbarlas/qbzd/main/install/proxmox-lxc.sh)
#
# Environment overrides:
#   CTID=200 MEMORY=512 DISK=4 STORAGE=local-lvm AUDIO=alsa bash <(...)

set -euo pipefail

# ── Colors ────────────────────────────────────────────────────────────────────
GN="\033[32m" RD="\033[31m" YW="\033[33m" BL="\033[36m" CL="\033[0m" BF="\033[1m"
msg()  { echo -e "${BL}▶${CL} $*"; }
ok()   { echo -e "${GN}✔${CL} $*"; }
warn() { echo -e "${YW}⚠${CL} $*"; }
die()  { echo -e "${RD}✘${CL} $*" >&2; exit 1; }

# ── Prerequisites ─────────────────────────────────────────────────────────────
[[ $EUID -eq 0 ]]         || die "This script must be run as root on the Proxmox host."
command -v pct >/dev/null || die "pct not found — this script must run on a Proxmox host."
command -v pvesh >/dev/null || die "pvesh not found."

# ── Parameters ────────────────────────────────────────────────────────────────
ARCH=$(dpkg --print-architecture 2>/dev/null || uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')
CTID="${CTID:-$(pvesh get /cluster/nextid 2>/dev/null || echo 200)}"
HOSTNAME="${HOSTNAME:-qbzd}"
MEMORY="${MEMORY:-256}"
DISK="${DISK:-2}"
CORES="${CORES:-1}"
BRIDGE="${BRIDGE:-vmbr0}"
STORAGE="${STORAGE:-local-lvm}"
# Audio backend: alsa | pipewire | none
AUDIO="${AUDIO:-alsa}"
# For PipeWire: UID of the host user owning the socket
PIPEWIRE_HOST_UID="${PIPEWIRE_HOST_UID:-1000}"

echo
echo -e "${BF}  qbzd — Proxmox LXC installer${CL}"
echo    "  ─────────────────────────────"
echo    "  Container ID  : $CTID"
echo    "  Hostname      : $HOSTNAME"
echo    "  RAM / Disk    : ${MEMORY} MB / ${DISK} GB"
echo    "  Storage       : $STORAGE"
echo    "  Arch          : $ARCH"
echo    "  Audio         : $AUDIO"
echo
read -r -p "  Continue? [Y/n] " _confirm
[[ "${_confirm:-Y}" =~ ^[Yy]$ ]] || { echo "Aborted."; exit 0; }
echo

# ── Debian 12 template ────────────────────────────────────────────────────────
msg "Looking for a Debian 12 template..."
pveam update --section system >/dev/null 2>&1 || true

TEMPLATE_NAME=$(pveam available --section system 2>/dev/null \
    | awk '/debian-12-standard/ {print $2}' | sort -V | tail -1)
[[ -n "$TEMPLATE_NAME" ]] || die "debian-12-standard template not found in pveam."

TEMPLATE_STORAGE="local"
if [[ ! -f "/var/lib/vz/template/cache/$TEMPLATE_NAME" ]]; then
    msg "Downloading template $TEMPLATE_NAME..."
    pveam download "$TEMPLATE_STORAGE" "$TEMPLATE_NAME"
fi
ok "Template ready: $TEMPLATE_NAME"

# ── Create container ──────────────────────────────────────────────────────────
msg "Creating LXC container $CTID..."
pct create "$CTID" "$TEMPLATE_STORAGE:vztmpl/$TEMPLATE_NAME" \
    --hostname    "$HOSTNAME" \
    --memory      "$MEMORY" \
    --cores       "$CORES" \
    --rootfs      "$STORAGE:$DISK" \
    --net0        "name=eth0,bridge=$BRIDGE,ip=dhcp,ip6=auto" \
    --ostype      debian \
    --unprivileged 1 \
    --features    nesting=1 \
    --start       0
ok "Container $CTID created."

# ── Audio passthrough ─────────────────────────────────────────────────────────
LXC_CONF="/etc/pve/lxc/${CTID}.conf"

case "$AUDIO" in
  alsa)
    if [[ -d /dev/snd ]]; then
        ALSA_MAJOR=$(stat -c '%t' /dev/snd/controlC0 2>/dev/null \
            | xargs printf '%d\n' || echo 116)
        msg "Adding ALSA passthrough (major $ALSA_MAJOR)..."
        cat >> "$LXC_CONF" <<EOF

# ALSA passthrough
lxc.cgroup2.devices.allow: c ${ALSA_MAJOR}:* rwm
lxc.mount.entry: /dev/snd dev/snd none bind,optional,create=dir
EOF
        ok "ALSA passthrough configured."
    else
        warn "/dev/snd not found on host — ALSA passthrough skipped."
        AUDIO=none
    fi
    ;;
  pipewire)
    PIPEWIRE_SOCK="/run/user/${PIPEWIRE_HOST_UID}/pipewire-0"
    if [[ -S "$PIPEWIRE_SOCK" ]]; then
        msg "Adding PipeWire passthrough ($PIPEWIRE_SOCK)..."
        cat >> "$LXC_CONF" <<EOF

# PipeWire passthrough
lxc.mount.entry: ${PIPEWIRE_SOCK} run/user/${PIPEWIRE_HOST_UID}/pipewire-0 none bind,optional,create=file
EOF
        ok "PipeWire passthrough configured."
    else
        warn "PipeWire socket $PIPEWIRE_SOCK not found — passthrough skipped."
        AUDIO=none
    fi
    ;;
  none)
    warn "No audio backend configured. qbzd will run without real audio output."
    ;;
  *)
    die "AUDIO must be alsa, pipewire, or none (got: $AUDIO)"
    ;;
esac

# ── Start container ───────────────────────────────────────────────────────────
msg "Starting container..."
pct start "$CTID"
sleep 3
ok "Container started."

# ── In-container setup ────────────────────────────────────────────────────────
msg "Installing qbzd inside the container..."

pct exec "$CTID" -- bash -euo pipefail <<INNEREOF
export DEBIAN_FRONTEND=noninteractive
ARCH="${ARCH}"
AUDIO="${AUDIO}"

apt-get update -qq
apt-get install -y --no-install-recommends \
    ca-certificates curl dbus libasound2 alsa-utils

if [[ "\$AUDIO" == "pipewire" ]]; then
    apt-get install -y --no-install-recommends \
        pipewire pipewire-pulse wireplumber pipewire-bin pulseaudio-utils
fi

if ! id qbzd &>/dev/null; then
    groupadd -r qbzd
    useradd -r -g qbzd -G audio -m -d /var/lib/qbzd -s /sbin/nologin qbzd
fi

mkdir -p /var/lib/qbzd/config /etc/qbz
chown -R qbzd:qbzd /var/lib/qbzd

BINARY_URL="https://github.com/qbarlas/qbzd/releases/latest/download/qbzd-linux-\${ARCH}"
echo "Downloading from \$BINARY_URL..."
curl -fsSL "\$BINARY_URL" -o /usr/local/bin/qbzd
chmod +x /usr/local/bin/qbzd

cat > /etc/qbz/qbzd.toml <<TOML
[server]
bind = "0.0.0.0"
port = 8182
token = "auto"

[audio]
backend = "\${AUDIO:-alsa}"
device = ""
gapless = true
normalization = false

[cache]
memory_mb = 0
disk_mb = 400
prefetch_count = 2
prefetch_concurrent = 1
cmaf_concurrent_segments = 2

[cache.auto]
enabled = true

[data]
dir = "/var/lib/qbzd"

[qconnect]
enabled = true
device_name = ""

[mdns]
enabled = true

[logging]
level = "info"
journal = true
TOML

cat > /etc/systemd/system/qbzd.service <<UNIT
[Unit]
Description=qbzd — Qobuz Connect receiver
After=network-online.target
Wants=network-online.target

[Service]
User=qbzd
Group=qbzd
ExecStart=/usr/local/bin/qbzd --config /etc/qbz/qbzd.toml --data-dir /var/lib/qbzd
Restart=on-failure
RestartSec=5s
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
UNIT

systemctl daemon-reload
systemctl enable --now qbzd
INNEREOF

ok "qbzd installed and started."

# ── Install the audio selection helper ────────────────────────────────────────
# Written via stdin (tee into the container) — no variable expansion.
msg "Installing qbzd-select-audio..."
pct exec "$CTID" -- tee /usr/local/sbin/qbzd-select-audio > /dev/null << 'SELECTEOF'
#!/usr/bin/env bash
# Select or change the ALSA audio device used by qbzd.
# Usage: qbzd-select-audio
# Can be re-run at any time after a DAC change.

set -euo pipefail

CONFIG="/etc/qbz/qbzd.toml"
GN="\033[32m" RD="\033[31m" BL="\033[36m" CL="\033[0m" BF="\033[1m"
ok()  { echo -e "${GN}✔${CL} $*"; }
die() { echo -e "${RD}✘${CL} $*" >&2; exit 1; }

[[ -f "$CONFIG" ]] || die "Config not found: $CONFIG"

CARDS_RAW=$(aplay -l 2>/dev/null | grep '^card' || true)
[[ -n "$CARDS_RAW" ]] \
    || die "No audio device detected.\nCheck /dev/snd passthrough from the Proxmox host."

echo
echo -e "${BF}  Available audio devices:${CL}"
echo    "  ─────────────────────────"

declare -a HW_IDS
declare -a HW_LABELS
i=0
while IFS= read -r line; do
    CARD_NUM=$(echo "$line" | sed 's/card \([0-9]*\):.*/\1/')
    SHORT=$(echo "$line"    | sed 's/card [0-9]*: \([^ ]*\) .*/\1/')
    DEV_NUM=$(echo "$line"  | sed 's/.*, device \([0-9]*\):.*/\1/')
    LONG=$(echo "$line"     | sed 's/.*\[\(.*\)\], device.*/\1/')
    DEV_LONG=$(echo "$line" | sed 's/.*device [0-9]*: [^ ]* \[\(.*\)\]/\1/')
    HW_IDS[$i]="hw:${SHORT},${DEV_NUM}"
    HW_LABELS[$i]="${LONG} — ${DEV_LONG}  (hw:${CARD_NUM},${DEV_NUM})"
    echo "  [$i] ${HW_LABELS[$i]}"
    ((i++))
done <<< "$CARDS_RAW"
echo

CURRENT=$(grep '^device' "$CONFIG" | sed 's/device *= *"\?\([^"]*\)"\?/\1/' | xargs)
[[ -n "$CURRENT" ]] && echo -e "  Current: ${BF}${CURRENT:-<default>}${CL}\n"

read -r -p "  Device number [0]: " CHOICE
CHOICE="${CHOICE:-0}"
[[ "$CHOICE" =~ ^[0-9]+$ ]] && [[ "$CHOICE" -lt "$i" ]] \
    || die "Invalid choice: $CHOICE"

SELECTED="${HW_IDS[$CHOICE]}"
sed -i "s|^device = .*|device = \"${SELECTED}\"|" "$CONFIG"
ok "Device updated: $SELECTED"

if systemctl is-active --quiet qbzd 2>/dev/null; then
    systemctl restart qbzd
    ok "qbzd restarted."
fi
SELECTEOF

pct exec "$CTID" -- chmod +x /usr/local/sbin/qbzd-select-audio
ok "qbzd-select-audio installed."

# ── Audio device selection ────────────────────────────────────────────────────
if [[ "$AUDIO" == "alsa" ]]; then
    echo
    msg "Select the audio output device..."
    pct exec "$CTID" -- qbzd-select-audio
fi

# ── Container IP ──────────────────────────────────────────────────────────────
sleep 2
CT_IP=$(pct exec "$CTID" -- hostname -I 2>/dev/null | awk '{print $1}') \
    || CT_IP="<container-ip>"

# ── Final instructions ────────────────────────────────────────────────────────
echo
echo -e "${GN}${BF}  Setup complete!${CL}"
echo    "  ─────────────────────────────────────────────────"
echo    "  Container: $CTID ($HOSTNAME)  —  IP: $CT_IP"
echo
echo    "  Authenticate with Qobuz:"
echo -e "    ${BF}pct exec $CTID -- qbzd login${CL}"
echo    "    (if the OAuth callback is unreachable from the browser:)"
echo -e "    ${BF}pct exec $CTID -- qbzd login --callback-host http://${CT_IP}:8182${CL}"
echo
echo    "  HTTP API:"
echo -e "    ${BF}http://${CT_IP}:8182/api/status${CL}"
echo
echo    "  To change the DAC later:"
echo -e "    ${BF}pct exec $CTID -- qbzd-select-audio${CL}"
echo
if [[ "$AUDIO" == "none" ]]; then
    echo -e "  ${YW}⚠ No audio backend configured.${CL}"
    echo    "    Edit /etc/qbz/qbzd.toml, then: pct exec $CTID -- systemctl restart qbzd"
    echo
fi
