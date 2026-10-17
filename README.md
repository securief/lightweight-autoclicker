# ⚡ Lightweight Autoclicker

<p align="center">
  <b>Ultra-lightweight, native cross-platform desktop autoclicker written in pure Rust.</b><br>
  <i>No Electron. No Tauri. No WebView. Just pure native performance.</i>
</p>

<p align="center">
  <img alt="Language" src="https://img.shields.io/badge/Language-Rust-orange?logo=rust">
  <img alt="Platform" src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blue">
  <img alt="RAM Usage" src="https://img.shields.io/badge/RAM-~2--3%20MB-brightgreen">
  <img alt="Idle CPU" src="https://img.shields.io/badge/Idle%20CPU-0.0%25-success">
</p>

---

## 📖 Overview

**Lightweight Autoclicker** is a fast, compact, and resource-friendly desktop autoclicker built with Rust and `eframe` (`egui`). Unlike modern desktop utilities that bundle an entire web browser engine, this application compiles into a single self-contained native binary (~1.65 MB, no installer, no packer) with **~2–3 MB RAM usage** and **0.0% CPU usage when idle**.

---

## ✨ Key Features

* **Extreme Resource Efficiency:**
  * **0.0% Idle CPU:** The background worker thread sleeps completely on an OS condition variable (`Condvar::wait`) while on standby — zero busy-waiting or polling loops. Hover/transition animations are disabled (`animation_time = 0`), so the window only repaints when something really changes.
  * **~2–3 MB RAM Footprint** (Task Manager "Memory", i.e. private working set): the OS system font is **memory-mapped read-only** — `CreateFileMappingW` + `MapViewOfFile` on Windows, `mmap(PROT_READ, MAP_PRIVATE)` on Unix — so its pages are shared with the OS file cache and stay trimmable instead of sitting in the private heap. Combined with a zero-dependency `key=value` configuration parser (no `serde`/`toml` bloat), 64 KB background thread stacks, and automatic working-set trimming (`K32EmptyWorkingSet` on Windows / `malloc_trim` on Linux) right after startup and every time the window loses focus. The unpacked build also keeps its code pages file-backed, which is one reason the shipped (no-packer) executable uses less RAM than the older UPX-packed build — that one unpacked its whole image into private memory at startup.
* **Tiny Standalone Binary:** No runtime, no installer, no webview, **no packer** — one plain executable (~1.65 MB).
* **No Input-Hooking APIs:** the coordinate picker reads the cursor with `GetCursorPos` and a single `GetAsyncKeyState(VK_LBUTTON)` call on Windows — no keyboard polling, no hooks, no raw-input capture.
* **Sub-Millisecond Precision:** Powered by hybrid OS sleep + spin-loop timing (`spin_sleep`) for accurate high-speed clicking with minimal CPU overhead.
* **Flexible Click Configuration:**
  * **Interval:** Configurable in hours (`h`), minutes (`m`), seconds (`s`), and milliseconds (`ms`).
  * **Mouse Options:** Left, Right, or Middle mouse button; Single or Double click.
  * **Repeat Modes:** Infinite (until stopped) or fixed count (N times) with automatic stop.
* **Target Position & Screen Coordinate Picker:**
  * Click at the current cursor position or lock to custom `(X, Y)` screen coordinates.
  * Includes an interactive **Pick** tool that captures screen coordinates on your next left click and immediately terminates the picker thread when done.
* **Customizable Global Hotkey:**
  * Unified `START / STOP` toggle button with a live keybind recorder (default: `F6`).
  * Works reliably even when the application window is unfocused or running in the background.
* **Quality-of-Life Features:**
  * **Single-Instance with Self-Restore:** prevents duplicate windows/sessions using clean OS-level file locking (`share_mode(0)` on Windows, `flock` on Unix). Launching the app a second time makes the window that is already running **un-minimize and take focus by itself** — through a temporary signal file and egui's official `ViewportCommand`s, i.e. without `FindWindowW` / `SetForegroundWindow`, so Windows Smart App Control heuristics stay quiet.
  * **Always on Top** (`OnTop`) & instant **Dark / Light Theme** toggle.
  * **AutoSave** & automatic restoration of the last window screen position (centers on screen on first launch).
  * **Bilingual UI:** Instant toggle between **English** and **Indonesian**.

---

## 🛠️ Project Architecture

```text
├── assets/
│   ├── app_icon.ico      # Windows .exe icon (multi-size 16..256)
│   ├── app_icon.rgba     # Raw 32x32 RGBA window/taskbar icon (4 KB, include_bytes!)
│   ├── app_icon.png      # Design preview
│   └── app_icon.rc       # Legacy resource script (reference only; build.rs embeds icon
│                         # + VersionInfo itself through winresource)
├── src/
│   ├── main.rs           # Entry point: single-instance guard, threads, viewport init
│   ├── config.rs         # Zero-dependency key=value settings persistence
│   ├── state.rs          # Lock-free Atomic shared state & Condvar sync
│   ├── worker.rs         # Background clicker thread (enigo + spin_sleep)
│   ├── hotkeys.rs        # OS-level global hotkey listener & dynamic rebinding
│   ├── ui.rs             # Reactive egui UI, pick-location trigger & show-signal watcher
│   ├── texts.rs          # Static zero-allocation EN/ID localization table
│   ├── memory.rs         # Read-only file mapping (system font) + working-set trimmer
│   ├── picker.rs         # Pick Location input: GetCursorPos+GetAsyncKeyState (Win) / device_query (Unix)
│   ├── instance.rs       # OS file-lock single-instance guard + show signal + tests
│   └── icon.rs           # Compile-time embedded RGBA icon loader
├── .cargo/config.toml    # Windows MSVC linker flags (/OPT:REF, /OPT:ICF)
├── build.rs              # Embeds Windows PE VersionInfo & icon resource (winresource)
├── release.ps1           # Release build wrapper (no packer) + binary size report
├── Cargo.toml            # Dependencies & anti-false-positive release profile
└── Cargo.lock            # Pinned dependency versions (binary crate → commit it)
```

