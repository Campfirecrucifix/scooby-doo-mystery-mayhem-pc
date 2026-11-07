# Scooby-Doo! Mystery Mayhem — PC port (Rust / Bevy)

A fan-made PC port / recompilation of the PlayStation 2 game *Scooby-Doo! Mystery Mayhem*. The game's
engine and its game-logic modules were reverse-engineered and rewritten in Rust; rendering, audio and
input run on [Bevy](https://bevyengine.org/).

**No game assets are included.** This repository contains only code. Every game file — levels,
models, textures, sounds, videos, text, and the data images of the game-logic modules — is read
at run time from **your own disc image** of the original PS2 game. You must own the game legally.

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
| `LEVEL` (e.g. `EP1/L01`, `EP2L01A`) | level to start (default `LOADER` = boot like the disc: logo FMVs, title, main menu -> New Game -> Episode 1) |
| `--iso <path>` | disc image |
| `--aspect 4:3\|16:9\|21:9\|32:9\|auto\|W:H` | aspect ratio (Hor+ widescreen; default `auto`) |
| `--res WxH` | window (or headless target) size |
| `--rt` | enhanced post-processing (HDR, bloom, FXAA, sharpening; cargo feature `rt`) |
| `--sound` | enable audio (the game starts muted by default) |
| `--headless`, `--frames N`, `--shots DIR` | run without a window / for N frames / save screenshots |

Env: `GAME_WORLD_MERGE=0` turns off world material sharing (default on: level geometry pieces
with the same colour + texture share one material; pixel-identical on all levels, much faster).

Controls: keyboard, mouse and gamepads, rebindable through `settings.toml` (or the launcher) —
see [CONTROLS.md](CONTROLS.md).

## Launcher and settings

- Start the game with **`Play Scooby-Doo.bat`** (repository root). It opens the settings launcher
  (`target\release\launcher.exe`, build it with `cargo build --release --features launcher --bin launcher`;
  without it the .bat starts `game.exe --sound` directly). Home page: disc image (**Browse...** /
  **Change...**; PLAY stays disabled until a valid image is chosen), **PLAY** (starts the game with sound
  on), **Settings**, **Open log folder**, **Report a bug**, **Quit**. Settings: tabs Graphics / Audio /
  Controls / Remaster, **Restore defaults** (current tab), **Save**, **Back**.
  Mouse, keyboard (Tab / arrows / Enter = Play / Esc = Back, or Quit on the home page / Ctrl+Tab = next
  tab) or an Xbox pad (D-pad / stick, A = press / toggle / open list, LB / RB = tab, Start = Play,
  B = same as Esc).
- **Skip launcher next time** starts the game at once; hold **Shift** while starting the .bat (or run
  `launcher.exe --settings`) to get the launcher back.
- In the game: pause > **Options** > **PC SETTINGS (SELECT)** has the same settings (L1/R1 = tab,
  Triangle = back + save).
- Everything lives in **`recomp\settings.toml`** (the folder the .bat starts in; else next to the
  exe). An old `controls.toml` is migrated on first start (and left alone). A broken file is kept as
  `settings.toml.bad` and the defaults are used. Command-line flags and `GAME_*` env vars override it.
- Live: volumes, mute, controls, display mode, window size, VSync, FPS cap. **Restart needed**:
  anti-aliasing and the Remaster options (marked `(R)` in the game).
- Remaster tab: only active when a remaster build (`game_remaster.exe` + `texpack\`) sits next to
  `game.exe`; Play then starts it with the matching `GAME_REMASTER_LIGHT` / `GAME_TEXPACK` /
  `GAME_RL_LEVELS` (`launcher.exe --print-env` shows exactly what it would start).

## Status

Playable: Episode 1 from a disc boot (main menu -> New Game -> L01 -> L02 -> L03A -> L03B -> L04 -> episode end
screen -> Episode 2), with checkpoint saves/loads and the pause menu; the Episode 2 level chain. Run with `--sound`
for audio (muted by default). Other levels load but their
game-logic is partially ported. See `ROADMAP.md` and `ENGINE_PORT.md` for details.

## Third-party code

`vendor/na_mpeg2_decoder` is a patched copy of `na_mpeg2_decoder` 0.1.0
(MPL-2.0, https://github.com/xmoezzz/siglus_rs).

## Disclaimer

This is an unofficial fan project, not affiliated with or endorsed by Warner Bros., Hanna-Barbera,
THQ, or Artificial Mind & Movement. *Scooby-Doo* and all related characters and elements are
trademarks of and © Hanna-Barbera / Warner Bros. Entertainment Inc. No copyrighted game data is
distributed with this project.
