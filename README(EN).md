# Simple Analog Clock for PC

Version 0.1.13 · Analog desktop clock for Windows, macOS, and Linux

To use the Windows x64 release, run `Simple Analog Clock for PC.exe`. See the build instructions below to compile it from source. Korean labels in parentheses match the app interface.

- Left-click and drag the clock to move it. On Windows, the clock snaps to a screen edge while you drag it nearby and detaches when you drag it inward. You can disable Snap to Edges (`가장자리 붙이기`) in Options (`옵션`).
- Right-click the clock to open the menu below and to the right of the pointer. Always on Top (`항상 위`) is enabled on first launch on Windows and macOS. This menu item is unavailable on Linux.
- Hover over Opacity (`불투명도`) to open a submenu with 20%, 40%, 60%, 80%, and 100%. The default is 100%.
- The Options (`옵션`) window opens to the left of the clock when space allows. Choose Ivory, Charcoal, or Navy under Design (`디자인`) and rounded or pointed hour and minute hands under Hand Shape (`바늘 모양`). Time Source (`시간 기준`) uses the current computer time by default. You can also select an IANA time zone by searching for a city such as `Seoul`.
- Clock Size (`시계 크기`), the first item in Options, sets the dial diameter from 116 to 400 px. The default is 116 px.
- Border Thickness (`테두리 두께`) defaults to 4 px. Set a fixed width from 2 to 14 px, or choose Auto (`자동`) to scale the border with the dial diameter. Changing the fixed border thickness changes the outer diameter while keeping the inner dial the same size.
- The default hands are pointed at 110% thickness. Hand Thickness (`바늘 두께`), immediately below Design, adjusts the hour and minute hands from 50% to 200%. At 100% display scaling, the second hand is 1 px thick in front of the pivot and 3 px behind it.
- Show Numerals (`숫자 표시`), below Hand Thickness, toggles the dial numerals. Numeral Font (`숫자 글꼴`) offers Roboto Condensed (the default), Clock Sans KR, Oswald, Noto Serif, and IBM Plex Mono. Font Size (`글꼴 크기`) ranges from 70% to 125% and defaults to 100%. Bold (`굵게`) and Show Numerals are enabled by default.
- To adjust a slider, hover over it and scroll the mouse wheel, or focus it and press the arrow keys.
- Click Reset (`초기화`) to restore the default options. Click OK (`확인`) to save or Cancel (`취소`) to discard the changes.
- Click OK (`확인(Enter)`) or press Enter to apply the options. Settings are saved in an `.ini` file alongside the executable, with the same base name. Delete the INI file to start with the defaults on the next launch.

## Building from source

You need Rust 1.88 or later and native build tools for your target platform. On Windows, the MSVC Rust toolchain requires Visual Studio C++ Build Tools and the Windows SDK. Run this command in the project folder.

```sh
cargo build --release --locked
```

The executable is `target/release/simple-analog-clock.exe` on Windows and `target/release/simple-analog-clock` on macOS and Linux.

Clock Sans KR is a renamed subset of Noto Sans KR. The SIL Open Font License for each font is included in the `assets` folder in the `OFL*.txt` files.
