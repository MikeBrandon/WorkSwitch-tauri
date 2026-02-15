# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

## 1.0.1 - 2026-02-15

### Added
- Theme system with `dark`, `light`, `auto`, and `frosted` modes, plus a new Theme setting.
- Per-step “Leave process running” option and terminal app selection (Windows Terminal or Command Prompt).
- Close-on-exit option to terminate last-launched processes.
- Windows app discovery for Steam, Epic Games, and installed apps.
- `LastLaunch` state and command to track last-launched processes.
- Kill & Wipe workflow with a toolbar action, selectable options, and optional immediate mode.
- Desktop shortcut creation for Kill & Wipe with startup flags (`--kill-and-wipe`, `--kill-and-wipe-immediate`).
- Post-logout greeting message after Kill & Wipe completes.

### Changed
- Terminal steps now display `WT`/`CMD` badges and show default labels when no command is provided.
- Close-on-switch ignores steps marked to keep running.
- Titlebar and overlay now use theme-aware colors.
- Kill & Wipe runs Windows-first process cleanup, temp clearing, browser data wiping, DNS flush, and optional logout.
