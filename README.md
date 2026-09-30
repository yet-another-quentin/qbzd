# qbzd on Proxmox

**Deployment tooling to run [qbzd](https://github.com/vicrodh/qbz) — the headless Qobuz Connect daemon — in an unprivileged Proxmox LXC, feeding a USB DAC.**

This repository contains **no application code**. It installs the official `qbzd`
binary published by the upstream project and wires it into a container: ALSA
passthrough from the host, a systemd unit, an update helper, and optional
automatic start/stop when the DAC is plugged in.

Once running, the container appears in the official Qobuz app (Android, iOS,
macOS, web) as a castable device.

> **Legal:** This project uses the Qobuz API but is not affiliated with, endorsed by, or certified by Qobuz. Qobuz is a registered trademark of Qobuz SAS.

---

## Install

On the Proxmox host, as root:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/yet-another-quentin/qbzd/main/install/proxmox-lxc.sh)
```

The installer creates an unprivileged Debian 12 container, passes `/dev/snd`
through, downloads the latest upstream `qbzd` release and registers a systemd
unit.

Then configure it — this step is interactive, so it needs a real terminal:

```bash
pct enter <CTID>
su - qbzd -c 'qbzd setup'      # Qobuz login, audio device, Connect device name
systemctl enable --now qbzd
```

`qbzd setup` is upstream's six-screen configurator. It must run as the `qbzd`
user, because it writes the same stores the service reads.

### Options

| Variable | Default | Description |
|----------|---------|-------------|
| `CTID` | next available | Container ID |
| `CT_HOSTNAME` | `qbzd` | Container hostname (not `HOSTNAME` — bash already owns that name) |
| `MEMORY` | `512` | RAM in MB |
| `DISK` | `2` | Disk in GB |
| `CORES` | `1` | CPU cores |
| `STORAGE` | `local-lvm` | Proxmox storage pool |
| `BRIDGE` | `vmbr0` | Network bridge |
| `AUDIO` | `alsa` | `alsa`, `pipewire` or `none` |
| `CHANNEL` | `latest` | Upstream release: `latest` or a tag (e.g. `v2.0.2`) |

```bash
CTID=200 MEMORY=1024 AUDIO=alsa bash <(curl -fsSL .../proxmox-lxc.sh)
```

**Use ALSA.** On a USB DAC it is bit-perfect and runs no sound server inside the
container. PipeWire adds three daemons (~60–90 MB) for no benefit here.

---

## Day-to-day

```bash
# Change the DAC, the device name, or any other setting
pct enter <CTID>
su - qbzd -c 'qbzd setup'

# Update to the latest upstream release
pct exec <CTID> -- qbzd-update
pct exec <CTID> -- qbzd-update v2.0.2   # or a specific tag

# Service
pct exec <CTID> -- systemctl status qbzd
pct exec <CTID> -- journalctl -u qbzd -f
```

The HTTP API is on port 8182: `http://<container-ip>:8182/api/status`.

Upstream ships a full CLI inside the same binary (`qbzd play`, `qbzd queue`,
`qbzd search`, …) — see the [upstream wiki](https://github.com/vicrodh/qbz/wiki/Headless-Daemon).

---

## Audio device permissions

Unprivileged containers cannot access `/dev/snd` by default. If qbzd fails to
start with an audio permission error, run on the **host**:

```bash
echo 'SUBSYSTEM=="sound", MODE="0666"' > /etc/udev/rules.d/99-lxc-audio.rules
udevadm control --reload-rules && udevadm trigger
pct restart <CTID>
```

---

## Auto start/stop with the DAC

Optional: start qbzd when the DAC is plugged in, stop it when unplugged.

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/yet-another-quentin/qbzd/main/install/proxmox-dac-watch.sh)
```

This writes a udev rule on the host that drives
`pct exec <CTID> -- systemctl start|stop qbzd`. Only physical USB disconnect is
detected — a DAC that stays enumerated when powered off will not trigger it.

---

## Why a system unit

Upstream ships a **user** unit that requires `loginctl enable-linger`. This
installer registers a **system** unit instead:

- no linger to enable, and no session to keep alive;
- `pct exec <CTID> -- systemctl start qbzd` works from the host, which is what
  the DAC watch rule needs — driving a user unit from outside the container is
  considerably more fragile.

The binary is the same, and boot-order resilience lives inside it (missing
network or DAC at start means retry with backoff, never a crash-exit).

---

## History

This repository used to carry a fork of [vicrodh/qbz](https://github.com/vicrodh/qbz)
stripped down to a headless daemon. Upstream now ships its own `qbzd` — more
complete (MPRIS, scrobbling, a CLI and a setup TUI) and actively maintained —
so the fork was retired in favour of these deployment scripts. The Git history
still holds it if you need to look back.

## License

MIT — see [LICENSE](LICENSE).
