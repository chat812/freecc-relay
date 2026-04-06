# Free CC Relay

Self-hosted WebSocket relay server for remote Claude Code sessions. Written in Rust -- single static binary, zero dependencies.

## Features

- **WebSocket relay** between CLI clients and web browsers
- **Session management** with persistence across restarts
- **Admin dashboard** with live session monitoring, kill/cleanup controls
- **Client pairing** -- new clients request access, admins approve from the dashboard
- **Key-based auth** with configurable per-key session limits
- **TLS support** via `--tls-cert` / `--tls-key`
- **Reverse proxy ready** -- respects `X-Forwarded-Proto` and `Host` headers

## Quick Start

```bash
# Clone and install (builds from source, sets up systemd service)
git clone https://github.com/chat812/freecc-relay.git
cd freecc-relay
sudo ./install.sh
```

The installer will:
1. Install Rust if not present
2. Build the release binary
3. Install to `/opt/freecc-relay/`
4. Generate an initial client key
5. Create and start a systemd service

## Manual Build

```bash
# Standard build
cargo build --release

# Static build (no dependencies, portable)
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

## Usage

```bash
# Run directly
./freecc-relay --port 8081 --host 0.0.0.0

# Generate a new client key
./freecc-relay --generate-key "my-laptop"

# With TLS
./freecc-relay --tls-cert cert.pem --tls-key key.pem
```

## API

### Public

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/` | Server status page |
| `GET` | `/api/health` | Health check JSON |
| `POST` | `/api/pair` | Request client pairing (no auth) |
| `GET` | `/api/pair/:id` | Check pairing status |

### Client (requires `X-Client-Key` header)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/sessions` | Create a new session |
| `GET` | `/api/sessions` | List your sessions |
| `DELETE` | `/api/sessions/:id` | Delete a session |

### Admin (requires `X-Admin-Token` header)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/admin` | Admin dashboard |
| `GET` | `/api/admin/sessions` | List all sessions |
| `POST` | `/api/admin/sessions/kill` | Kill specific sessions |
| `POST` | `/api/admin/sessions/kill-all` | Kill all sessions |
| `POST` | `/api/admin/sessions/cleanup` | Remove stale sessions |
| `GET` | `/api/admin/pairings` | List pairing requests |
| `POST` | `/api/admin/pairings/:id/approve` | Approve a pairing |
| `POST` | `/api/admin/pairings/:id/reject` | Reject a pairing |

### WebSocket

| Path | Description |
|------|-------------|
| `/ws/cli/:id?key=` | CLI client connection |
| `/ws/web/:id?token=` | Web browser connection |
| `/s/:id?token=` | Web UI page |

## Configuration

Config is stored in `config/server.json` (next to the binary):

```json
{
  "auth": {
    "mode": "key",
    "sessionExpiry": "24h",
    "maxSessionsPerKey": 10,
    "webAccessPolicy": "token"
  },
  "clients": [
    { "name": "default", "key": "ck_..." }
  ]
}
```

## Service Management

```bash
sudo systemctl status freecc-relay
sudo systemctl restart freecc-relay
sudo journalctl -u freecc-relay -f
```

The admin password is regenerated on each start -- check the logs:

```bash
sudo journalctl -u freecc-relay | grep 'Admin password'
```

## License

MIT
