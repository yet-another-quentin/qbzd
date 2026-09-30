#!/usr/bin/env bash
# qbzd — Proxmox LXC installer
#
# Creates an unprivileged Debian 12 container running qbzd, the headless
# Qobuz Connect daemon from https://github.com/vicrodh/qbz, with the host's
# USB DAC passed through via ALSA.
#
# This installer does not build anything: it downloads the official qbzd
# release tarball published by the upstream project.
#
# Usage (on the Proxmox host, as root):
#   bash <(curl -fsSL https://raw.githubusercontent.com/yet-another-quentin/qbzd/main/install/proxmox-lxc.sh)
#
# Environment overrides:
#   CTID=200 MEMORY=1024 DISK=4 STORAGE=local-lvm AUDIO=alsa CHANNEL=v2.0.2 bash <(...)

set -euo pipefail

# ── Colors ────────────────────────────────────────────────────────────────────
GN="\033[32m" RD="\033[31m" YW="\033[33m" BL="\033[36m" CL="\033[0m" BF="\033[1m"
msg()  { echo -e "${BL}▶${CL} $*"; }
ok()   { echo -e "${GN}✔${CL} $*"; }
warn() { echo -e "${YW}⚠${CL} $*"; }
die()  { echo -e "${RD}✘${CL} $*" >&2; exit 1; }

# ── Prerequisites ─────────────────────────────────────────────────────────────
[[ $EUID -eq 0 ]]           || die "This script must be run as root on the Proxmox host."
command -v pct >/dev/null   || die "pct not found — this script must run on a Proxmox host."
command -v pvesh >/dev/null || die "pvesh not found."

# ── Parameters ────────────────────────────────────────────────────────────────
ARCH=$(dpkg --print-architecture 2>/dev/null || uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')
# Upstream names its tarballs amd64 / aarch64, Debian calls the latter arm64.
case "$ARCH" in
  amd64) ASSET_ARCH="amd64"   ;;
  arm64) ASSET_ARCH="aarch64" ;;
  *)     die "Unsupported architecture: $ARCH (upstream ships amd64 and aarch64 only)." ;;
esac

CTID="${CTID:-$(pvesh get /cluster/nextid 2>/dev/null || echo 200)}"
# CT_HOSTNAME, not HOSTNAME: bash sets HOSTNAME itself, to the name of the
# machine running the script. `${HOSTNAME:-qbzd}` therefore never falls back to
# qbzd — it silently names the container after the Proxmox host.
CT_HOSTNAME="${CT_HOSTNAME:-qbzd}"
MEMORY="${MEMORY:-512}"
DISK="${DISK:-2}"
CORES="${CORES:-1}"
BRIDGE="${BRIDGE:-vmbr0}"
STORAGE="${STORAGE:-local-lvm}"
# Audio backend: alsa | pipewire | none
# alsa is the recommended path for a USB DAC: bit-perfect, and no sound
# server daemons running inside the container.
AUDIO="${AUDIO:-alsa}"
# Release channel: latest | any upstream tag (e.g. v2.0.2)
CHANNEL="${CHANNEL:-latest}"
# For PipeWire: UID of the host user owning the socket
PIPEWIRE_HOST_UID="${PIPEWIRE_HOST_UID:-1000}"

UPSTREAM_REPO="vicrodh/qbz"

# ── Resolve the release to install ────────────────────────────────────────────
# The tarball name embeds the version (qbzd-2.0.2-linux-amd64.tar.gz), so even
# "latest" has to be resolved to a concrete tag before building the URL.
msg "Resolving the qbzd release to install..."
if [[ "$CHANNEL" == "latest" ]]; then
    TAG=$(curl -fsSL "https://api.github.com/repos/${UPSTREAM_REPO}/releases/latest" \
        | sed -n 's/.*"tag_name" *: *"\([^"]*\)".*/\1/p' | head -1)
    [[ -n "$TAG" ]] || die "Could not resolve the latest release of ${UPSTREAM_REPO}."
else
    TAG="$CHANNEL"
fi
VERSION="${TAG#v}"
TARBALL="qbzd-${VERSION}-linux-${ASSET_ARCH}.tar.gz"
TARBALL_URL="https://github.com/${UPSTREAM_REPO}/releases/download/${TAG}/${TARBALL}"

curl -fsIL -o /dev/null "$TARBALL_URL" \
    || die "Release asset not found:\n    ${TARBALL_URL}\n  Check the tag with: gh release list --repo ${UPSTREAM_REPO}"
ok "Release ${TAG} (${ASSET_ARCH})."

echo
echo -e "${BF}  qbzd — Proxmox LXC installer${CL}"
echo    "  ─────────────────────────────"
echo    "  Container ID  : $CTID"
echo    "  Hostname      : $CT_HOSTNAME"
echo    "  RAM / Disk    : ${MEMORY} MB / ${DISK} GB"
echo    "  Storage       : $STORAGE"
echo    "  Arch          : $ARCH"
echo    "  Audio         : $AUDIO"
echo    "  qbzd version  : $TAG (from ${UPSTREAM_REPO})"
echo
read -r -p "  Continue? [Y/n] " _confirm
[[ "${_confirm:-Y}" =~ ^[Yy]$ ]] || { echo "Aborted."; exit 0; }
echo

