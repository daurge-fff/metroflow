# Сторонние ресурсы и зависимости

В этом файле перечислены внешние ресурсы, используемые MetroFlow, с указанием
источника, лицензии и требуемой атрибуции. Версии зафиксированы в
[`Cargo.toml`](Cargo.toml) и `Cargo.lock`.

## Шрифты

| Ресурс | Файлы | Источник | Лицензия |
| ------ | ----- | -------- | -------- |
| PT Sans (Regular, Bold) | `assets/fonts/PTSans-Regular.ttf`, `assets/fonts/PTSans-Bold.ttf` | ParaType / Google Fonts | SIL Open Font License 1.1 |
| Material Icons | `assets/icons/MaterialIcons-Regular.ttf` | Google Material Design Icons | Apache License 2.0 |

Тексты лицензий: [`assets/fonts/OFL.txt`](assets/fonts/OFL.txt) и
[`assets/icons/LICENSE.txt`](assets/icons/LICENSE.txt).

Оба ресурса включены в бинарник. PT Sans распространяется на условиях SIL
OFL 1.1, иконки — на условиях Apache-2.0. При распространении приложения
сохраняйте соответствующие файлы лицензий рядом с исходниками. Соответствие
имён и кодовых точек иконок — в
`assets/icons/MaterialIcons-Regular.codepoints`.

## Прямые зависимости Rust

| Crate | Версия | Лицензия |
| ----- | ------ | -------- |
| macroquad | 0.4.16 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.151 | MIT OR Apache-2.0 |
| directories | 5.0.1 | MIT OR Apache-2.0 |

## Транзитивные зависимости

Полный список — `cargo tree`. Лицензии транзитивных зависимостей:

- **MIT OR Apache-2.0**: miniquad, glam, quad-rand, fontdue, image, png,
  flate2, miniz_oxide, hashbrown, itoa, memchr, crc32fast, cfg-if, libc,
  serde_derive, serde_core, syn, quote, proc-macro2, num-traits, byteorder,
  byteorder, allocator-api2, equivalent, adler2, simd-adler32, fdeflate,
  core_maths, color_quant, ttf-parser, macroquad_macro.
- **иные совместимые лицензии**:
  - `libm` — MIT;
  - `foldhash` — Zlib;
  - `zmij` — MIT;
  - `objc-rs`, `malloc_buf` — MIT;
  - `option-ext` — MPL-2.0 (используется транзитивно через `dirs-sys`;
    MPL-2.0 — копилефт на уровне файлов и не накладывает ограничений на
    собственный код MetroFlow);
  - `bytemuck` — Zlib OR Apache-2.0 OR MIT;
  - `unicode-ident` — (MIT OR Apache-2.0) AND Unicode-3.0.

## Ассеты игры

Все игровые визуальные элементы карты (линии, станции, поезда, фон) **нарисованы
программно** средствами Macroquad. Интерфейс использует набор иконок
Material Icons (Apache-2.0), отрисованных векторным иконочным шрифтом.
Растровые изображения и звуки в проекте не используются.

## Лицензия проекта

Код MetroFlow распространяется под лицензией **PolyForm Noncommercial 1.0.0**
(см. `LICENSE`). Сторонние зависимости распространяются на условиях,
перечисленных выше; их лицензии и уведомления необходимо сохранять при
распространении.
