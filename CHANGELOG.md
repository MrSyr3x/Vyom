# Changelog

## [1.0.235] - 2026-02-13
### Fixed
- **CI**: Replaced Homebrew workflow with manual shell script to ensure correct authentication and file paths for external tap.

## [1.0.234] - 2026-02-13
### Fixed
- **CI**: Updated Homebrew workflow (renamed secret to `VYOMTOKEN`).

## [1.0.233] - 2026-02-13
### Fixed
- **Audio Engine**: Implemented robust WAV header parsing for MPD HTTP streams to fix intermittent audio speed/pitch issues.
- **Cleanup**: Resolved clippy lints (collapsible matches, range checks) for a stricter codebase.

### Added
- **CI**: Added GitHub Action for automated Homebrew formula updates.

## [1.0.232]
- Initial version with MPD and EQ support.
