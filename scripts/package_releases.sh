#!/usr/bin/env bash
# Builds all release artifacts into dist/ for a GitHub release.
#
# Requires: cargo, the Android toolchain (see scripts/build_android.sh) and the
# Windows cross target (rustup target add x86_64-pc-windows-gnu + mingw-w64).
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"
echo "Packaging MetroFlow $VERSION"
mkdir -p dist

# ── macOS: universal binary (arm64 + x86_64) in a .app, then a .dmg ──
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

APP="dist/MetroFlow.app"
rm -rf "$APP" dist/macos
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create \
    target/aarch64-apple-darwin/release/metroflow \
    target/x86_64-apple-darwin/release/metroflow \
    -output "$APP/Contents/MacOS/MetroFlow"

# .icns from the 1024px icon
ICONSET="target/MetroFlow.iconset"
rm -rf "$ICONSET" && mkdir -p "$ICONSET"
SRC=icon/icon_1024.png
sips -z 16 16      "$SRC" --out "$ICONSET/icon_16x16.png"      >/dev/null
sips -z 32 32      "$SRC" --out "$ICONSET/icon_16x16@2x.png"   >/dev/null
sips -z 32 32      "$SRC" --out "$ICONSET/icon_32x32.png"      >/dev/null
sips -z 64 64      "$SRC" --out "$ICONSET/icon_32x32@2x.png"   >/dev/null
sips -z 128 128    "$SRC" --out "$ICONSET/icon_128x128.png"    >/dev/null
sips -z 256 256    "$SRC" --out "$ICONSET/icon_128x128@2x.png" >/dev/null
sips -z 256 256    "$SRC" --out "$ICONSET/icon_256x256.png"    >/dev/null
sips -z 512 512    "$SRC" --out "$ICONSET/icon_256x256@2x.png" >/dev/null
sips -z 512 512    "$SRC" --out "$ICONSET/icon_512x512.png"    >/dev/null
sips -z 1024 1024  "$SRC" --out "$ICONSET/icon_512x512@2x.png" >/dev/null
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/MetroFlow.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>CFBundleName</key><string>MetroFlow</string>
    <key>CFBundleDisplayName</key><string>MetroFlow</string>
    <key>CFBundleIdentifier</key><string>com.daurge.metroflow</string>
    <key>CFBundleVersion</key><string>${VERSION}</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundleExecutable</key><string>MetroFlow</string>
    <key>CFBundleIconFile</key><string>MetroFlow</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>LSMinimumSystemVersion</key><string>10.15</string>
    <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST

# Ad-hoc signature (lets it run on Apple Silicon without a Developer ID)
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true

# .dmg with a drag-to-Applications shortcut
STAGE="target/dmg-stage"
rm -rf "$STAGE" && mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "MetroFlow" -srcfolder "$STAGE" -ov -format UDZO \
    "dist/MetroFlow-${VERSION}-macos.dmg" >/dev/null
# zip of the .app as a fallback
( cd dist && zip -q -r "MetroFlow-${VERSION}-macos.zip" MetroFlow.app )
echo "macOS   -> dist/MetroFlow-${VERSION}-macos.dmg (+ .zip)"

# ── Windows (x86_64) ────────────────────────────────────────────
cargo build --release --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/metroflow.exe \
   "dist/MetroFlow-${VERSION}-windows-x86_64.exe"
echo "Windows -> dist/MetroFlow-${VERSION}-windows-x86_64.exe"

# ── Linux (x86_64): native on Linux, else via Docker, else CI ────
LINUX_OUT="dist/MetroFlow-${VERSION}-linux-x86_64"
if [ "$(uname -s)" = "Linux" ]; then
    cargo build --release
    cp target/release/metroflow "$LINUX_OUT" && chmod +x "$LINUX_OUT"
    echo "Linux   -> $LINUX_OUT"
elif command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    docker run --rm --platform linux/amd64 -v "$PWD":/work -w /work rust:1-bookworm \
      bash -lc "apt-get update -qq && apt-get install -y -qq libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev pkg-config >/dev/null && cargo build --release --target x86_64-unknown-linux-gnu && cp target/x86_64-unknown-linux-gnu/release/metroflow /work/$LINUX_OUT && chmod +x /work/$LINUX_OUT"
    echo "Linux   -> $LINUX_OUT (via Docker)"
else
    echo "Linux   -> skip (no Linux host / Docker); CI release workflow builds it"
fi

# ── Android (APK) ───────────────────────────────────────────────
bash scripts/build_android.sh
cp target/android-artifacts/release/apk/metroflow.apk \
   "dist/MetroFlow-${VERSION}-android.apk"
echo "Android -> dist/MetroFlow-${VERSION}-android.apk"

echo
echo "Artifacts:"
ls -lh dist/MetroFlow-"${VERSION}"-* 2>/dev/null
echo
cat <<EOF
Publish to GitHub:
  git add -A && git commit -m "Release v${VERSION}"
  git tag v${VERSION}
  git push origin main --tags
  gh release create v${VERSION} dist/MetroFlow-${VERSION}-* \\
     --title "MetroFlow ${VERSION}" --notes-file CHANGELOG.md
EOF
