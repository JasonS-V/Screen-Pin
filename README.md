<p align="center">
  <img src="static/favicon.svg" alt="Screen-Pin logo" width="96" height="96">
</p>
<h1 align="center">Screen-Pin</h1>
<p align="center">A screen crop that stays on top, with local memory.</p>
<p align="center"><a href="https://github.com/JasonS-V/Screen-Pin/releases/tag/v0.1.0"><strong>Download v0.1.0</strong></a></p>

Screen-Pin is a desktop app for Windows and macOS. Capture a region, keep it floating above other windows, then find its image or recognized text in your local library. OCR and storage stay on your computer.

## Download

| Platform | Package |
| --- | --- |
| Windows x64 | [NSIS installer](https://github.com/JasonS-V/Screen-Pin/releases/download/v0.1.0/Screen-Pin_0.1.0_x64-setup.exe) |
| Windows x64 | [Portable ZIP](https://github.com/JasonS-V/Screen-Pin/releases/download/v0.1.0/Screen-Pin_0.1.0_x64_portable.zip) |
| macOS Apple Silicon | [App ZIP](https://github.com/JasonS-V/Screen-Pin/releases/download/v0.1.0/Screen-Pin_0.1.0_macos_arm64.zip) |
| macOS Intel | [App ZIP](https://github.com/JasonS-V/Screen-Pin/releases/download/v0.1.0/Screen-Pin_0.1.0_macos_x64.zip) |

The Windows portable package needs a writable folder and Microsoft Edge WebView2 Runtime. Keep the executable, DLLs, `resources`, and `portable.flag` together. Captures and the database are stored in a `data` folder beside the executable, so move the entire folder to keep them.

On macOS 11 or later, grant **Screen Recording** permission in **System Settings > Privacy & Security** when prompted. You may need to restart Screen-Pin after granting it. The v0.1.0 apps are signed ad hoc but are not notarized, so macOS may also require you to allow the app manually in Privacy & Security.

## Use Screen-Pin

| Action | Windows | macOS |
| --- | --- | --- |
| Capture a region | `Alt + Shift + S` | `⌥ + ⇧ + S` |
| Open Memory | `Alt + Shift + M` | `⌥ + ⇧ + M` |
| Restore interaction after Ghost Mode | `Alt + Shift + G` | `⌥ + ⇧ + G` |

1. Capture freezes the displays. Drag over a region to pin it, or press `Esc` or right-click to cancel. Arrow keys move the selection by 10 logical pixels; `Shift` + arrow keys move it by one. Press `Space` to set an anchor and `Enter` to confirm.
2. The crop stays above other windows. Drag it from the orange corner or an area without text. Select and copy recognized text, or hover over the crop to reveal **Copy text**, **Ghost**, and **Archive**.
3. **Ghost** lets clicks pass through the crop. Use the restore shortcut, open Memory, or use the tray/menu-bar menu to interact with it again.
4. **Archive**, `Esc`, or double-clicking an area without text saves the crop to Memory and closes its floating window. Search recognized text, copy content, or pin a saved crop again.

Closing the main window leaves Screen-Pin running in the Windows system tray or macOS menu bar. If another app has claimed a shortcut, Screen-Pin reports it on startup; the buttons and tray/menu-bar actions remain available.

## OCR

- **macOS:** Apple Vision recognizes text locally, requesting English and Spanish where the OS supports them. It preserves punctuation and text positions.
- **Windows:** Windows Media OCR reads the image in color and with local contrast adjustments. Tesseract supplements it for console output, paths, variables, and operators such as `C:\Users>`, `|`, `<=`, `!=`, `&&`, `foo_bar`, and `%PATH%`. Large captures are processed in overlapping regions to respect the [Windows OCR size limit](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr.ocrengine.maximagedimension).

The saved PNG keeps its original appearance. OCR can still miss or confuse symbols, especially in passwords and technical text; check the result before using it. Improvements to OCR apply to new captures rather than rewriting text in existing ones.

## Local data and privacy

Captures are stored as PNG files with a SQLite database in `%APPDATA%/ScreenPin` on Windows or `~/Library/Application Support/ScreenPin` on macOS. The Windows portable package uses its own `data` folder instead. OCR runs entirely on the device.

Captures classified as accounts, payment cards, or credentials expire after 30 minutes. Screen-Pin checks for expired captures at startup and every 60 seconds while running; cleanup removes the database record, attempts to delete the PNG, and releases an open crop and its in-memory copy. **Keep** prevents expiry, while **Release** starts a new 30-minute period.

Sensitive-content detection can miss information. Local files are not encrypted, and deletion is not a secure disk erase. The initial Memory list and search results show up to 50 captures. Existing databases are migrated automatically and their search index is rebuilt when needed.

## Develop and build

Requirements:

- **macOS 11+:** Node.js 24, pnpm, stable Rust, and Xcode Command Line Tools (`xcode-select --install`). OCR uses the native Apple Vision framework.
- **Windows:** Node.js 24, pnpm, stable Rust, CMake 3.31+, Visual Studio C++ Build Tools, and WebView2. CMake must be on `PATH` or set through `CMAKE`. The first native build downloads and compiles Tesseract and Leptonica; later builds reuse the cache.

~~~sh
pnpm install --frozen-lockfile
pnpm tauri dev
~~~

`pnpm dev` opens a web preview only. Capture, Memory, and global shortcuts require the desktop app; the preview disables those native actions.

Run the project checks before packaging:

~~~sh
pnpm check
pnpm test
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
pnpm build
~~~

Build a Windows installer with `pnpm tauri build --bundles nsis`, or a macOS app with `pnpm tauri build --bundles app` on a Mac. Use `pnpm tauri build --debug --no-bundle` for a native test build without an installer. The [macOS bundle workflow](.github/workflows/macos-bundle.yml) builds separate Apple Silicon and Intel ZIPs. Test Screen Recording permission, Retina and external displays, OCR, clipboard, and global shortcuts on a target Mac.

Windows x64 builds include the Microsoft C++ DLLs needed by Tesseract. Keep them beside the executable when moving an uninstalled build. Their provenance is recorded in [the runtime notice](src-tauri/resources/runtime/windows-x64/NOTICE.md); builds for another architecture need matching DLLs. In development builds, `SCREEN_PIN_DATA_DIR` selects a separate test data folder.

## Visual assets

[The source logo](static/favicon.svg) defines the favicon and Tauri icons; regenerate them with `pnpm tauri icon static/favicon.svg`. The interface uses the charcoal, paper, and orange palette in [DESIGN.md](DESIGN.md). The NSIS installer uses `src-tauri/nsis/sidebar.bmp`, `src-tauri/nsis/header.bmp`, and `src-tauri/icons/icon.ico`.

## Feedback and support

Have an idea to improve Screen-Pin, or encountered a bug?
- 💡 [Submit a suggestion or feedback](https://github.com/JasonS-V/Screen-Pin/issues/new?template=feedback.yml)
- 🐛 [Report a bug](https://github.com/JasonS-V/Screen-Pin/issues/new?template=bug_report.yml)
- 💬 [Join GitHub Discussions](https://github.com/JasonS-V/Screen-Pin/discussions)
