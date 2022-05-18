# Changes

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- Add "copy song details to clipboard" [!53]

### Changed

- Add whole folder at once [!30]
- Provide user feedback during loading [!54]
- Translation updates:
  - Swedish
  - Ukrainian
  - Occitan
  - Basque
  - Turkish
  - Polish
  - Russian
  - Persian

### Fixed

- Ensure that the remaining time sign is consistent in RTL locales [#118]
- Improve the UI consistency when clearing the playlist
- Apply darkening to the playlist view unconditionally [#128]

### Removed

## [0.6.1] - 2022-05-09

### Added

- Add translations for:
  - Dutch
  - Indonesian
  - Occitan
  - Spanish

### Changed

- Make the cover art image slightly bigger
- Update translations for:
  - Ukrainian
  - Swedish
  - Polish
  - Persian

### Fixed

- Improve the appearance of the initial landing page [#106]
- Stabilise the width of the playlist panel [#110]
- Rely on gdk-pixbuf instead of lofty for image format detection [#111]
- Multiple papercut style fixes [#105, #108]
- Fix selector for playlist background when folded [#107]

## [0.6.0] - 2022-05-06

### Added

- Use a selection mode for the playlist management [#81]
- Allow disabling UI recoloring
- Expose more song state through MPRIS
- Add better error messages in the UI
- Support RTL text direction in the waveform widget
- Add translations for:
  - Russian
  - Turkish
  - Brazilian Portuguese
  - Hebrew
  - Galician
  - Swedish
  - Basque
  - Persian
  - Ukrainian
  - Polish

### Changed

- Reset to the initial state when clearing the playlist [#101]
- Tone down the recoloring to improve legibility of text and controls [#97]
- Recolor only the main window [#104]
- Use better icon for playlist toggle button [#102]
- Use the cover art palette for the waveform view accent color [#61]
- Set the minimum and maximum width for the playlist view [#93]

### Fixed

- Fix the background recoloring gradient to use the whole cover art palette
- Reset the waveform generator and view on failure [#57]
- Darken the playlist background when unfolded [#85]
- Improve the tooltips for playback controls [#69]
- Fix extra spacing in the playlist view [#98]
- Fix elapsed song time in RTL locales [#95]
- Remove missing shortcuts from the shortcuts dialog [#96]

## [0.5.0] - 2022-04-29

### Added

- Improve fallback paths for song metadata

### Changed

- Move the playlist side panel to the left of the playback controls [#50]
- Make sure that the remove button in the playlist rows is accessible
  without hovering

### Fixed

- Align the waveform to the pixel grid [#76]

### Removed

- Drop the seek buttons, and rely on the waveform control [#59]

## [0.4.3] - 2022-04-26

### Added

- Add scrolling support to the volume control [#50]

### Fixed

- Fix behaviour of the waveform with short songs and avoid overdrawing [#68]
- Make the waveform control more legible [#52]
- Reset the shuffle state when clearing the playlist [#60]
- Keep the playlist visibility, folded or unfolded, in sync with the
  toggle button that controls it [#55]
- Fix a crash when manually advancing through the playlist [#54]

## [0.4.2] - 2022-04-22

### Fixed

- Fix the fallback cover art in the playlist

## [0.4.1] - 2022-04-22

### Fixed

- Don't skip songs without a cover art [#46]
- Clean up unnecessary overrides [Bilal Elmoussaoui, !32]

## [0.4.0] - 2022-04-22

### Added

- Add waveform display and quick navigation
- Allow queueing folders recursively
- Add initial status page at startup [#27]
- Add remove button to the playlist [#40]
- Show cover art in the playlist

### Changed

- Allow adding folders via drag and drop [#17]
- Allow shuffling only when the playlist contains more than one song [#15]
- Style the popover using a similar background as the main window [#12]
- Small style tweaks for the recoloring
- Reduce the height of the full window to fit in 768p displays [#16]
- Make the layout more mobile friendly [#28]
- Ship our own icon assets

### Fixed

- Fix an assertion failure when reaching the end of a shuffled playlist
- Scroll playlist to the current song [#29]
- Update dependency on lofty for m4a support [#22]
- Add divider above scrolling playlist [#26]
- Fix styling of the missing cover fallback image [#36]
- Set the album art metadata for MPRIS [#13]

## [0.3.0] - 2022-04-15

### Added

- Allow shuffling the contents of the playlist
- Support dropping multiple files
- Volume control
- Allow Amberol to be set as the default application for Music in
  the GNOME Settings

### Changed

- Miscellaneous cleanups [Christopher Davis, !10]
- Use idiomatic Rust as suggested by Clippy
- Improve handling the end of playlist state
- Skip songs that cannot be queried for metadata
- Switch to a portrait layout

### Fixed

- Stop playback when clearing the playlist
- Immediately play the song selected from the playlist
- Use the appropriate color format for the texture data [#7]
- Use the proper fallback asset for albums with no cover
- Start playing when opening a file [#8]

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
