# Contributing

This repository is deployment tooling only: two Bash scripts that install and
wire up the upstream [qbzd](https://github.com/vicrodh/qbz) daemon inside a
Proxmox LXC. There is no application code here.

## Where a change belongs

- **Playback, audio backends, the HTTP API, Qobuz Connect** — these live
  upstream in [vicrodh/qbz](https://github.com/vicrodh/qbz). Report and fix
  them there.
- **Container creation, ALSA passthrough, the systemd unit, DAC hotplug** —
  here.

If you are unsure: if the fix would still be needed when running qbzd on bare
metal, it belongs upstream.

## Ground rules

- Keep PRs focused. One concern per PR.
- Commit messages: `<type>: <short description>`, type in
  `feat|fix|chore|docs|refactor`.
- No emojis in code, comments, or commit messages.
- Do not modify Qobuz branding or legal disclaimers without discussion.

## Testing changes

These scripts run as root on a Proxmox host and create containers, so they
cannot be meaningfully unit-tested. Before submitting:

```bash
bash -n install/proxmox-lxc.sh        # syntax
shellcheck install/*.sh               # if available
```

Then run the installer end-to-end on a real Proxmox host, against a throwaway
CTID, and confirm:

1. the container is created and `/dev/snd` is visible inside it;
2. `qbzd setup` completes and the device shows up in the Qobuz app;
3. `systemctl enable --now qbzd` survives a `pct restart`;
4. `qbzd-update` replaces the binary and restarts the service.

State in the PR which Proxmox version and which DAC you tested against — the
audio passthrough behaviour varies with both.

## Things to be careful about

- The installer builds a script and pipes it into the container through a
  heredoc. Variables meant to be resolved **inside** the container must be
  escaped (`\$FOO`); variables resolved on the host must not. Getting this
  wrong silently produces empty values.
- Upstream tarballs embed the version in their filename
  (`qbzd-2.0.2-linux-amd64.tar.gz`), so `latest` has to be resolved to a
  concrete tag before the URL can be built.
- Upstream names architectures `amd64` and `aarch64`; Debian calls the latter
  `arm64`. The mapping is deliberate, not a typo.
