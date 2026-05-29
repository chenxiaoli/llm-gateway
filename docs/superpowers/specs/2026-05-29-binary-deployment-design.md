# Binary Deployment Design

## Context

Currently LLM Gateway supports Docker-based deployment via `docker-compose.yml` but lacks a bare-metal / VM deployment path. Users want to install directly on Linux servers using pre-built release binaries without Docker.

## Architecture

### Directory Structure

```
/opt/llm-gateway/          # Binaries and embedded assets
├── llm-gateway           # Main API gateway
├── llm-gateway-usage-worker  # Usage recording worker
└── llm-gateway-audit-worker  # Audit logging worker

/etc/llm-gateway/
└── config.toml            # Runtime configuration (user-managed)

/var/log/llm-gateway/      # Log files (symlinked by systemd)
```

### Three Separate Systemd Units

All services share the same `config.toml` at `/etc/llm-gateway/config.toml` and log to the journal via systemd.

| Unit | Binary | Description |
|------|--------|-------------|
| `llm-gateway.service` | `llm-gateway` | API gateway on port 8080 |
| `llm-gateway-usage-worker.service` | `llm-gateway-usage-worker` | NATS consumer → writes usage records |
| `llm-gateway-audit-worker.service` | `llm-gateway-audit-worker` | NATS consumer → writes audit logs |

### Installation Flow

1. **Detect** — OS (Linux only), architecture (x86_64, aarch64)
2. **Download** — Fetch `ghcr.io/chenxiaoli/llm-gateway:${VERSION}-linux-x86_64` from GitHub Releases
3. **Verify** — SHA256 checksum via GitHub Release API
4. **Install binaries** — Extract to `/opt/llm-gateway/`, set executable
5. **Create user** — `llm-gateway` system user (no login shell)
6. **Install config** — Create `/etc/llm-gateway/` and install default `config.toml`
7. **Install systemd units** — Three units, each with `Restart=always`, `RestartSec=5s`, journal logging
8. **Start services** — `systemctl enable --now` all three units

### Uninstall Flow

1. Stop all three services
2. Disable all three units
3. Remove systemd unit files
4. Remove binaries from `/opt/llm-gateway/`
5. Remove config (optional: prompt)
6. Remove log directory (if empty)
7. Remove `llm-gateway` user (optional: prompt)

## CI Changes

The CI build-release job needs to produce a tarball instead of a raw binary, so `install.sh` can download one artifact per platform:

```
llm-gateway-${VERSION}-linux-x86_64.tar.gz   # contains all 3 binaries
```

The tarball should be created by zipping all three built binaries.

## Design Decisions

- **Location `/opt/llm-gateway/`**: Standard for third-party software on Linux
- **Config at `/etc/llm-gateway/`**: Config belongs in `/etc`, binaries in `/opt`
- **No embedded config defaults**: User must provide `config.toml` — the binary will refuse to start without one
- **System user, not root**: All services run as `llm-gateway` user
- **Journal logging only**: No file-based log rotation — systemd `journald` handles it
- **Architecture detection**: `uname -m` mapped to CI artifact names (`x86_64` → `linux-x86_64`, `aarch64` → `linux-arm64`)
- **Version from CLI arg or env**: `install.sh --version 1.5.0` or `VERSION=1.5.0 ./install.sh`, defaults to `latest`

## Files to Create

| File | Purpose |
|------|---------|
| `deploy/bin/install.sh` | Main installation script |
| `deploy/bin/uninstall.sh` | Uninstall (also callable as `install.sh --uninstall`) |
| `deploy/systemd/llm-gateway.service` | Systemd unit for gateway |
| `deploy/systemd/llm-gateway-usage-worker.service` | Systemd unit for usage worker |
| `deploy/systemd/llm-gateway-audit-worker.service` | Systemd unit for audit worker |
| `deploy/systemd/llm-gateway.conf` | Log rate-limit config |
| `.github/workflows/ci.yml` | Update build-release job to produce tarball |