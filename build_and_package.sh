#!/bin/bash

# Exit on error
set -e

# Define directories
PROJECT_ROOT=$(pwd)
FRONTEND_DIR="$PROJECT_ROOT/frontend"
BACKEND_DIR="$PROJECT_ROOT/backend"
DIST_DIR="$PROJECT_ROOT/release"

echo "Step 1: Building Frontend..."
cd "$FRONTEND_DIR"
pnpm install
pnpm run build

echo "Step 2: Building Backend (Release)..."
cd "$BACKEND_DIR"
# The rust-embed macro will find files in frontend/dist at compile time
cargo build --release

echo "Step 3: Packaging..."
mkdir -p "$DIST_DIR"
cp "$BACKEND_DIR/target/release/coldstack" "$DIST_DIR/coldstack"

echo "Success! Your single binary is located at: $DIST_DIR/coldstack"
echo "You can now run it with: ./release/coldstack"
