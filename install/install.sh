#!/bin/bash
set -euo pipefail

# OtterShipper Installation Script for Ubuntu 24.04
# Downloads pre-built binary from GitHub releases with checksum verification

REPO="ottershipper/ottershipper"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/ottershipper"
DATA_DIR="/var/lib/ottershipper"
SERVICE_FILE="/etc/systemd/system/ottershipper.service"
USER="ottershipper"
TMP_DIR=$(mktemp -d)

# Parse arguments
USE_NIGHTLY=false
LOCAL_BINARY=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --nightly)
            USE_NIGHTLY=true
            shift
            ;;
        --local-binary)
            LOCAL_BINARY="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--nightly] [--local-binary PATH]"
            exit 1
            ;;
    esac
done

# Cleanup function
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# Error handler
error_exit() {
    echo "Error: $1" >&2
    exit 1
}

# Logging functions
log_info() {
    echo "==> $1"
}

log_step() {
    echo "    $1"
}

log_info "OtterShipper Installation Script"
if [ "$USE_NIGHTLY" = true ]; then
    log_step "Installing NIGHTLY build (unstable)"
else
    log_step "Installing STABLE release"
fi
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    error_exit "Please run as root or with sudo"
fi

# Check OS
if [ ! -f /etc/os-release ]; then
    error_exit "Cannot detect OS. /etc/os-release not found"
fi

. /etc/os-release
if [ "$ID" != "ubuntu" ]; then
    echo "Warning: This script is designed for Ubuntu 24.04. Detected: $ID $VERSION_ID"
    echo "Continue anyway? (y/N)"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        exit 0
    fi
fi

# Detect architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        if [ "$USE_NIGHTLY" = true ]; then
            BINARY_NAME="ottershipper-nightly-linux-x86_64"
            CHECKSUM_FILE="SHA256SUMS-nightly"
        else
            BINARY_NAME="ottershipper-linux-x86_64"
            CHECKSUM_FILE="SHA256SUMS"
        fi
        ;;
    aarch64|arm64)
        if [ "$USE_NIGHTLY" = true ]; then
            BINARY_NAME="ottershipper-nightly-linux-aarch64"
            CHECKSUM_FILE="SHA256SUMS-nightly"
        else
            BINARY_NAME="ottershipper-linux-aarch64"
            CHECKSUM_FILE="SHA256SUMS"
        fi
        ;;
    *)
        error_exit "Unsupported architecture: $ARCH"
        ;;
esac

log_info "Detected architecture: $ARCH"
echo ""

# Check for required commands
for cmd in curl sha256sum systemctl; do
    if ! command -v $cmd &> /dev/null; then
        error_exit "Required command not found: $cmd"
    fi
done

# Install Docker if not present
if ! command -v docker &> /dev/null; then
    log_info "Installing Docker..."
    apt-get update -qq || error_exit "Failed to update package lists"
    apt-get install -y -qq ca-certificates curl gnupg || error_exit "Failed to install Docker prerequisites"

    install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc || error_exit "Failed to download Docker GPG key"
    chmod a+r /etc/apt/keyrings/docker.asc

    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu \
      $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
      tee /etc/apt/sources.list.d/docker.list > /dev/null

    apt-get update -qq || error_exit "Failed to update package lists after adding Docker repo"
    apt-get install -y -qq docker-ce docker-ce-cli containerd.io || error_exit "Failed to install Docker"

    systemctl enable docker || error_exit "Failed to enable Docker service"
    systemctl start docker || error_exit "Failed to start Docker service"

    # Wait for Docker to be ready
    sleep 2
    if ! docker ps &> /dev/null; then
        error_exit "Docker installed but not responding"
    fi

    log_step "Docker installed successfully"
else
    log_info "Docker already installed"

    # Verify Docker is running
    if ! docker ps &> /dev/null; then
        log_step "Starting Docker service..."
        systemctl start docker || error_exit "Failed to start Docker service"
        sleep 2
    fi
fi
echo ""

# Check if port 3000 is available
log_info "Checking port availability..."
if ss -tlnp 2>/dev/null | grep -q ":3000 " || netstat -tuln 2>/dev/null | grep -q ":3000 "; then
    error_exit "Port 3000 is already in use. Please stop the service using this port first."
fi
log_step "Port 3000 is available"
echo ""

# Download or use local binary
if [ -n "$LOCAL_BINARY" ]; then
    log_info "Using local binary: $LOCAL_BINARY"

    if [ ! -f "$LOCAL_BINARY" ]; then
        error_exit "Local binary not found: $LOCAL_BINARY"
    fi

    # Copy local binary to temp dir
    cp "$LOCAL_BINARY" "$TMP_DIR/$BINARY_NAME"
    log_step "Local binary copied"

    # Generate checksum for local binary
    cd "$TMP_DIR"
    sha256sum "$BINARY_NAME" > "$CHECKSUM_FILE"
    log_step "Checksum generated for local binary"
    echo ""