---

## 🚀 Getting Started

### Prerequisites

1. **Rust Toolchain:** Install stable Rust via [rustup.rs](https://rustup.rs/).
2. **System Libraries (Linux only):**
   ```bash
   # Debian / Ubuntu
   sudo apt install libxdo-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
   ```
   *(Windows and macOS require no extra runtime dependencies.)*
3. **System Font (Linux only):** no font is bundled — the app memory-maps the first available file from a list of common paths (DejaVu, Liberation, Noto, Ubuntu, FreeSans). On minimal systems install one, e.g. `sudo apt install fonts-dejavu-core`, otherwise the window opens without glyphs.

### Build & Run

To test the true memory and binary size optimizations, always build in **Release** mode:

```bash
cargo build --release
```

The standalone executable will be generated at:
* **Windows:** `target\release\autoclicker.exe`
* **Linux / macOS:** `target/release/autoclicker`

### Release build script (no packer)

```powershell
powershell -ExecutionPolicy Bypass -File release.ps1
```

`release.ps1` is a thin wrapper around `cargo build --release` that reports the final size.

### Antivirus & packaging

**UPX is deliberately not used anywhere in this project.** Executable packing is the single strongest false-positive trigger for machine-learning scanners on unsigned binaries: the previous UPX-packed build (~0.9 MB) was flagged on VirusTotal as `Trojan:Win32/Wacatac.B!ml` (Microsoft) and `Generic ML PUA` (Sophos). The packed build has been dropped; the only build produced now is the plain, size-optimized one (~1.65 MB), which also keeps the working set lower because its code pages stay file-backed instead of being unpacked into private memory.

That is also why the release profile compresses via codegen instead of a packer (`opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`) and why the coordinate picker avoids keylogger-style APIs (see *Key Features*): no packer, no input hooks, no keyboard polling.

> Please report any remaining false positive to the vendor (Microsoft's false-positive submission portal, etc.) rather than re-packing the binary — code signing is the durable fix for an unsigned executable.

### Running Tests

```bash
cargo test                              # unit tests (see below)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The test suite covers the interruptible sleep logic, hotkey name mapping, the single-instance file lock plus the show-signal hand-off (second launch → first window restores), read-only file mapping (content identity, empty file, missing path, real system font headers), and the Windows picker FFI (`GetCursorPos`).

---

## ⚙️ Configuration

Settings are loaded from `autoclicker_settings.toml`, relative to the process working directory. The extension is kept for backward compatibility; the format is a plain, dependency-free `key=value` list:

```ini
hours=0
minutes=0
seconds=3
milliseconds=0
repeat_infinite=true
repeat_times=1
mouse_button=0
click_type=0
use_custom_pos=false
pos_x=0
pos_y=0
lang=0
hotkey=F6
always_on_top=false
dark_mode=true
autosave=true
window_x=812.5
window_y=344.0
```

* `mouse_button`: `0` Left, `1` Right, `2` Middle. `click_type`: `0` Single, `1` Double.
* `lang`: `0` English, `1` Indonesian.
* `window_x` / `window_y` are appended automatically once the window position has been stored (the app remembers where you left it). If they are absent, the window opens centered on screen.
* Old TOML-style values such as `hotkey = "F6"` are still read correctly.
* Unknown keys and malformed lines are ignored (defaults are used), so a damaged file never blocks startup.
* With **AutoSave** disabled, only the window position is written on exit — the other settings in memory are not saved back over the file.

---

## 🖥️ Platform Notes

* **Windows:** If you want to use the autoclicker inside a game or application running as **Administrator**, you must also run `autoclicker.exe` as **Administrator** (due to Windows UIPI — *User Interface Privilege Isolation*). The `.exe` icon and VersionInfo are embedded at build time via `winresource`; if the Windows SDK (`rc.exe`) is missing, the build still succeeds with a warning and only the icon falls back to the default one.
* **macOS:** On first launch, grant **Accessibility** permission in *System Settings → Privacy & Security → Accessibility* so the OS allows simulated mouse events.
* **Linux:** The window supports both X11 and Wayland (`eframe` features `x11` + `wayland`), but input simulation uses `enigo`'s **X11 (`xdo`) backend**. Clicks and hotkeys therefore work on X11 and on XWayland-hosted apps, while *native* Wayland apps may not receive synthesized input. Make sure your user has the appropriate input permissions on Wayland compositors.
