# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/).

## [1.0.1] - 2026-08-05

### Changed

- Improved Windows taskbar and title-bar icon clarity with DPI-aware icon selection that updates when moving the settings window between displays with different scaling.
- Updated Windows icon generation to preserve the prepared size-specific PNG frames in the multi-resolution ICO, including a 96 px frame for high-DPI displays.

### Fixed

- Rest overlay danger buttons now use the configured button text color instead of always displaying black text.

## [1.0.0] - 2026-07-30

### Added

- Configurable work and rest intervals based on the 20-20-20 rule.
- Fullscreen rest overlays for selected monitors.
- System tray controls for managing reminders and opening settings.
- Options to skip a reminder or start a rest early.
- Idle detection and fullscreen-application suppression.
- Application whitelist support.
- Optional automatic startup with Windows.
- Local settings and diagnostic logging.
- Portable mode with settings and logs stored beside the application.
- Windows NSIS setup executable and MSI installer packages.
- SHA-256 checksum generation for portable and installer packages.
