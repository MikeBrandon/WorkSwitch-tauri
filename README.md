# WorkSwitch

A desktop app for managing profiles of apps and steps that launch together. Switch between work setups, gaming rigs, or dev environments with one click.
[Buy Me A Ko-Fi] (https://ko-fi.com/mikebrandon)

Built with [Tauri v2](https://v2.tauri.app/) (Rust backend + vanilla JS frontend).

## Features

- **Profile Management** - Create profiles with multiple launch steps (apps, terminal commands, folders, URLs)
- **One-Click Launch** - Launch all steps in a profile sequentially with configurable delays
- **Import/Export** - Share profiles as JSON files
- **Hotkey Launch** - Assign keyboard shortcuts to launch profiles instantly
- **Profile Scheduling** - Auto-launch profiles at specific times and days of the week
- **Profile Tags** - Organize profiles with tags and filter the sidebar
- **Startup Apps** - Configure apps that launch automatically when WorkSwitch starts
- **Launch with Windows** - Optional auto-start via Windows registry
- **Running Processes** - Monitor and kill tracked processes across all profiles
- **Launch History** - View when profiles were launched with success/failure tracking
- **Close-on-Switch** - Offers to close the previous profile's apps when switching
- **System Tray** - Minimize to tray, quick-launch profiles from the tray menu
- **App Discovery** - Scan and pick from installed Steam, Epic Games, and Windows apps
- **Custom Titlebar** - Dark themed window with integrated controls

## Download

Get the latest installer from [Releases](https://github.com/MikeBrandon/WorkSwitch-tauri/releases).

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Tauri CLI](https://v2.tauri.app/start/create-project/) (`cargo install tauri-cli`)

### Run in dev mode

```bash
cargo tauri dev
```

### Build for production

```bash
cargo tauri build
```

The installer will be at `src-tauri/target/release/bundle/nsis/WorkSwitch_1.0.0_x64-setup.exe`.

## Project Structure

```
frontend/               # Vanilla JS frontend
  index.html
  src/
    main.js             # App init, event wiring, hotkeys
    config.js           # Config load/save, data constructors
    profiles.js         # Profile list, tags, import/export
    steps.js            # Step list rendering and reordering
    launcher.js         # Launch orchestration and progress
    dialogs.js          # Modal dialogs (editors, settings, history)
    startup.js          # Startup apps panel
    processes.js        # Running processes panel
    styles.css          # All styles (dark theme)

src-tauri/              # Rust backend
  src/
    main.rs             # Entry point
    lib.rs              # Tauri builder setup
    config.rs           # Config structs and file I/O
    commands.rs         # Tauri command handlers
    launcher.rs         # Step launch logic
    process.rs          # Process detection and killing
    scheduler.rs        # Background schedule checker
    discovery.rs        # App scanning (Steam, Epic, Windows)
    tray.rs             # System tray icon and menu
```

## License

MIT
