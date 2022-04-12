# Changes

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

### Changed

- Miscellaneous cleanups [Christopher Davis, !10]
- Use idiomatic Rust as suggested by Clippy
- Improve handling the end of playlist state
- Skip songs that cannot be queried for metadata

### Fixed

- Stop playback when clearing the playlist
- Immediately play the song selected from the playlist
- Use the appropriate color format for the texture data [#7]
- Use the proper fallback asset for albums with no cover

### Removed

## [0.2.1] - 2022-04-11

### Changed

- Style tweaks [Jakub Steiner, !9]

### Fixed

- Handle songs with unset fields without panicking

## [0.2.0] - 2022-04-11

### Added

- Inhibit system suspend when playing

### Changed

- Tweak the behaviour of the window when toggling the playlist
- Improve the style of the window [Alexander Mikhaylenko, !7]
  - Deal with margins and padding
  - Style the playlist list view
  - Style the drag overlay

## [0.1.0] - 2022-04-08

Initial alpha release for Amberol

### Added

- Basic playback
- Playlist control:
  - Add single file
  - Add folder
  - Drag and drop
- Support opening files from the CLI
- Recolor the UI using the cover art palette
