# Simple Analog Clock for PC

Version 0.1.15 · Analog desktop clock for Windows, macOS, and Linux

To use the Windows x64 release, run `Simple Analog Clock for PC.exe`. See the build instructions below to compile it from source. The app starts in Korean by default.


## Building from source

You need Rust 1.88 or later and native build tools for your target platform. On Windows, the MSVC Rust toolchain requires Visual Studio C++ Build Tools and the Windows SDK. Run this command in the project folder.

```sh
cargo build --release --locked
```

The executable is `target/release/simple-analog-clock.exe` on Windows and `target/release/simple-analog-clock` on macOS and Linux.

Clock Sans KR is a renamed subset of Noto Sans KR. The SIL Open Font License for each font is included in the `assets` folder in the `OFL*.txt` files.
