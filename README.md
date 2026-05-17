# qbzd

**Qobuz Connect receiver for Linux — headless, containerized.**

qbzd is a lightweight daemon that makes any Linux machine (or container) appear as a Qobuz Connect device. Once running, it shows up in the Qobuz app (Android, iOS, macOS, web) and accepts playback commands from there.

Fork of [vicrodh/qbz](https://github.com/vicrodh/qbz) (MIT), stripped down to the headless daemon and its dependencies.

> **Legal:** This project uses the Qobuz API but is not affiliated with, endorsed by, or certified by Qobuz. Qobuz is a registered trademark of Qobuz SAS.

---

## Quick start (Docker)

```bash
docker run -d \
  --name qbzd \
  -p 8182:8182 \
  -v qbzd-data:/var/lib/qbzd \
  ghcr.io/qbarlas/qbzd:latest
```

Then authenticate:

```bash
docker exec -it qbzd qbzd login
```

This opens the Qobuz OAuth URL. Open it in any browser on the same network. After login the device appears in the Qobuz app within a few seconds.

If the browser redirect cannot reach the container (e.g. Docker Desktop on macOS), pass the public address explicitly:

```bash
docker exec -it qbzd qbzd login --callback-host http://<your-host-ip>:8182
```

Alternatively, authenticate with a direct token (obtained from a Qobuz session cookie or a trusted tool):

```bash
docker exec -it qbzd qbzd login --token <user_auth_token>
```

---

## Configuration

Mount a `qbzd.toml` into `/etc/qbz/qbzd.toml` to override defaults:

```toml
[server]
port = 8182

[audio]
backend = "pipewire"   # pipewire | alsa | pulse

[qconnect]
enabled     = true
device_name = "Living Room"   # name shown in Qobuz app

```

A default config is baked into the image at `/etc/qbz/qbzd.toml.default`.

### Proxmox LXC

The recommended way to run qbzd on Proxmox is as an unprivileged LXC container with ALSA passthrough. A one-liner installer handles everything:

```bash
# On the Proxmox host, as root:
bash <(curl -fsSL https://raw.githubusercontent.com/qbarlas/qbzd/main/install/proxmox-lxc.sh)
```

Environment overrides (all optional):

| Variable | Default | Description |
|----------|---------|-------------|
| `CTID` | next available | Container ID |
| `HOSTNAME` | `qbzd` | Container hostname |
| `MEMORY` | `256` | RAM in MB |
| `DISK` | `2` | Disk in GB |
| `STORAGE` | `local-lvm` | Proxmox storage pool |
| `AUDIO` | `alsa` | Audio backend: `alsa`, `pipewire`, `none` |
| `CHANNEL` | `latest` | Release channel: `latest` or any tag (e.g. `v0.1.0`) |

After installation:

```bash
# Authenticate with Qobuz
pct exec <CTID> -- qbzd login

# Select the audio output device (DAC)
pct exec <CTID> -- qbzd-select-audio

# Update the binary
pct exec <CTID> -- qbzd-update            # latest release
pct exec <CTID> -- qbzd-update v0.1.0     # specific tag
```

**Audio device permissions** — unprivileged containers cannot access `/dev/snd` by default. If qbzd fails to start with an audio permission error, run on the host:

```bash
echo 'SUBSYSTEM=="sound", MODE="0666"' > /etc/udev/rules.d/99-lxc-audio.rules
udevadm control --reload-rules && udevadm trigger
pct restart <CTID>
```

**Auto start/stop with DAC** — optionally start qbzd automatically when the DAC is plugged in and stop it on unplug:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/qbarlas/qbzd/main/install/proxmox-dac-watch.sh)
```

### Docker / audio passthrough

For real audio output, pass through the ALSA device or PipeWire socket from the host:

```yaml
# docker-compose.yml
devices:
  - /dev/snd:/dev/snd
environment:
  - ALSA_CARD=0
```

Or for PipeWire:

```yaml
volumes:
  - /run/user/1000/pipewire-0:/run/user/1000/pipewire-0
```

---

## HTTP API

The daemon exposes a local REST API on port 8182.

| Group | Endpoints |
|-------|-----------|
| Auth | `POST /api/auth/oauth/start` · `GET /api/auth/oauth/callback` · `GET /api/auth/oauth/status` · `POST /api/auth/token` |
| System | `GET /api/ping` · `GET /api/status` · `GET /api/info` · `GET /api/events` (SSE) · `GET /api/system/resources` · `DELETE /api/cache` |
| Playback | `GET /api/playback` · play · pause · stop · next · previous · seek · volume |
| Queue | `GET /api/queue` · set · add · add-next · play-index · remove · move · clear · shuffle · repeat |
| Audio | `GET/PATCH /api/audio/settings` · backends · devices · hardware-status |

Full spec: `docs/openapi.yaml`

---

## Building from source

**Prerequisites:** Rust stable, `pkg-config`, `libasound2-dev`, `libdbus-1-dev`

```bash
git clone https://github.com/qbarlas/qbzd.git
cd qbzd/crates
cargo build --release -p qbzd
```

**Docker:**

```bash
docker build -t qbzd:latest .
```

---

## Architecture

```
crates/
  qbzd/                   Headless daemon — HTTP API, auth, QConnect bridge
  qbz-core/               Orchestrator (player + audio + API client)
  qbz-player/             Playback engine, queue, streaming
  qbz-audio/              Audio backends (PipeWire, ALSA, PulseAudio)
  qbz-qobuz/              Qobuz API client and OAuth
  qbz-models/             Shared domain types
  qbz-cache/              Audio cache (memory + disk)
  qbz-cmaf/               CMAF/MP4 fragment parser (used by qbz-qobuz)
  qconnect-protocol/      Qobuz Connect protobuf wire format
  qconnect-core/          Queue and renderer state machines
  qconnect-app/           QConnect application logic
  qconnect-transport-ws/  WebSocket transport with qcloud framing
```

---

## Credits

qbzd is a fork of [vicrodh/qbz](https://github.com/vicrodh/qbz) by
[blitzkriegfc](https://github.com/vicrodh), the original author of the Qobuz
Connect receiver. The core architecture, audio backends, Qobuz API client, and
QConnect protocol implementation all originate from that project.

This fork strips the desktop UI and refocuses the project as a headless daemon.
The upstream project is MIT-licensed and its copyright is preserved in full in
[LICENSE](LICENSE).

## License

MIT — see [LICENSE](LICENSE).
