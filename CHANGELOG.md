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

### Changed
- Terminal steps now display `WT`/`CMD` badges and show default labels when no command is provided.
- Close-on-switch ignores steps marked to keep running.
- Titlebar and overlay now use theme-aware colors.
