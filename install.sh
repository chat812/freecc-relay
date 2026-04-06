#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="/opt/freecc-relay"
SERVICE_NAME="freecc-relay"
DEFAULT_PORT=8081

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}[*]${NC} $*"; }
ok()    { echo -e "${GREEN}[+]${NC} $*"; }
warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
err()   { echo -e "${RED}[-]${NC} $*"; exit 1; }

# Must be root
[[ $EUID -eq 0 ]] || err "This script must be run as root (use sudo)"

echo
echo -e "${CYAN}  ╔══════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}  ║     Free CC Relay — Installer                 ║${NC}"
echo -e "${CYAN}  ╚══════════════════════════════════════════════╝${NC}"
echo

# ── Dependencies ──────────────────────────────────────────────

info "Checking dependencies..."

if ! command -v cargo &>/dev/null; then
    warn "Rust not found. Installing via rustup..."
    if command -v curl &>/dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    elif command -v wget &>/dev/null; then
        wget -qO- https://sh.rustup.rs | sh -s -- -y
    else
        err "Neither curl nor wget found. Install one and retry."
    fi
    source "$HOME/.cargo/env"
    ok "Rust installed"
else
    ok "Rust found: $(rustc --version)"
fi

# ── Build ─────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

info "Building release binary..."
cd "$SCRIPT_DIR"
cargo build --release
ok "Build complete"

# ── Install ───────────────────────────────────────────────────

info "Installing to ${INSTALL_DIR}..."
mkdir -p "$INSTALL_DIR" "$INSTALL_DIR/config" "$INSTALL_DIR/data"
cp "$SCRIPT_DIR/target/release/freecc-relay" "$INSTALL_DIR/freecc-relay"
chmod 755 "$INSTALL_DIR/freecc-relay"
ok "Binary installed"

# ── Generate initial client key if no config exists ───────────

CONFIG_DIR="$INSTALL_DIR/config"
CONFIG_FILE="$CONFIG_DIR/server.json"

if [[ ! -f "$CONFIG_FILE" ]]; then
    info "Generating initial client key..."
    "$INSTALL_DIR/freecc-relay" --generate-key "default"
    ok "Client key generated (see output above)"
else
    ok "Existing config found at ${CONFIG_FILE}, skipping key generation"
fi

# ── Systemd service ──────────────────────────────────────────

info "Creating systemd service..."

cat > /etc/systemd/system/${SERVICE_NAME}.service <<EOF
[Unit]
Description=Free CC Relay Server
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/freecc-relay --port ${DEFAULT_PORT} --host 0.0.0.0
WorkingDirectory=${INSTALL_DIR}
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

# Hardening
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=${INSTALL_DIR}/config ${INSTALL_DIR}/data
ProtectHome=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable "$SERVICE_NAME"
systemctl restart "$SERVICE_NAME"
ok "Service installed and started"

# ── Done ─────────────────────────────────────────────────────

echo
echo -e "${GREEN}  ╔══════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}  ║     Installation complete!                    ║${NC}"
echo -e "${GREEN}  ╚══════════════════════════════════════════════╝${NC}"
echo
echo "  Useful commands:"
echo "    sudo systemctl status  ${SERVICE_NAME}"
echo "    sudo journalctl -u ${SERVICE_NAME} -f"
echo "    sudo systemctl restart ${SERVICE_NAME}"
echo
echo "  Generate additional client keys:"
echo "    ${INSTALL_DIR}/freecc-relay --generate-key \"name\""
echo
echo "  Config:  ${CONFIG_FILE}"
echo "  Logs:    journalctl -u ${SERVICE_NAME}"
echo
warn "The admin password is printed at startup — check the logs:"
echo "    sudo journalctl -u ${SERVICE_NAME} | grep 'Admin password'"
echo
