#!/bin/bash
set -e

BINARY="pi-vitals"
PI_USER="${PI_USER:-your-pi-username}"
PI_HOST="${PI_HOST:-raspberrypi.local}"
PI_PATH="~/pi-vitals/"
TARGET="aarch64-unknown-linux-gnu"

echo "🔨 Building for Pi 3 (aarch64)..."
cargo build --target $TARGET --release

echo "📁 Ensuring remote directory exists..."
ssh $PI_USER@$PI_HOST "mkdir -p $PI_PATH"

echo "🚀 Deploying binary to Pi..."
scp target/$TARGET/release/$BINARY $PI_USER@$PI_HOST:$PI_PATH

echo ""
echo "✅ Deployed! To run on your Pi:"
echo "   ssh $PI_USER@$PI_HOST"
echo "   cd pi-vitals && ./$BINARY"
echo ""
echo "   Then open: http://$PI_HOST:3000"