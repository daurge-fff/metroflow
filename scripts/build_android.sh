#!/usr/bin/env bash
# Сборка Android APK для MetroFlow (macOS, Apple Silicon).
#
# Требуется:
#   - Android SDK + NDK (через Android Studio SDK Manager),
#   - rustup target aarch64-linux-android,
#   - cargo-quad-apk (cargo install cargo-quad-apk),
#   - JDK 8 (только для javac: cargo-apk ищет rt.jar),
#   - Java 11+ из Android Studio JBR (для d8 — используется внутри dx-shim).
#
# Особенности NDK 27+/28: убраны per-target binutils и отдельный libunwind,
# поэтому билд-скрипт ниже создаёт нужные симлинки автоматически.

set -euo pipefail
cd "$(dirname "$0")/.."

# Старый cargo-quad-apk не умеет читать Cargo.lock версии 4 — понижаем до 3.
if [ -f Cargo.lock ]; then
  sed -i '' 's/^version = 4$/version = 3/' Cargo.lock 2>/dev/null || sed -i 's/^version = 4$/version = 3/' Cargo.lock
fi

export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
export ANDROID_SDK_ROOT="$ANDROID_HOME"

NDK_VER="$(ls "$ANDROID_HOME/ndk" 2>/dev/null | sort -V | tail -1)"
export NDK_HOME="$ANDROID_HOME/ndk/$NDK_VER"
export ANDROID_NDK_HOME="$NDK_HOME"
echo "NDK: $NDK_HOME"

PRE="$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64"
BIN="$PRE/bin"

# 1) Симлинки binutils (NDK 27+ отдаёт только llvm-*).
for t in aarch64-linux-android arm-linux-androideabi armv7a-linux-androideabi armv7-linux-androideabi x86_64-linux-android i686-linux-android; do
  ln -sf llvm-ar "$BIN/$t-ar"
  ln -sf llvm-ranlib "$BIN/$t-ranlib"
  ln -sf ld.lld "$BIN/$t-ld"
  for tool in readelf objcopy strip nm strings; do ln -sf "llvm-$tool" "$BIN/$t-$tool"; done
done

# 2) libunwind из рантайма clang кладём туда, где его ищет линкер.
CLANG_LIB="$(ls -d "$PRE"/lib/clang/*/lib/linux/aarch64 | tail -1)"
GCC="$PRE/lib/gcc/aarch64-linux-android/4.9.x"
mkdir -p "$GCC"
ln -sf "$CLANG_LIB/libunwind.a" "$GCC/libunwind.a"

# 3) dx-shim: старый cargo-apk зовёт `dx`, а в build-tools ≥26 есть только `d8`.
BT_DIR="$(ls -d "$ANDROID_HOME"/build-tools/* | sort -V | tail -1)"
cat > "$BT_DIR/dx" <<'SH'
#!/bin/sh
JBR="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
out="classes.dex"; minapi=1; inputs=""
while [ $# -gt 0 ]; do
  case "$1" in
    --dex) shift ;;
    --output=*) out="${1#--output=}"; shift ;;
    --output) out="$2"; shift 2 ;;
    --min-sdk-version) minapi="$2"; shift 2 ;;
    --min-api) minapi="$2"; shift 2 ;;
    *) inputs="$inputs $1"; shift ;;
  esac
done
expanded=""
for inp in $inputs; do
  if [ -d "$inp" ]; then expanded="$expanded $(find "$inp" -name '*.class')"; else expanded="$expanded $inp"; fi
done
exec env JAVA_HOME="$JBR" PATH="$JBR/bin:$PATH" "$(dirname "$0")/d8" --output "$(dirname "$out")" --min-api "$minapi" $expanded
SH
chmod +x "$BT_DIR/dx"

# 4) JDK 8 для javac.
export JAVA_HOME="${JDK8_HOME:-$HOME/.jdks/temurin-8-jdk/Contents/Home}"
export PATH="$JAVA_HOME/bin:$ANDROID_HOME/platform-tools:$PATH"
echo "javac: $("$JAVA_HOME/bin/javac" -version 2>&1)"

cargo quad-apk build --release --target aarch64-linux-android
echo
echo "Готово: target/android-artifacts/release/apk/metroflow.apk"
