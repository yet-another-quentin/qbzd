# Contributing to qbzd

Contributions are welcome. This project is a headless Rust daemon — no frontend, no build system beyond `cargo`.

> qbzd is a fork of [vicrodh/qbz](https://github.com/vicrodh/qbz). The core
> architecture comes from that project. When working on audio backends, the
> Qobuz API client, or the QConnect protocol, the original codebase is a
> useful reference.

## Ground rules

- Keep PRs focused. One concern per PR.
- Write clear commit messages (type: short description).
- No emojis in code, comments, or commit messages.
- Do not modify Qobuz branding or legal disclaimers without discussion.

## Development setup

**Prerequisites:** Rust stable, `pkg-config`, `libasound2-dev`, `libdbus-1-dev`

```bash
git clone https://github.com/qbarlas/qbzd.git
cd qbzd/crates
cargo build -p qbzd
```

Run the daemon:

```bash
cargo run -p qbzd -- --log-level debug
```

Then authenticate in another terminal:

```bash
cargo run -p qbzd -- login
```

## Project structure

```
crates/
  qbzd/                   Daemon binary — HTTP API, CLI, auth, QConnect bridge
  qbz-core/               Orchestrator (player + audio + Qobuz API)
  qbz-player/             Playback engine, queue, streaming
  qbz-audio/              Audio backends (PipeWire, ALSA, PulseAudio)
  qbz-qobuz/              Qobuz API client and OAuth
  qbz-models/             Shared domain types
  qbz-cache/              Audio cache (memory + disk)
  qbz-cmaf/               CMAF/MP4 fragment parser (used by qbz-qobuz)
  qconnect-protocol/      Qobuz Connect protobuf wire format
  qconnect-core/          Queue and renderer state machines
  qconnect-app/           QConnect application logic and orchestration
  qconnect-transport-ws/  WebSocket transport with qcloud framing
```

## Checks before submitting

```bash
cargo check -p qbzd
cargo clippy -p qbzd
cargo test -p qbzd
```

## Commit message format

```
<type>: <short description>

<optional body>
```

Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`

## What not to include

- Changes to the Qobuz Connect protocol (reverse-engineered, changes break things).
- New audio backends without testing on real hardware.
- Large refactors mixed with feature work.
