# Changelog

## [1.0.2485] - 2026-03-10
### Added
- **Audio Engine**: Implemented `flush_signal` for zero-latency seeking and pausing.
- **UI**: Added `ArtStyle` cycling with support for ASCII, Braille, and high-fidelity Image rendering.
- **Performance**: Optimized artwork caching and memory footprint.
- **Audit**: Completed full production-readiness audit with 0 panics/unwraps in runtime paths.

## [1.0.248] - 2026-03-09
### Changed
- **UI**: Enhanced popup stability and image rendering overlays.
- **Configuration**: Improved home directory detection and fallback logic.

## [1.0.247] - 2026-03-05
### Fixed
- **Audio Engine**: Resolved intermittent "buzzing" in HTTP streams.
- **UI**: Polished quality badges and track metadata display.

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
