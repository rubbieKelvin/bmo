# BMO — Project description

**BMO** is a desktop Pomodoro-style focus timer for macOS (and other targets supported by [GPUI](https://github.com/zed-industries/zed)), written in Rust. The name is a nod to BMO from *Adventure Time*. The app presents a compact, dark-themed window with a live countdown, a visual session timeline, and simple transport controls.

This document summarizes how the repository is organized, how data flows through the app, and how the main pieces fit together. For a shorter feature list and screenshots, see [README.md](README.md).

---

## Goals and scope

- Run repeating **focus** and **break** segments in sequence (classic Pomodoro-style cadence by default).
- Give clear feedback: large monospace time display, segment icons (focus vs break), and a **timeline** that highlights the active segment and fills with progress.
- Let users manage **named presets** backed by SQLite: create presets from a template pattern, pick which preset is **active**, and remember that choice across launches.

Out of scope or not yet implemented in code (see README for aspirational items): system notifications, sound cues, per-segment editing in the UI, and cloud sync.

---

## Technology stack


| Area            | Choice                                                                                                            |
| --------------- | ----------------------------------------------------------------------------------------------------------------- |
| Language        | Rust (edition 2024)                                                                                               |
| UI              | [GPUI](https://crates.io/crates/gpui) `0.2.x` — GPU-oriented, immediate-mode style desktop UI                     |
| UI kit          | [gpui-component](https://crates.io/crates/gpui-component) — title bar, buttons, lists, inputs, theming hooks      |
| Persistence     | [SQLx](https://crates.io/crates/sqlx) with **SQLite**, async runtime `async-std`, migrations via `sqlx::migrate!` |
| Embedded assets | [rust-embed](https://crates.io/crates/rust-embed) — SVG icons and graphics compiled into the binary               |
| Errors          | `anyhow` at the application boundary (`main` window spawn)                                                        |


Release builds use **LTO** and **strip**; an extra `release-distro` profile favors maximum optimization for distribution.

---

## Repository layout

```
src/
  main.rs           # Application entry: GPUI app, window options, Root + TitleBar
  app/
    mod.rs          # BmoApp shell: screen routing, Database entity, preset sync
    timer.rs        # TimerScreen: title bar, timer + timeline, controls
    settings.rs     # SettingScreen: preset list, new preset input, navigation
  components/
    timer.rs        # Countdown Timer entity, tick/completed events, async tick loop
    timeline.rs     # Segment strip + progress for the current preset
  session.rs        # TimerPreset + Session + SessionKind (in-memory model)
  db.rs             # Database entity: SQLite pool, presets, active preset persistence
  events/
    navigation.rs   # Screen enum + NavigationEvent (optional TimerPreset payload)
  assets.rs         # RustEmbed AssetSource for GPUI
  constants.rs      # Shared timing constants (e.g. one minute in ms)
assets/             # SVGs (icons/, svg/) loaded via rust-embed
migrations/         # SQL migrations applied on DB connect
bmo.db              # Local SQLite file (created at runtime; not required in repo)
```

---

## Runtime architecture

### Application bootstrap (`main.rs`)

1. `Application::new().with_assets(Assets)` registers embedded SVGs.
2. `gpui_component::init` installs shared component theming/behavior.
3. A **single window** opens (~600×450 px, minimum ~500×450) wrapping the root view in `gpui_component::Root` with a custom title bar.
4. When all windows close, the app quits.

### Shell: `BmoApp` (`app/mod.rs`)

`BmoApp` is the top-level **router** between two screens:

- **Timer** — `TimerScreen`
- **Settings** — `SettingScreen`

It owns one shared `Entity<Database>` and wires **subscriptions**:

- **Timer → shell**: navigating to Settings when the user clicks the settings control.
- **Settings → shell**: navigating back to the Timer, optionally applying a selected preset; also refreshes preset metadata in the database layer.
- **Database observation**: when the active preset id or preset list changes (for example after reload from disk), the timer receives `set_preset` so the UI stays aligned with SQLite.

### Session model (`session.rs`)

- `**SessionKind`**: `WORK` (focus) or `BREAK`.
- `**Session**`: title (`SharedString`), `Duration`, and kind.
- `**TimerPreset**`: title, ordered list of sessions, optional `source_id` linking to a DB row when loaded from SQLite.

`TimerPreset::default()` encodes a **classic Pomodoro-style** sequence (alternating focus and short/long breaks). New database presets are seeded with this same template (see `Database::insert_preset_with_template_sessions` in `db.rs`).

### Timer behavior (`components/timer.rs`, `app/timer.rs`)

- The **Timer** entity runs an async loop on GPUI’s background executor (~200 ms cadence), decrements elapsed time, emits `**TimerTickEvent`** (progress fraction) and `**TimerCompletedEvent**` when a segment reaches zero.
- **TimerScreen** subscribes to ticks to update the timeline’s `current_progress`.
- On **completion** of a segment that is not the last in the preset, the screen advances `session_index`, updates the timeline’s active segment, and **starts** the next session automatically.
- **Controls**: Start (from idle), Pause/Play while running, Stop (resets running state). When the countdown is zero, the footer shows the idle “Start” affordance again.
- When **all** segments in a preset have finished, the completion handler returns without chaining another session (end of cycle); the user can start again from the idle state.

### Timeline (`components/timeline.rs`)

For each preset segment, the UI builds a colored block. Colors are **deterministic** from a hash of segment title and index so the strip stays stable across runs. The active segment grows and shows a fill proportional to `current_progress`; inactive segments are compact outlined pills.

### Settings (`app/settings.rs`)

- Lists **non-deleted** presets from the database with a “current” indicator when the row matches `active_preset_id`.
- **Confirm** on a list row (per `ListEvent::Confirm`) sets that preset active in the DB, persists `active_preset_id`, emits navigation to the Timer with the resolved `TimerPreset`.
- **New preset**: text field + Add (or Enter) calls `Database::create_preset`, which inserts a preset row and template sessions in a transaction, then refreshes the list.

Closing settings via the **X** returns to the timer **without** changing the active preset payload (preset remains whatever was already selected unless the user confirmed a row).

---

## Data layer

### SQLite file and migrations

- On init, the app connects to `**bmo.db`** in the process working directory (`Database::init`), enables WAL, runs embedded migrations from `./migrations`, and loads state.
- `**001_create_tables.sql**`: `presets` (id, name, description, timestamps, soft-delete flag) and `session` (FK to preset, name, duration in seconds, optional color, `type` ∈ `focus` | `break`).
- `**002_app_settings.sql**`: singleton row `app_settings` (`id = 1`) storing `**active_preset_id**` (nullable), with `INSERT OR IGNORE` for the default row.

### In-memory `Database` entity (`db.rs`)

- Holds `SqlitePool` (once connected), a `Vec<Preset>` (each with nested sessions), and `active_preset_id`.
- `**reload_from_disk**`: loads presets + validates stored active id against existing presets; clears invalid foreign keys and schedules persistence.
- `**update_preset_list**`: refreshes preset rows while preserving a valid active id when possible.
- `**set_active_preset_id` / `schedule_persist_active_preset**`: update memory and asynchronously write `app_settings`.
- `**Preset::to_timer_preset**`: maps DB enums and durations into `session::TimerPreset` for the timer UI (empty session list yields `None`).

SQLx’s compile-time checked queries are used for preset loading; the project includes generated `.sqlx` JSON for offline `cargo sqlx prepare` workflows.

---

## Assets

SVG files under `assets/` are embedded at compile time. The timer screen references paths like `svg/eye.svg` and `icons/pause.svg` through GPUI’s asset pipeline (`Assets` implements `AssetSource`).

---

## Building and running

```bash
cargo run          # debug
cargo build --release
```

**Note:** `sqlx` offline macros expect prepared query metadata; if you change queries, you may need to run `cargo sqlx prepare` (or use the `SQLX_OFFLINE` workflow) according to your environment.

---

## License

The project is licensed under the **GNU GPLv3** (see [LICENSE](LICENSE)).

---

## Relationship to README

[README.md](README.md) is the user-facing quick start. Parts of it still describe early roadmap items (for example “settings needs implementation”) that the codebase has since grown beyond: **settings, presets, and persistent active preset selection are implemented** as described above. Notifications, sounds, and richer preset editing remain natural next steps.