
#!/usr/bin/env bash
set -e

BIN_NAME="quantum_launcher"
PROFILE="release-ql"
RUST_TARGET="aarch64-apple-darwin"

# macOS Sequoia 15 only
export MACOSX_DEPLOYMENT_TARGET=15.0

echo "Adding target..."
rustup target add "$RUST_TARGET"

echo "Building (arm64, macOS 15)..."
cargo build --profile "$PROFILE" --target "$RUST_TARGET"

echo "Creating .app bundle..."
APP_DIR="dist/QuantumLauncher.app"
MACOS_DIR="$APP_DIR/Contents/MacOS"
RESOURCES_DIR="$APP_DIR/Contents/Resources"

rm -rf dist
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

cp "target/$RUST_TARGET/$PROFILE/$BIN_NAME" "$MACOS_DIR/$BIN_NAME"
cp assets/freedesktop/Info.plist "$APP_DIR/Contents/Info.plist"

sips -s format icns "assets/icon/ql_logo.png" \
  --out "$RESOURCES_DIR/ql_logo.icns"

echo "Done → $APP_DIR"
