# QuickClock (Rust)

A native fullscreen clock: shows the time (HH:MM:SS) centered on screen with
the date and weekday at the bottom. Press `ESC` or `Q` to close. Launching a second
instance restores and raises the existing window instead of opening a new one.

Rust rewrite of the original Python/pygame version, built for instant startup
and single-file distribution.

## Build

```
cargo build --release
```

The executable is `target/release/QuickClock.exe` (~0.7 MB, no runtime
dependencies, no console window).

## Features

- Fullscreen dark background with cyan text, matching the Python version
- Time in Consolas, date and weekday in Microsoft YaHei (with fallback chains)
- Local timezone, updates every second (event-driven, ~0% idle CPU)
- DPI-aware font scaling
- Single instance via a named Win32 mutex; relaunching raises the running window

## Differences from the Python version

- Single-file native exe instead of a PyInstaller folder (no extraction, no
  temp files, millisecond startup)
- `cargo build --release` replaces the PyInstaller/cx_Freeze packaging step
- Windows only (Win32 mutex and window discovery for the single-instance guard)

## Development

```
cargo test      # unit tests (time formatting, font fallback, rasterizer, layout)
cargo run       # debug build
```
