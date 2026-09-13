# `rscat`

A terminal RSVP (Rapid Serial Visual Presentation) speed-reader: words are
streamed one at a time, anchored on their Optimal Recognition Point, so the
gaze can stay fixed while reading.

## Usage

```sh
rscat book.txt                 # read a file
cat book.txt | rscat           # or pipe from stdin
rscat -w 450 book.txt          # 450 words per minute
rscat -b --ghost 0.5 book.txt  # big pixel font, with a fading ghost
```

With Nix: `nix run .#rscat -- book.txt`.

## Options

| Option                    | Default | Meaning                                                                                 |
| ------------------------- | ------- | --------------------------------------------------------------------------------------- |
| `[FILE]`                  | stdin   | File to read. Required when stdin is a terminal.                                        |
| `-w`, `--wpm <N>`         | `300`   | Reading speed in words per minute (60–50000).                                           |
| `-c`, `--clause <MULT>`   | `1.5`   | Extra time for a word ending in `,` `;` `:`.                                            |
| `-s`, `--sentence <MULT>` | `2.0`   | Extra time for a word ending in `.` `?` `!`.                                            |
| `-l`, `--length <FACTOR>` | `0.04`  | Extra time proportional to `sqrt(word length)`. `0` disables.                           |
| `-r`, `--rarity <FACTOR>` | `0.3`   | Extra time for words that are rare *within this text*. `0` disables.                    |
| `-f`, `--font <FONT>`     | `normal`| Word size; see below.                                                                    |
| `-b`, `--big`             |         | Shorthand for `--font sextant`.                                                         |
| `--ghost <FRACTION>`      | `0`     | Overlay the previous word for this fraction of its display time. `0` disables.          |

`--ghost` requires a pixel font (anything but the default `normal`).

### Fonts

`normal` is plain one-line text. The others render the word as pixel art (via
`tui-big-text`); the number in parentheses is the height in terminal rows:

`full` (8), `half-height` (4), `half-width` (8), `quadrant` (4),
`third-height` (3), `sextant` (3), `quarter-height` (2), `octant` (2).

## Keys

| Key                       | Action                                                  |
| ------------------------- | ------------------------------------------------------- |
| `space`                   | Pause / resume.                                         |
| `←` / `→`                 | Previous / next word (only while paused).               |
| `+` / `=` / `-`           | Faster / slower by 10 wpm (works while playing too).    |
| `q` / `Esc` / `Ctrl-C`    | Quit.                                                   |

## Display and timing

The viewport is inline (it does not take over the screen): the word on top, and
one stats line showing position, speed and ETA, with progress drawn as its
background.

Each word is shown for the base `60/wpm`, scaled by:

- a punctuation pause (`--clause` / `--sentence`);
- a length term (`--length`);
- a rarity term (`--rarity`) based on how often the word occurs in this text, so
  recurring terms such as character names speed up once familiar.

Length and rarity are correlated, so they are combined with `max`, not added.
The displayed ETA is a lower bound: it ignores all of the above scaling.

With `--ghost`, the previous word lingers behind the current one, dimmed, for
that fraction of the *previous* word's display time.
