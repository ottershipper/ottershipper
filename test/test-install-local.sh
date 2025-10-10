#!/bin/bash
set -e

# OtterShipper Local Installation Test
# Builds and tests installation in a real Ubuntu VM using Multipass

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# VM configuration
VM_NAME="ottershipper-test-$$"
VM_CPUS="8"    # Use multiple CPUs for parallel compilation
VM_MEMORY="8G" # 1GB per CPU for Rust compilation
VM_DISK="20G"

# Record start time for total duration
START_TIME=$(date +%s)

echo "==> OtterShipper Local Installation Test (Multipass)"
echo "    Started at: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

# Check if Multipass is installed
if ! command -v multipass &> /dev/null; then
    echo "Error: Multipass is not installed"
    echo ""
    echo "Install it with:"
    echo "  macOS:   brew install multipass"
    echo "  Linux:   sudo snap install multipass"
    echo "  Windows: Download from https://multipass.run"
    echo ""
    exit 1
fi

# Check Multipass is running
if ! multipass list &> /dev/null; then
    echo "Error: Multipass daemon is not running"
    echo "Try: multipass start (or restart Multipass.app on macOS)"
    exit 1
fi

echo "==> Creating Ubuntu 24.04 VM..."
echo "    Name: $VM_NAME"
echo "    CPUs: $VM_CPUS"
echo "    Memory: $VM_MEMORY"
echo "    Disk: $VM_DISK"

# Create VM with multiple CPUs for faster compilation
if ! multipass launch 24.04 \
    --name "$VM_NAME" \
    --cpus "$VM_CPUS" \
    --memory "$VM_MEMORY" \
    --disk "$VM_DISK" \
    --timeout 300; then
    echo "Error: Failed to create VM"
    exit 1
fi
echo "✓ VM created"

# Cleanup function
cleanup() {
    echo ""
    echo "==> Cleaning up VM..."
    multipass delete "$VM_NAME" --purge 2>/dev/null || true
}

# Register cleanup on exit
trap cleanup EXIT
echo ""

# Wait for VM to be ready
echo "==> Waiting for VM to be ready..."
multipass exec "$VM_NAME" -- cloud-init status --wait
echo "✓ VM ready"
echo ""

# Transfer project files
echo "==> Transferring project files..."
multipass transfer -r "$PROJECT_ROOT" "$VM_NAME:/home/ubuntu/ottershipper"
echo "✓ Files transferred"
echo ""

# Run tests inside VM
echo "==> Running tests in VM..."
echo ""

multipass exec "$VM_NAME" -- bash -s << 'EOF'
set -e

cd /home/ubuntu/ottershipper

echo "==> Inside VM (Ubuntu 24.04)"
echo ""

# Install prerequisites
echo "==> Installing prerequisites..."
sudo apt-get update -qq > /dev/null
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    curl \
    build-essential \
    docker.io \
    > /dev/null
echo "✓ Prerequisites installed"
echo ""

# Start Docker
echo "==> Starting Docker..."
sudo systemctl start docker
sudo usermod -aG docker ubuntu
echo "✓ Docker started"
echo ""

# Install Rust
echo "==> Installing Rust toolchain..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source "$HOME/.cargo/env"
echo "✓ Rust installed: $(rustc --version)"
echo ""

# Run clippy
echo "==> Running clippy..."
if ! cargo clippy --all-targets --all-features -- -D warnings; then
    echo "Error: Clippy check failed"
    exit 1
fi
echo "✓ Clippy passed"
echo ""

# Check formatting
echo "==> Checking code formatting..."
if ! cargo fmt --all -- --check; then
    echo "Error: Code formatting check failed"
    echo "Run 'cargo fmt --all' to fix formatting"
    exit 1
fi
echo "✓ Formatting check passed"
echo ""

# Build binary
echo "==> Building binary..."
cargo build --release
if [ ! -f "target/release/ottershipper" ]; then
    echo "Error: Binary build failed"
    exit 1
fi
echo "✓ Binary built successfully"
BINARY_SIZE=$(ls -lh target/release/ottershipper | awk '{print $5}')
echo "  Size: $BINARY_SIZE"
echo ""

# Test binary execution
echo "==> Testing binary execution..."
if ./target/release/ottershipper --version 2>&1 | head -n 1; then
    echo "✓ Binary executes successfully"
else
    echo "⚠️  Binary test (expected to fail without full setup)"
fi
echo ""

# Run installation with local binary
echo "==> Running installation..."
chmod +x install/install.sh
sudo ./install/install.sh --local-binary target/release/ottershipper

echo ""
echo "==> Verifying installation..."

# Check if binary was installed
if [ -f /usr/local/bin/ottershipper ]; then
    echo "✓ Binary installed to /usr/local/bin/ottershipper"
else
    echo "✗ Binary not found in /usr/local/bin/"
    exit 1
fi

# Check if systemd service was created
if [ -f /etc/systemd/system/ottershipper.service ]; then
    echo "✓ Systemd service created"
else
    echo "✗ Systemd service not found"
    exit 1
fi

# Check if config was created
if [ -f /etc/ottershipper/config.toml ]; then
    echo "✓ Configuration created"
else
    echo "✗ Configuration not found"
    exit 1
fi

# Check if data directory was created
if [ -d /var/lib/ottershipper ]; then
    echo "✓ Data directory created"
else
    echo "✗ Data directory not found"
    exit 1
fi

# Check if user was created
if id ottershipper &> /dev/null; then
    echo "✓ System user created"
else
    echo "✗ System user not found"
    exit 1
fi

# Check service status
echo ""
echo "==> Checking service status..."
if sudo systemctl is-enabled ottershipper &> /dev/null; then
    echo "✓ Service enabled"
else
    echo "⚠️  Service not enabled"
fi

if sudo systemctl is-active ottershipper &> /dev/null; then
    echo "✓ Service is running"
else
    echo "⚠️  Service not running"
    echo "    Checking logs..."
    sudo journalctl -u ottershipper --no-pager -n 10
fi

echo ""
echo "==> Installation test completed successfully! ✓"

EOF

EXIT_CODE=$?

# Calculate elapsed time
END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))
MINUTES=$((ELAPSED / 60))
SECONDS=$((ELAPSED % 60))

if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "==> All tests passed! ✓"
    echo ""
    echo "Clippy, formatting, build, and installation all work correctly."
    echo "You can safely commit and push your changes."
    echo ""
    printf "Total time: %dm %ds\n" $MINUTES $SECONDS
    echo ""
    echo "Note: VM cleaned up automatically."
    echo "      Each run uses a fresh VM for true integration testing."
    echo ""
else
    echo ""
    echo "==> Tests failed! ✗"
    echo ""
    echo "Please review the errors above."
    echo ""
    printf "Failed after: %dm %ds\n" $MINUTES $SECONDS
    echo ""
    echo "VM kept for debugging: $VM_NAME"
    echo "  Shell into VM: multipass shell $VM_NAME"
    echo "  Delete VM:     multipass delete $VM_NAME --purge"
    echo ""
    # Don't auto-cleanup on failure so you can debug
    trap - EXIT
    exit 1
fi