# ── Debian 12 template ────────────────────────────────────────────────────────
# Upstream builds qbzd against glibc 2.35, so bookworm (2.36) runs it as-is.
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
    --hostname    "$CT_HOSTNAME" \
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
        # /proc/devices lists the major in decimal ("116 alsa"). Do not read it
        # from `stat -c '%t'`: that prints hex (74), which then has to be
        # converted — getting it wrong writes a cgroup rule for major 74 and
        # every open of /dev/snd/* inside the container fails with EPERM.
        ALSA_MAJOR=$(awk '$2 == "alsa" { print $1 }' /proc/devices | head -1)
        [[ -n "$ALSA_MAJOR" ]] || ALSA_MAJOR=116
        msg "Adding ALSA passthrough (major $ALSA_MAJOR)..."
        cat >> "$LXC_CONF" <<EOF

# ALSA passthrough
lxc.cgroup2.devices.allow: c ${ALSA_MAJOR}:* rwm
lxc.mount.entry: /dev/snd dev/snd none bind,optional,create=dir
EOF
        ok "ALSA passthrough configured."
        echo
        warn "Unprivileged containers cannot access /dev/snd by default."
        warn "If qbzd fails to start with an audio permission error, run these"
        warn "commands on the Proxmox host then restart the container:"
        echo -e "    ${BF}echo 'SUBSYSTEM==\"sound\", MODE=\"0666\"' > /etc/udev/rules.d/99-lxc-audio.rules${CL}"
        echo -e "    ${BF}udevadm control --reload-rules && udevadm trigger${CL}"
        echo -e "    ${BF}pct restart $CTID${CL}"
        echo
    else
        warn "/dev/snd not found on host — ALSA passthrough skipped."
        AUDIO=none
    fi
    ;;
  pipewire)
    warn "PipeWire runs three extra daemons inside the container for no benefit"
    warn "on a USB DAC. ALSA (AUDIO=alsa) is lighter and bit-perfect."
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
AUDIO="${AUDIO}"
TARBALL_URL="${TARBALL_URL}"
TARBALL="${TARBALL}"
VERSION="${VERSION}"
ASSET_ARCH="${ASSET_ARCH}"

apt-get update -qq
apt-get install -y --no-install-recommends \
    ca-certificates curl dbus libasound2 alsa-utils

if [[ "\$AUDIO" == "pipewire" ]]; then
    apt-get install -y --no-install-recommends \
        pipewire pipewire-pulse wireplumber pipewire-bin pulseaudio-utils
fi

# Dedicated service account. It needs a real shell because \`qbzd setup\` is an
# interactive TUI that must run as this user — it writes the very stores the
# service reads, so running it as root would configure the wrong account.
if ! id qbzd &>/dev/null; then
    groupadd -r qbzd
    useradd -r -g qbzd -G audio -m -d /var/lib/qbzd -s /bin/bash qbzd
fi
mkdir -p /var/lib/qbzd
chown -R qbzd:qbzd /var/lib/qbzd

echo "Downloading \$TARBALL_URL..."
TMP=\$(mktemp -d)
curl -fsSL "\$TARBALL_URL" -o "\$TMP/\$TARBALL"
tar xzf "\$TMP/\$TARBALL" -C "\$TMP"
SRC="\$TMP/qbzd-\${VERSION}-linux-\${ASSET_ARCH}"

install -Dm755 "\$SRC/qbzd" /usr/bin/qbzd
if [[ -f "\$SRC/completions/qbzd.bash" ]]; then
    install -Dm644 "\$SRC/completions/qbzd.bash" \
        /usr/share/bash-completion/completions/qbzd
fi
rm -rf "\$TMP"

# System unit rather than the user unit upstream ships: no linger needed, it
# survives logout by construction, and \`pct exec <CTID> -- systemctl start qbzd\`
# works from the Proxmox host — which is what proxmox-dac-watch.sh drives.
cat > /etc/systemd/system/qbzd.service <<UNIT
[Unit]
Description=qbzd — Qobuz Connect receiver
Documentation=https://github.com/vicrodh/qbz/wiki/Headless-Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=qbzd
Group=qbzd
ExecStart=/usr/bin/qbzd run
Restart=on-failure
RestartSec=10
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
UNIT

systemctl daemon-reload

# Deliberately not enabled or started yet: qbzd has no credentials and no
# audio device until \`qbzd setup\` has run, and an unconfigured unit would
# just restart-loop.
INNEREOF

ok "qbzd ${TAG} installed (service registered, not started yet)."

# ── Install the update helper ─────────────────────────────────────────────────
msg "Installing qbzd-update..."
pct exec "$CTID" -- tee /usr/local/sbin/qbzd-update > /dev/null << 'UPDATEEOF'
#!/usr/bin/env bash
# Update the qbzd binary from the upstream release tarballs.
# Usage: qbzd-update [latest|<tag>]   (default: latest)

