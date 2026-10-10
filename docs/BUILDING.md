# Сборка MetroFlow под разные платформы

Кратко: приложение кросс-платформенное (Rust + Macroquad). Ниже — как собрать
релиз под macOS, Windows, Linux и Android, чтобы отправлять друзьям.

> Во всех командах после установки Rust: `source "$HOME/.cargo/env"`.

---

## macOS

```sh
rustup target add aarch64-apple-darwin      # Apple Silicon (или x86_64-apple-darwin)
cargo build --release
```

Готовый бинарник: `target/release/metroflow` (запускается двойным кликом из
терминала: `./target/release/metroflow`).

Чтобы сделать `.app`-бандл (перетаскивается в «Программы»/отправляется друзьям):

```sh
APP=MetroFlow.app
mkdir -p "$APP/Contents/MacOS"
cp target/release/metroflow "$APP/Contents/MacOS/MetroFlow"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>MetroFlow</string>
  <key>CFBundleExecutable</key><string>MetroFlow</string>
  <key>CFBundleIdentifier</key><string>com.metroflow.game</string>
  <key>CFBundleVersion</key><string>1.0</string>
  <key>CFBundlePackageType</key><string>APPL</string>
</dict></plist>
PLIST
```

Для распространения без предупреждений Gatekeeper нужна подпись/нотаризация
(Apple Developer ID) — иначе друзьям придётся открывать через
«ПКМ → Открыть». Это отдельный шаг.

---

## Windows

### Вариант A — собрать на Windows (рекомендуется)

Установите Rust (rustup с https://rustup.rs, выберите toolchain MSVC и
Visual Studio Build Tools с «Desktop development with C++»), затем:

```powershell
cargo build --release
```

Готовый файл: `target\release\metroflow.exe`. Отправляйте его — он
самодостаточный (может понадобиться установленный
[Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)).

### Вариант B — кросс-сборка из macOS/Linux (экспериментально)

```sh
rustup target add x86_64-pc-windows-gnu
brew install mingw-w64                       # macOS
cargo build --release --target x86_64-pc-windows-gnu
```

Файл: `target/x86_64-pc-windows-gnu/release/metroflow.exe`.
С сборкой графики (miniquad) кросс-компиляция иногда капризна — если не соберётся,
используйте вариант A или CI на windows-раннере.

---

## Linux

На Debian/Ubuntu нужны X11/OpenGL-заголовки:

```sh
sudo apt-get install -y libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev
rustup target add x86_64-unknown-linux-gnu
cargo build --release
```

Файл: `target/release/metroflow`. На другой машине нужны те же системные
библиотеки (или собрать AppImage/`.deb`).

---

## Android

Проще всего — готовым скриптом `scripts/build_android.sh` (он делает всё ниже
автоматически). Ручной рецепт для macOS (Apple Silicon), проверен с NDK 28 и
build-tools 36.

**Предпосылки** (через Android Studio → SDK Manager): Android SDK, NDK
(«NDK (Side by side)»), Command-line Tools, build-tools, `platform-tools`.

```sh
rustup target add aarch64-linux-android        # arm64 (телефоны)
cargo install cargo-quad-apk                    # сборщик APK для macroquad
```

> Грабля: `cargo-apk` считает имя нативной библиотеки равным **последнему
> сегменту** `package_name` и делает `System.loadLibrary("<сегмент>")`. Поэтому
> последний сегмент обязан совпадать с именем крейта: у нас
> `package_name = "com.metroflow.metroflow"` (библиотека `libmetroflow.so`).
> Иначе приложение падает с `UnsatisfiedLinkError: lib<...>.so not found`.
>
> Также для Android 12+ в `[package.metadata.android.activity_attributes]` задано
> `android:exported = "true"` (обязательно) и `android:screenOrientation = "landscape"`
> (игра всегда в альбомной ориентации).

> В `Cargo.toml` уже есть секция `[package.metadata.android]`
> (`package_name`, `label`, `android_version = 34`, `build_targets`).
> Старый `cargo-quad-apk` не читает `Cargo.lock` версии 4, а обычный `cargo`
> после изменений манифеста снова пишет v4. **Скрипт `build_android.sh` сам
> понижает версию lock до 3** перед сборкой — просто запускайте его.

**Особенности нового NDK/build-tools** (из-за них старому cargo-apk нужны
обходы — всё это делает скрипт):
1. NDK 27+ отдаёт только `llvm-*` без per-target `ar`/`ld`/`readelf` —
   создаются симлинки `aarch64-linux-android-{ar,ranlib,ld,readelf,...}`.
2. `libunwind` лежит в рантайме clang — симлинк в `lib/gcc/aarch64-linux-android/4.9.x`.
3. `javac` для `cargo-apk` ищет `rt.jar` (только JDK 8): ставим Temurin 8
   (`~/.jdks/temurin-8-jdk`, x64 под Rosetta) и используем только для javac.
4. В build-tools ≥26 нет `dx` (есть `d8`) — кладём `dx`-shim, который
   транслирует вызовы в `d8`, запуская `d8` под Java 11+ из Android Studio JBR.

**Сборка и установка:**

```sh
./scripts/build_android.sh
# APK: target/android-artifacts/release/apk/metroflow.apk

adb devices
adb install -r target/android-artifacts/release/apk/metroflow.apk
```

Или одной командой — сборка, установка и запуск с логами:

```sh
cargo quad-apk run   --release --target aarch64-linux-android
cargo quad-apk logcat
```

**Эмулятор:** у него ABI `x86_64`. Добавьте в `[package.metadata.android]`
`build_targets = ["aarch64-linux-android", "x86_64-linux-android"]`, поставьте
`rustup target add x86_64-linux-android` и соберите с
`--target x86_64-linux-android`.

Сборка проверена: APK собирается, подписывается debug-ключом и проходит
`zipalign`/`apksigner`.

---

## Общие замечания

- Проверки перед сборкой: `cargo fmt --check`, `cargo clippy --all-targets`,
  `cargo test`.
- Релиз оптимизирован (см. `[profile.release]` в `Cargo.toml`: `lto`, `opt-level=3`).
- Файл `opencode.json` и `AGENTS.md` — служебные (для разработки), в
  дистрибутив не нужны.
