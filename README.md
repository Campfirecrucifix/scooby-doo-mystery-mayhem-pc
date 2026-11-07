# Scooby-Doo! Mystery Mayhem — PC port (Rust / Bevy)

A fan-made PC port / recompilation of the PlayStation 2 game *Scooby-Doo! Mystery Mayhem*. The game's
engine and its game-logic modules were reverse-engineered and rewritten in Rust; rendering, audio and
input run on [Bevy](https://bevyengine.org/).

**No game assets are included.** This repository contains only code. Every game file — levels,
models, textures, sounds, videos, text, and the data images of the game-logic modules — is read
at run time from **your own disc image** of the original PS2 game. You must own the game legally.

## Download (Windows)

Get the latest build from [Releases](https://github.com/Campfirecrucifix/scooby-doo-mystery-mayhem-pc/releases):
unzip it, run **`Scooby-Doo Mystery Mayhem.exe`** (the launcher), click **Browse...** to pick your own
PS2 disc image, then **PLAY**. Settings (graphics, audio, controls) are in the launcher and in-game
under Options > PC SETTINGS. Logs and crash reports go to `%LOCALAPPDATA%\ScoobyMM\logs`.

Known issues in 1.0.0-rc.2: music may not play and short voice lines can be missing in some setups.

## Requirements

- A Rust toolchain (stable, edition 2024), `cargo`.
- A disc image (`.ISO`) of *Scooby-Doo! Mystery Mayhem* for PS2 (the port was developed against
  the SCEE disc, `Scooby Doo Mystery Mayhem.ISO`). If the disc does not hold a module the port
  expects, it stops with an error naming the missing module (modules are verified by SHA-1).

## Disc image location

Pass `--iso <path>`, or place `Scooby Doo Mystery Mayhem.ISO` in one of (relative to the current
directory): `../`, `./`, `../../`. Tests expect it at `../Scooby Doo Mystery Mayhem.ISO` relative to
`recomp/`.

## Build and run

```sh
cd recomp
cargo run --release --features game --bin game -- --iso "path/to/Scooby Doo Mystery Mayhem.ISO"
cargo run --release --features game --bin game -- EP1/L02          # start at a level
cargo test --release                                              # tests (need the ISO)
```

## Command line

| Flag | Meaning |
|---|---|
| `LEVEL` (e.g. `EP1/L01`, `EP2L01A`) | level to start (default `EP1/L01`) |
| `--iso <path>` | disc image |
| `--aspect 4:3\|16:9\|21:9\|32:9\|auto\|W:H` | aspect ratio (Hor+ widescreen; default `auto`) |
| `--res WxH` | window (or headless target) size |
| `--rt` | enhanced post-processing (HDR, bloom, FXAA, sharpening; cargo feature `rt`) |
| `--headless`, `--frames N`, `--shots DIR` | run without a window / for N frames / save screenshots |

Controls: keyboard, mouse and gamepads, rebindable through `controls.toml` (written on first run) —
see [CONTROLS.md](CONTROLS.md).

## Status

Playable: Episode 1 levels L01–L03A and the Episode 2 level chain. Other levels load but their
game-logic is partially ported. See `ROADMAP.md` and `ENGINE_PORT.md` for details.

## Third-party code

`vendor/na_mpeg2_decoder` is a patched copy of `na_mpeg2_decoder` 0.1.0
(MPL-2.0, https://github.com/xmoezzz/siglus_rs).

## Disclaimer

This is an unofficial fan project, not affiliated with or endorsed by Warner Bros., Hanna-Barbera,
THQ, or Artificial Mind & Movement. *Scooby-Doo* and all related characters and elements are
trademarks of and © Hanna-Barbera / Warner Bros. Entertainment Inc. No copyrighted game data is
distributed with this project.