set -euo pipefail

CHANNEL="${1:-latest}"
UPSTREAM_REPO="vicrodh/qbz"

GN="\033[32m" RD="\033[31m" BL="\033[36m" CL="\033[0m"
msg() { echo -e "${BL}▶${CL} $*"; }
ok()  { echo -e "${GN}✔${CL} $*"; }
die() { echo -e "${RD}✘${CL} $*" >&2; exit 1; }

ARCH=$(dpkg --print-architecture 2>/dev/null || uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')
case "$ARCH" in
  amd64) ASSET_ARCH="amd64"   ;;
  arm64) ASSET_ARCH="aarch64" ;;
  *)     die "Unsupported architecture: $ARCH" ;;
esac

if [[ "$CHANNEL" == "latest" ]]; then
    TAG=$(curl -fsSL "https://api.github.com/repos/${UPSTREAM_REPO}/releases/latest" \
        | sed -n 's/.*"tag_name" *: *"\([^"]*\)".*/\1/p' | head -1)
    [[ -n "$TAG" ]] || die "Could not resolve the latest release."
else
    TAG="$CHANNEL"
fi
VERSION="${TAG#v}"
TARBALL="qbzd-${VERSION}-linux-${ASSET_ARCH}.tar.gz"
URL="https://github.com/${UPSTREAM_REPO}/releases/download/${TAG}/${TARBALL}"

CURRENT=$(qbzd --version 2>/dev/null || echo "unknown")
msg "Installed: ${CURRENT}"
msg "Downloading ${TAG} (${ASSET_ARCH})..."

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
curl -fsSL "$URL" -o "$TMP/$TARBALL" || die "Download failed: $URL"
tar xzf "$TMP/$TARBALL" -C "$TMP"
SRC="$TMP/qbzd-${VERSION}-linux-${ASSET_ARCH}"
[[ -x "$SRC/qbzd" ]] || die "No qbzd binary inside $TARBALL"

WAS_ACTIVE=no
if systemctl is-active --quiet qbzd 2>/dev/null; then
    WAS_ACTIVE=yes
    systemctl stop qbzd
fi

install -Dm755 "$SRC/qbzd" /usr/bin/qbzd
if [[ -f "$SRC/completions/qbzd.bash" ]]; then
    install -Dm644 "$SRC/completions/qbzd.bash" \
        /usr/share/bash-completion/completions/qbzd
fi
ok "Updated to $(qbzd --version 2>/dev/null || echo "$TAG")."

if [[ "$WAS_ACTIVE" == "yes" ]]; then
    systemctl start qbzd
    ok "qbzd restarted."
fi
UPDATEEOF

pct exec "$CTID" -- chmod +x /usr/local/sbin/qbzd-update
pct exec "$CTID" -- ln -sf /usr/local/sbin/qbzd-update /usr/bin/qbzd-update
ok "qbzd-update installed."

# ── Container IP ──────────────────────────────────────────────────────────────
sleep 2
CT_IP=$(pct exec "$CTID" -- hostname -I 2>/dev/null | awk '{print $1}') \
    || CT_IP="<container-ip>"

# ── Final instructions ────────────────────────────────────────────────────────
echo
echo -e "${GN}${BF}  Container ready — one manual step left.${CL}"
echo    "  ─────────────────────────────────────────────────"
echo    "  Container: $CTID ($CT_HOSTNAME)  —  IP: $CT_IP"
echo
echo    "  1. Run the configurator. It is an interactive TUI (Qobuz login,"
echo    "     audio device, Connect device name), so it needs a real terminal:"
echo -e "       ${BF}pct enter $CTID${CL}"
echo -e "       ${BF}su - qbzd -c 'qbzd setup'${CL}"
echo
echo    "  2. Then enable and start the daemon:"
echo -e "       ${BF}systemctl enable --now qbzd${CL}"
echo -e "       ${BF}systemctl status qbzd${CL}"
echo
echo    "  The device then appears in the official Qobuz app."
echo
echo    "  HTTP API:"
echo -e "    ${BF}http://${CT_IP}:8182/api/status${CL}"
echo
echo    "  To change the DAC or any setting later, re-run the configurator:"
echo -e "    ${BF}pct enter $CTID${CL} then ${BF}su - qbzd -c 'qbzd setup'${CL}"
echo
echo    "  To update qbzd:"
echo -e "    ${BF}pct exec $CTID -- qbzd-update${CL}           # latest upstream release"
echo -e "    ${BF}pct exec $CTID -- qbzd-update v2.0.2${CL}    # specific tag"
echo
echo    "  Auto start/stop with DAC (optional):"
echo -e "    ${BF}bash <(curl -fsSL https://raw.githubusercontent.com/yet-another-quentin/qbzd/main/install/proxmox-dac-watch.sh)${CL}"
echo
if [[ "$AUDIO" == "none" ]]; then
    echo -e "  ${YW}⚠ No audio backend configured — qbzd will run without real output.${CL}"
    echo
fi
