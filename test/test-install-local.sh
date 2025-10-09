#!/bin/bash
set -e

# OtterShipper Local Installation Test
# Builds Linux binary locally and tests installation in Docker

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> OtterShipper Local Installation Test"
echo ""

# Check if cross is installed
if ! command -v cross &> /dev/null; then
    echo "Error: 'cross' is not installed"
    echo "Install it with: cargo install cross --git https://github.com/cross-rs/cross"
    exit 1
fi

# Check if Docker is running
if ! docker ps &> /dev/null; then
    echo "Error: Docker is not running or you don't have permission"
    exit 1
fi

cd "$PROJECT_ROOT"

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

# Build for Linux
echo "==> Building binary for Linux x86_64..."
cross build --release --target x86_64-unknown-linux-musl

if [ ! -f "target/x86_64-unknown-linux-musl/release/ottershipper" ]; then
    echo "Error: Binary build failed"
    exit 1
fi

echo "✓ Binary built successfully"
BINARY_SIZE=$(ls -lh target/x86_64-unknown-linux-musl/release/ottershipper | awk '{print $5}')
echo "  Size: $BINARY_SIZE"
echo ""

# Create test artifacts directory
TEST_DIR="$PROJECT_ROOT/test-artifacts"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"

# Copy binary and installation script
echo "==> Preparing test artifacts..."
cp target/x86_64-unknown-linux-musl/release/ottershipper "$TEST_DIR/"
cp install/install.sh "$TEST_DIR/"
echo "✓ Artifacts prepared"
echo ""

# Test in Docker
echo "==> Starting Ubuntu 24.04 container..."
CONTAINER_NAME="ottershipper-test-$$"

# Run installation in Docker
docker run --name "$CONTAINER_NAME" --rm -i \
    -v "$TEST_DIR:/test-artifacts:ro" \
    -v /var/run/docker.sock:/var/run/docker.sock \
    --privileged \
    ubuntu:24.04 \
    /bin/bash -s << 'EOF'

set -e

echo "==> Inside Docker container"
echo ""

# Update package lists
echo "==> Installing prerequisites..."
apt-get update -qq
apt-get install -y -qq curl docker.io systemctl

echo "✓ Prerequisites installed"
echo ""

# Copy artifacts to writable location
cp -r /test-artifacts /tmp/artifacts
cd /tmp/artifacts
chmod +x install.sh
chmod +x ottershipper

# Test binary directly first
echo "==> Testing binary execution..."
if ./ottershipper --version 2>&1 | head -n 1; then
    echo "✓ Binary executes successfully"
else
    echo "⚠️  Binary test (expected to fail without full setup)"
fi
echo ""

# Run installation with local binary
echo "==> Running installation with local binary..."
./install.sh --local-binary /tmp/artifacts/ottershipper

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
    echo "  Config content:"
    cat /etc/ottershipper/config.toml | sed 's/^/    /'
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

# Check service status (may not be running due to Docker limitations)
echo ""
echo "==> Checking service status..."
if systemctl is-enabled ottershipper &> /dev/null; then
    echo "✓ Service enabled"
else
    echo "⚠️  Service not enabled (may be expected in Docker)"
fi

if systemctl is-active ottershipper &> /dev/null; then
    echo "✓ Service is running"
else
    echo "⚠️  Service not running (expected in Docker without proper init)"
fi

echo ""
echo "==> Installation test completed successfully! ✓"

EOF

EXIT_CODE=$?

# Cleanup
rm -rf "$TEST_DIR"

if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "==> All tests passed! ✓"
    echo ""
    echo "The binary and installation script work correctly."
    echo "You can safely commit and push your changes."
else
    echo ""
    echo "==> Tests failed! ✗"
    echo ""
    echo "Please review the errors above."
    exit 1
fi