else
    # Get release version
    if [ "$USE_NIGHTLY" = true ]; then
        log_info "Fetching nightly build..."
        RELEASE_TAG="nightly"
        log_step "Using nightly tag"
    else
        log_info "Fetching latest release..."
        RELEASE_TAG=$(curl -fsSL https://api.github.com/repos/$REPO/releases/latest | grep '"tag_name"' | cut -d'"' -f4)

        if [ -z "$RELEASE_TAG" ]; then
            error_exit "Could not fetch latest version from GitHub API"
        fi

        log_step "Latest version: $RELEASE_TAG"
    fi

    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$RELEASE_TAG/$BINARY_NAME"
    CHECKSUM_URL="https://github.com/$REPO/releases/download/$RELEASE_TAG/$CHECKSUM_FILE"
    log_step "Download URL: $DOWNLOAD_URL"
    echo ""

    # Download checksum file
    log_info "Downloading checksums..."
    if ! curl -fsSL -o "$TMP_DIR/$CHECKSUM_FILE" "$CHECKSUM_URL"; then
        error_exit "Failed to download checksum file from $CHECKSUM_URL"
    fi
    log_step "Checksums downloaded"

    # Download binary
    log_info "Downloading OtterShipper binary..."
    if ! curl -fsSL -o "$TMP_DIR/$BINARY_NAME" "$DOWNLOAD_URL"; then
        error_exit "Failed to download binary from $DOWNLOAD_URL"
    fi
    log_step "Binary downloaded"

    # Verify checksum
    log_info "Verifying binary integrity..."
    cd "$TMP_DIR"
    if ! grep "$BINARY_NAME" "$CHECKSUM_FILE" | sha256sum -c - &> /dev/null; then
        error_exit "Checksum verification failed! Binary may be corrupted or tampered with."
    fi
    log_step "✓ Checksum verified"
    echo ""
fi

# Stop service if running (for upgrades)
if systemctl is-active --quiet ottershipper; then
    log_info "Stopping existing OtterShipper service..."
    systemctl stop ottershipper || log_step "Warning: Failed to stop service"
    echo ""
fi

# Backup existing binary if present
if [ -f "$INSTALL_DIR/ottershipper" ]; then
    log_info "Backing up existing binary..."
    cp "$INSTALL_DIR/ottershipper" "$INSTALL_DIR/ottershipper.backup.$(date +%s)" || log_step "Warning: Failed to create backup"
    echo ""
fi

# Install binary
log_info "Installing binary to $INSTALL_DIR/ottershipper..."
cp "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/ottershipper" || error_exit "Failed to copy binary"
chmod +x "$INSTALL_DIR/ottershipper" || error_exit "Failed to set binary permissions"
log_step "Binary installed successfully"
echo ""

# Create system user
if ! id -u $USER &> /dev/null; then
    log_info "Creating system user: $USER..."
    useradd --system --no-create-home --shell /usr/sbin/nologin $USER || error_exit "Failed to create user"
    log_step "User created"
else
    log_info "User $USER already exists"
fi
echo ""

# Create directories
log_info "Creating directories..."
mkdir -p $CONFIG_DIR || error_exit "Failed to create config directory"
mkdir -p $DATA_DIR || error_exit "Failed to create data directory"
chown $USER:$USER $DATA_DIR || error_exit "Failed to set directory ownership"
log_step "Directories created"
echo ""

# Install config file
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    log_info "Creating default configuration..."
    cat > "$CONFIG_DIR/config.toml" <<'EOF'
[server]
transport = "http"
bind_address = "0.0.0.0"
port = 3000

[database]
path = "/var/lib/ottershipper/ottershipper.db"
EOF
    chmod 644 "$CONFIG_DIR/config.toml" || error_exit "Failed to set config permissions"
    log_step "Configuration created at $CONFIG_DIR/config.toml"
else
    log_info "Configuration already exists at $CONFIG_DIR/config.toml"
    log_step "Keeping existing configuration"
fi
echo ""

# Install systemd service
log_info "Installing systemd service..."
cat > "$SERVICE_FILE" <<'EOF'
[Unit]
Description=OtterShipper - AI-First Deployment Platform
Documentation=https://github.com/ottershipper/ottershipper
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
User=ottershipper
Group=ottershipper
ExecStart=/usr/local/bin/ottershipper
WorkingDirectory=/var/lib/ottershipper
Restart=on-failure
RestartSec=5s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/ottershipper

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=ottershipper

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload || error_exit "Failed to reload systemd"
log_step "Systemd service installed"
echo ""

# Enable and start service
log_info "Enabling and starting OtterShipper service..."
systemctl enable ottershipper || error_exit "Failed to enable service"
systemctl start ottershipper || error_exit "Failed to start service"
log_step "Service started"
echo ""

# Health check
log_info "Performing health check..."
sleep 3

if systemctl is-active --quiet ottershipper; then
    log_step "✓ Service is running"
else
    error_exit "Service failed to start. Check logs: journalctl -u ottershipper -n 50"
fi

# Check if port is listening (for HTTP mode)
if grep -q 'transport = "http"' "$CONFIG_DIR/config.toml" 2>/dev/null; then
    PORT=$(grep -A3 '\[server\]' "$CONFIG_DIR/config.toml" | grep 'port' | cut -d'=' -f2 | tr -d ' ' || echo "3000")
    sleep 2
    if ss -tlnp 2>/dev/null | grep -q ":$PORT "; then
        log_step "✓ HTTP server listening on port $PORT"
    else
        log_step "Warning: Port $PORT does not appear to be listening yet"
        log_step "Check logs: journalctl -u ottershipper -f"
    fi
fi
echo ""

# Show status
log_info "Installation complete!"
echo ""
echo "Service status:"
systemctl status ottershipper --no-pager -l || true
echo ""
echo "Next steps:"
echo "  1. Configure: Edit /etc/ottershipper/config.toml"
echo "  2. Restart: sudo systemctl restart ottershipper"
echo "  3. View logs: sudo journalctl -u ottershipper -f"
echo "  4. MCP endpoint: http://YOUR_SERVER_IP:3000/sse"
echo ""
echo "Documentation: https://github.com/ottershipper/ottershipper"
echo ""

if [ "$USE_NIGHTLY" = true ]; then
    echo "⚠️  You installed a NIGHTLY build. This is unstable development version."
    echo "   To switch to stable: curl -sSL ... | sudo bash"
    echo ""
fi
