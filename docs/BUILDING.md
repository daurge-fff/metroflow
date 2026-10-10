# Сборка MetroFlow под разные платформы

Кратко: приложение кросс-платформенное (Rust + Macroquad). Ниже — как собрать
релиз под macOS, Windows, Linux и Android, чтобы отправлять друзьям.

> Во всех командах после установки Rust: `source "$HOME/.cargo/env"`.

---

## macOS

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release            # host (Apple Silicon по умолчанию)
```

Готовый бинарник: `target/release/metroflow`.

**`.app` (universal, arm64 + Intel) и `.dmg`** собираются скриптом
`scripts/package_releases.sh` — он делает `lipo` (обе архитектуры в одном
файле), иконку `.icns` (`sips` + `iconutil`), `Info.plist`, ad-hoc подпись
(`codesign -s -`) и `.dmg` (`hdiutil`) с ярлыком-ссылкой на «Программы». Тот же
скрипт собирает Windows и Android:

```sh
bash scripts/package_releases.sh
# dist/MetroFlow-<ver>-macos.dmg (+ .zip), -windows-x86_64.exe, -android.apk
```

Без нотаризации Apple при первом запуске возможно предупреждение Gatekeeper
(обойти: «ПКМ → Открыть» либо `xattr -dr com.apple.quarantine MetroFlow.app`).
Чтобы убрать его совсем, нужен Apple Developer ID и `notarytool` — отдельный шаг.

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

### Вариант B — кросс-сборка из macOS/Linux

```sh
rustup target add x86_64-pc-windows-gnu
brew install mingw-w64                       # macOS
cargo build --release --target x86_64-pc-windows-gnu
```

Файл: `target/x86_64-pc-windows-gnu/release/metroflow.exe`. Линкер уже прописан
в `.cargo/config.toml` (`x86_64-w64-mingw32-gcc`), так что ничего
донастраивать не нужно. Бинарь GUI (`windows_subsystem = "windows"`) — окно
консоли не открывается.

---

## Linux

На Debian/Ubuntu нужны X11/OpenGL-заголовки:

```sh
sudo apt-get install -y libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev
rustup target add x86_64-unknown-linux-gnu
cargo build --release
```

Файл: `target/release/metroflow`. На другой машине нужны те же системные
библиотеки.

Кросс-сборка под Linux из macOS: `scripts/package_releases.sh` собирает
x86_64-бинарник в контейнере `rust:1-bookworm` через Docker
(`--platform linux/amd64`). Либо тот же артефакт собирает CI (`.github/workflows/release.yml`)
при пуше тега `v*`.

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

## Релиз для GitHub

Скрипт `scripts/package_releases.sh` собирает все артефакты в `dist/` и печатает
готовую команду:

```sh
git add -A && git commit -m "Release v0.16.0"
git tag v0.16.0 && git push origin main --tags
gh release create v0.16.0 dist/MetroFlow-0.16.0-* \
   --title "MetroFlow 0.16.0" --notes-file CHANGELOG.md
```

Или через сайт: **Releases → Draft a new release** → тег → перетащить файлы из
`dist/` в **Attach binaries** → Publish.

## Общие замечания

- Проверки перед сборкой (и в CI): `cargo fmt --check`, `cargo clippy --all-targets`,
  `cargo test`.
- Релиз оптимизирован (см. `[profile.release]` в `Cargo.toml`: `lto`, `opt-level=3`).
- Иконки лежат в `icon/` (исходники `.svg`/`.mvg`/`.png`) и `android/res/mipmap-*`
  (готовые плотности для APK) — это исходники, их нужно коммитить.
- Служебные файлы `opencode.json` и `AGENTS.md` — в `.gitignore` (в публичный
  репозиторий не попадают).
