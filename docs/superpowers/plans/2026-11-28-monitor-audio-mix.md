# Monitor Source and Per-App Audio Mix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Telai stream a whole monitor, with the sound of every app that plays on the PC mixed together, except the apps the user leaves out (Discord by default), each with its own live level and volume.

**Architecture:** Video gains a second capture target (a monitor, through `windows-capture`'s `Monitor`). In monitor mode, audio comes from one WASAPI include-loopback capture per audible app (process tree root), summed by a mixer thread that publishes 48 kHz stereo `f32` at real-time pace. The live monitor loop re-reads the audio sessions once per second, syncs the set of captures with the user's saved choices, and reports per-app levels in the live snapshot. Window mode is unchanged.

**Tech Stack:** Rust 2024 (`wasapi` 0.27, `windows-capture` 2.0.1, `windows` 0.62, Tauri 2), React 19 + TypeScript, Vitest.

**Spec:** none written; the user skipped the design questions. The requirements agreed in chat are under Global Constraints.

## Global Constraints

- Window mode keeps capturing only the window's process tree, as today.
- Monitor mode captures every app that has an active audio session, except excluded ones. Exclusion and volume are per app, keyed by the lowercase executable stem (`discord`, `vlc`, `brave`).
- With no saved preferences, `discord`, `discordptb` and `discordcanary` are excluded. Everything else is included at 100%.
- Volume range 0–100% (gain 0.0–1.0). It changes only what is streamed, never what the user hears.
- Choices persist across restarts in `audio-mix.json` under the Tauri app config dir.
- Telai's own process and session process id 0 are never listed.
- Wire format to the player is unchanged: interleaved stereo `f32` LE at 48 kHz, header type 2.
- No new dependencies.
- No local Rust toolchain: Rust is verified by CI on push to `main` (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` with `clippy::pedantic`, `cargo test --workspace -- --include-ignored` on `windows-latest`). Commit and push straight to `main`; no branches, no PRs, no attribution trailers.
- Frontend checks: `pnpm format:check`, `pnpm lint`, `pnpm typecheck`, `pnpm test`.
- User-facing copy exists in `english.ts` and `portuguese.ts`; add every new string to both.

## Review Focus

1. An app that starts playing after the stream started should be heard within about a second, without restarting the stream. Test in Task 4 (`sync` adds a source) and Task 3 (a new source primes and then mixes).
2. An app that quits mid-stream should drop out silently and leave the others playing. Test in Task 3 (`remove` stops its contribution) and Task 4 (`sync` without the id closes it).
3. With nothing playing, the stream must keep sending silence at real-time pace, so the player does not underrun and re-buffer. Test in Task 3 (`mix` with no sources yields zeros of the requested length).
4. Two loud apps together must not wrap or distort past full scale. Test in Task 3 (sum clamps to [-1, 1]).
5. A missing or corrupt `audio-mix.json` must fall back to the defaults (Discord excluded), not fail the stream. Test in Task 5.

---

### Task 1: Monitor as a capture target

**Files:**

- Create: `crates/capture/src/monitor.rs`
- Modify: `crates/capture/src/video.rs` (`VideoCapture::start`), `crates/capture/src/thumbnail.rs` (add monitor still), `crates/capture/src/lib.rs` (exports)
- Test: `crates/capture/tests/live_capture.rs`

**Interfaces:**

- Produces:
  - `pub enum CaptureTarget { Window(isize), Monitor(isize) }` in `video.rs` (handle = `HWND` or `HMONITOR` as integer), exported from `lib.rs`.
  - `VideoCapture::start(target: CaptureTarget, sink: impl VideoSink) -> Result<Self, Error>` (replaces the `handle: isize` parameter). For `Monitor`, build `Settings` with `Monitor::from_raw_hmonitor`; the window existence check applies only to `Window`. `buffer_without_title_bar` already returns the full frame for monitors.
  - `pub struct MonitorInfo { pub handle: isize, pub number: usize, pub name: String, pub width: u32, pub height: u32, pub primary: bool, pub thumbnail_jpeg: Option<Vec<u8>> }` and `pub fn list_monitors() -> Result<Vec<MonitorInfo>, Error>` in `monitor.rs`; `number` is 1-based in enumeration order; `name` from `Monitor::name()`, empty string on error.
  - `pub(crate) fn monitor_thumbnail(hmonitor: isize) -> Option<Vec<u8>>` in `thumbnail.rs`: `GetMonitorInfoW` rect, `BitBlt` from the screen DC into the existing DIB path, then the existing `downscale` and `encode_jpeg`.

- [ ] **Step 1: Write the failing test** in `live_capture.rs`:

```rust
#[test]
#[ignore = "needs a desktop session"]
fn lists_and_captures_a_monitor() {
    let monitors = telai_capture::list_monitors().expect("monitors are listed");
    let first = monitors.first().expect("at least one monitor");
    assert_eq!(first.number, 1);
    assert!(first.width > 0 && first.height > 0);
    assert!(monitors.iter().filter(|m| m.primary).count() == 1);

    let (width, height, _) = capture_first_frame(CaptureTarget::Monitor(first.handle));
    assert_eq!((width, height), (first.width, first.height));
}
```

Change `capture_first_frame` to take a `CaptureTarget`, and pass `CaptureTarget::Window(window.handle)` in the existing test.

- [ ] **Step 2: Implement** `CaptureTarget`, the `start` change, `monitor.rs` and `monitor_thumbnail`. Add `Win32_Graphics_Gdi` calls `BitBlt`, `GetMonitorInfoW` (feature already enabled). Update the one caller in `src-tauri/src/live.rs` to `CaptureTarget::Window(window.handle)` so the workspace still builds.

- [ ] **Step 3: Commit and push; verify CI**

```bash
git add crates/capture src-tauri/src/live.rs docs/superpowers/plans
git commit -m "feat(capture): capture a whole monitor"
git push origin main
```

Expected: CI run on the commit is green, and the `Windows app` job log shows `lists_and_captures_a_monitor ... ok`.

---

### Task 2: Listing the apps that play sound

**Files:**

- Modify: `crates/capture/src/sessions.rs`, `crates/capture/src/process.rs`, `crates/capture/src/lib.rs`

**Interfaces:**

- Consumes: `ProcessTree::snapshot`, `image_path`.
- Produces:
  - `pub fn parent(&self, process_id: u32) -> Option<u32>` on `ProcessTree` (None for 0, self-parent or unknown).
  - `pub struct AudioApp { pub process_id: u32, pub executable: PathBuf, pub name: String }`: `process_id` is the app's root, `name` the lowercase executable stem.
  - `pub fn audio_apps() -> Result<Vec<AudioApp>, Error>`: every session in state `Active` on any render device, peak ignored; pid 0 and `std::process::id()` skipped; grouped by root; sorted by `name`, then `process_id`.
  - `fn app_root(process_id: u32, tree: &ProcessTree, executable_of: impl Fn(u32) -> Option<PathBuf>) -> u32`: walks up while the parent's executable equals the session process's executable (compare case-insensitively).

- [ ] **Step 1: Write the failing tests** in `sessions.rs`:

```rust
#[test]
fn a_child_with_the_same_executable_belongs_to_its_root() {
    // 30 (Discord voice) -> 20 (Discord) -> 10 (explorer)
    let tree = ProcessTree::from_links(&[(30, 20), (20, 10), (10, 1)]);
    let exe = |id| Some(PathBuf::from(match id { 30 | 20 => r"C:\D\Discord.exe", _ => r"C:\W\explorer.exe" }));
    assert_eq!(app_root(30, &tree, exe), 20);
}

#[test]
fn a_process_whose_parent_runs_another_executable_is_its_own_root() {
    let tree = ProcessTree::from_links(&[(30, 20)]);
    let exe = |id| Some(PathBuf::from(if id == 30 { "vlc.exe" } else { "explorer.exe" }));
    assert_eq!(app_root(30, &tree, exe), 30);
}
```

Add `#[cfg(test)] pub(crate) fn from_links(links: &[(u32, u32)]) -> ProcessTree` (the `tree` helper in `process.rs` tests moves here).

- [ ] **Step 2: Implement** `parent`, `app_root`, `AudioApp`, `audio_apps`. Keep `audible_processes` as is.

- [ ] **Step 3: Commit and push; verify CI green**

```bash
git add crates/capture
git commit -m "feat(capture): list the apps that play sound"
git push origin main
```

---

### Task 3: Mixer core

**Files:**

- Create: `crates/capture/src/mixer.rs`

**Interfaces:**

- Produces (crate-private, used by Task 4):
  - `pub(crate) const TARGET_FRAMES: usize = 1_920;` (40 ms) and `MAX_FRAMES: usize = 4_800;` (100 ms)
  - `pub(crate) struct Mixer` with:
    - `fn push(&mut self, id: u32, samples: &[f32])`: appends interleaved stereo; creates the source (gain 1.0) on first push; when more than `MAX_FRAMES` are buffered, drops the oldest down to `TARGET_FRAMES`.
    - `fn set_gain(&mut self, id: u32, gain: f32)`: clamps to 0.0–1.0; creates the source if missing.
    - `fn remove(&mut self, id: u32)`
    - `fn mix(&mut self, frames: usize, out: &mut Vec<f32>)`: replaces `out` with `frames * 2` samples: the sum of `gain * sample` over primed sources, clamped to [-1, 1]. A source is primed once it holds `TARGET_FRAMES`; on underrun it contributes what it has, pads with silence, and needs priming again. Tracks each source's peak after gain.
    - `fn take_peaks(&mut self) -> Vec<(u32, f32)>`: per-source peak since the last call, then resets; sorted by id.

- [ ] **Step 1: Write the failing tests** (`#[cfg(test)] mod tests` in `mixer.rs`):

```rust
fn frames(value: f32, count: usize) -> Vec<f32> { vec![value; count * 2] }

#[test]
fn silence_flows_with_no_sources() {
    let mut mixer = Mixer::default();
    let mut out = vec![9.0];
    mixer.mix(480, &mut out);
    assert_eq!(out, vec![0.0; 960]);
}

#[test]
fn primed_sources_are_summed_with_their_gain() {
    let mut mixer = Mixer::default();
    mixer.push(1, &frames(0.25, TARGET_FRAMES));
    mixer.push(2, &frames(0.5, TARGET_FRAMES));
    mixer.set_gain(2, 0.5);
    let mut out = Vec::new();
    mixer.mix(480, &mut out);
    assert!(out.iter().all(|s| (*s - 0.5).abs() < 1e-6));
}

#[test]
fn a_source_is_silent_until_primed() {
    let mut mixer = Mixer::default();
    mixer.push(1, &frames(0.5, TARGET_FRAMES - 1));
    let mut out = Vec::new();
    mixer.mix(480, &mut out);
    assert!(out.iter().all(|s| *s == 0.0));
}

#[test]
fn the_sum_is_clamped_to_full_scale() {
    let mut mixer = Mixer::default();
    mixer.push(1, &frames(0.9, TARGET_FRAMES));
    mixer.push(2, &frames(-0.9, TARGET_FRAMES));
    mixer.push(3, &frames(0.9, TARGET_FRAMES));
    mixer.push(4, &frames(0.9, TARGET_FRAMES));
    let mut out = Vec::new();
    mixer.mix(10, &mut out);
    assert!(out.iter().all(|s| (*s - 1.0).abs() < 1e-6));
}

#[test]
fn a_removed_source_stops_contributing() {
    let mut mixer = Mixer::default();
    mixer.push(1, &frames(0.5, TARGET_FRAMES));
    mixer.remove(1);
    let mut out = Vec::new();
    mixer.mix(10, &mut out);
    assert!(out.iter().all(|s| *s == 0.0));
}

#[test]
fn a_backlog_is_trimmed_to_the_target() {
    let mut mixer = Mixer::default();
    mixer.push(1, &frames(0.1, MAX_FRAMES));
    mixer.push(1, &frames(0.2, 10));
    let mut out = Vec::new();
    mixer.mix(TARGET_FRAMES, &mut out);
    assert!((out[out.len() - 1] - 0.2).abs() < 1e-6, "newest samples are kept");
}

#[test]
fn peaks_are_reported_after_gain_and_reset() {
    let mut mixer = Mixer::default();
    mixer.push(7, &frames(0.8, TARGET_FRAMES));
    mixer.set_gain(7, 0.5);
    let mut out = Vec::new();
    mixer.mix(10, &mut out);
    assert_eq!(mixer.take_peaks(), vec![(7, 0.4)]);
    assert_eq!(mixer.take_peaks(), vec![(7, 0.0)]);
}
```

(Compare floats with a tolerance in the peak test if exact equality fails clippy's `float_cmp`.)

- [ ] **Step 2: Implement** `Mixer` with a `VecDeque<f32>` per source in a `BTreeMap<u32, Source>`.

- [ ] **Step 3: Commit and push; verify CI green**

```bash
git add crates/capture/src/mixer.rs crates/capture/src/lib.rs
git commit -m "feat(capture): mix the sound of several apps"
git push origin main
```

---

### Task 4: Mixed audio capture

**Files:**

- Modify: `crates/capture/src/audio.rs` (no behavior change; `AudioCapture` is reused per app), `crates/capture/src/mixer.rs` (runtime part), `crates/capture/src/lib.rs`
- Test: `crates/capture/tests/live_capture.rs`

**Interfaces:**

- Consumes: `Mixer`, `AudioCapture::start(process_id, on_samples)`.
- Produces:
  - `pub struct MixedAudioCapture` with:
    - `pub fn start(on_samples: impl FnMut(&[f32]) + Send + 'static) -> Result<Self, Error>`: spawns thread `audio-mixer` that mixes on a deadline: frames due = elapsed since start × 48 000 − frames already sent, sent in chunks of 480 frames; sleeps until the next 10 ms deadline. Mixing and pushes share an `Arc<Mutex<Mixer>>`.
    - `pub fn sync(&mut self, apps: &[(u32, f32)])`: (root process id, gain). Starts an `AudioCapture` for each new id whose callback pushes into the mixer; stops and removes captures (and mixer sources) for ids not in `apps`; updates gains. A capture that fails to start is logged at `debug` and retried on the next `sync`.
    - `pub fn take_levels(&self) -> Vec<(u32, f32)>`: `Mixer::take_peaks`.
    - `pub fn stop(self)`, and `Drop` that stops the thread and every capture.

- [ ] **Step 1: Write the failing test** in `live_capture.rs`, reusing `write_tone`, `TestWindow`, `wait_for` and `find_window`:

```rust
#[test]
#[ignore = "needs a desktop session and an audio output device"]
fn mixes_only_the_included_apps() {
    let tone = write_tone(&std::env::temp_dir().join("telai-mix-test.wav"));
    let child = TestWindow::spawn(&tone);
    let window = wait_for(|| find_window(|w| w.playing_audio)).expect("the tone plays");

    let included = mixed_peak(&[(window.process_id, 1.0)]);
    assert!(included > 0.05, "included app peak {included}");

    let excluded = mixed_peak(&[]);
    assert!(excluded < 0.001, "excluded app peak {excluded}");
    drop(child);
}
```

`mixed_peak(apps)` starts a `MixedAudioCapture`, calls `sync(apps)`, collects the peak of the published samples for 3 seconds, and stops it.

- [ ] **Step 2: Implement** `MixedAudioCapture`; export it from `lib.rs`.

- [ ] **Step 3: Commit and push; verify CI**

```bash
git add crates/capture
git commit -m "feat(capture): capture and mix the sound of chosen apps"
git push origin main
```

Expected: green, and `mixes_only_the_included_apps ... ok` in the `Windows app` log.

---

### Task 5: Saved audio choices

**Files:**

- Create: `src-tauri/src/audio_prefs.rs`
- Modify: `src-tauri/src/main.rs` (`mod audio_prefs;`)

**Interfaces:**

- Produces:
  - `#[derive(Serialize, Deserialize)] pub struct AudioPrefs { excluded: BTreeSet<String>, volumes: BTreeMap<String, f32> }` with `Default` = Discord names excluded, no volumes.
  - `pub fn load(path: &Path) -> AudioPrefs`: defaults when the file is missing or invalid (warn on invalid).
  - `pub fn save(&self, path: &Path) -> std::io::Result<()>`: creates the parent dir; pretty JSON.
  - `pub fn is_included(&self, name: &str) -> bool`, `pub fn volume(&self, name: &str) -> f32` (1.0 when unset), `pub fn set(&mut self, name: &str, included: bool, volume: f32)` (volume clamped to 0.0–1.0; names lowercased).

- [ ] **Step 1: Write the failing tests** (in `audio_prefs.rs`, using a temp dir under `std::env::temp_dir()`):

```rust
#[test]
fn discord_is_excluded_by_default() {
    let prefs = AudioPrefs::default();
    assert!(!prefs.is_included("discord"));
    assert!(prefs.is_included("vlc"));
    assert!((prefs.volume("vlc") - 1.0).abs() < f32::EPSILON);
}

#[test]
fn a_missing_or_corrupt_file_gives_the_defaults() {
    let dir = temp_dir("corrupt");
    assert!(!load(&dir.join("absent.json")).is_included("discord"));
    std::fs::write(dir.join("bad.json"), "{not json").unwrap();
    assert!(!load(&dir.join("bad.json")).is_included("discord"));
}

#[test]
fn choices_survive_a_save_and_load() {
    let path = temp_dir("roundtrip").join("audio-mix.json");
    let mut prefs = AudioPrefs::default();
    prefs.set("Discord", true, 0.3);
    prefs.set("vlc", false, 1.5);
    prefs.save(&path).unwrap();

    let loaded = load(&path);
    assert!(loaded.is_included("discord"));
    assert!((loaded.volume("discord") - 0.3).abs() < 1e-6);
    assert!(!loaded.is_included("vlc"));
    assert!((loaded.volume("vlc") - 1.0).abs() < 1e-6);
}
```

- [ ] **Step 2: Implement.** **Step 3: Commit and push; verify CI green**

```bash
git add src-tauri/src/audio_prefs.rs src-tauri/src/main.rs
git commit -m "feat: remember which apps the stream includes"
git push origin main
```

---

### Task 6: Live session with a monitor and the mix

**Files:**

- Modify: `src-tauri/src/live.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/app.rs` (register commands, keep `Mutex<AudioPrefs>` and its path in `AppState`)

**Interfaces:**

- Consumes: `CaptureTarget`, `list_monitors`, `audio_apps`, `MixedAudioCapture`, `AudioPrefs`, `icon_data_url`, `executable_icon_png`.
- Produces (IPC, camelCase JSON):
  - `#[serde(tag = "kind", rename_all = "camelCase")] enum SourceRef { Window { handle: isize }, Monitor { handle: isize } }`.
  - Command `list_monitors() -> Vec<MonitorView { handle, number, name, width, height, primary, thumbnail }>`.
  - Command `start(source: SourceRef) -> LiveSnapshot` (replaces `handle`). For a monitor: `MixedAudioCapture` whose samples go to `hub.publish_audio` and the session level, title `"<name> (<width>×<height>)"` or the number when the name is empty.
  - Command `set_audio_app(name: String, included: bool, volume: f32)`: updates and saves prefs, then syncs the running mix at once.
  - `SourceView` gains `kind: "window" | "monitor"`.
  - `LiveSnapshot` gains `audio_apps: Option<Vec<AudioAppView>>`: `None` in window mode; in monitor mode one entry per `audio_apps()` row: `{ processId, name, icon, included, volume, level }`, `level` from `take_levels` (0 for excluded apps).
- Behavior: in monitor mode the monitor loop calls `audio_apps()` on every slow tick (1 s) and `sync`s the included ones with their volume; `minimized` is always false; `BrowserAudio::update` gets source process 0. Icons are cached per executable path for the session.

- [ ] **Step 1:** Add a unit test in `live.rs` for the pure helper `fn monitor_title(name: &str, number: usize, width: u32, height: u32, screen: &str) -> String`: `("DELL U2720Q", 1, 3840, 2160, "Tela")` → `"DELL U2720Q (3840×2160)"`, `("", 2, 1920, 1080, "Tela")` → `"Tela 2 (1920×1080)"`. The caller passes the localized word from `text()` in `text.rs` (add `screen`: "Screen" / "Tela"), so the test does not depend on the CI machine's locale.

- [ ] **Step 2: Implement** the commands and session changes.

- [ ] **Step 3: Commit and push; verify CI green**

```bash
git add src-tauri
git commit -m "feat: stream a whole monitor with the sound of the chosen apps"
git push origin main
```

---

### Task 7: Picker lists the monitors

**Files:**

- Modify: `src/app/types.ts`, `src/app/bridge.ts`, `src/app/use-live-session.ts`, `src/app/CurrentScreen.tsx`, `src/app/picker/PickerScreen.tsx`, `src/app/picker/PickerBody.tsx`, `src/app/picker/PickerFooter.tsx`, `src/app/picker/group-windows.ts`, `src/app/i18n/english.ts`, `src/app/i18n/portuguese.ts`, `src/app/common/error-message.ts`
- Create: `src/app/picker/use-sources.ts` (replaces `use-windows.ts`), `src/app/picker/MonitorGroup.tsx`, `src/app/picker/MonitorTile.tsx`
- Test: `src/app/picker/group-windows.test.ts`

**Interfaces:**

- Consumes: Task 6 IPC.
- Produces:
  - `type SourceRef = { kind: "window"; handle: number } | { kind: "monitor"; handle: number }`; `MonitorView`; `AudioAppView`; `SourceView.kind`; `LiveSnapshot.audioApps: AudioAppView[] | null`.
  - `bridge.listMonitors()`, `bridge.start(source: SourceRef)`, `bridge.setAudioApp(name, included, volume)`.
  - `selectedSource(windows, monitors, picked: SourceRef | null): SourceRef | null` in `group-windows.ts`: the picked source while it still exists; otherwise the frontmost window playing sound; otherwise null.
  - Monitors show as a "Telas"/"Screens" group above the window groups, filtered by the same query on their name; tile shows the thumbnail and `name`, with the `Monitor` lucide icon.
  - Copy: `groupScreens`, `screenName(number)`, and `endedWindowClosed` becomes "A transmissão parou: a janela ou tela foi fechada." / "The stream stopped: the window or screen closed."

- [ ] **Step 1: Write the failing tests** in `group-windows.test.ts`: `selectedSource` returns a picked monitor while listed; falls back to the first playing window when the picked monitor is gone; returns null when nothing plays and nothing is picked.

- [ ] **Step 2: Run** `pnpm test` → the new tests fail. **Step 3: Implement.** **Step 4: Run** `pnpm format:check && pnpm lint && pnpm typecheck && pnpm test` → all pass.

- [ ] **Step 5: Commit and push; verify CI green**

```bash
git add src/app
git commit -m "feat(app): pick a whole screen to stream"
git push origin main
```

---

### Task 8: Audio panel on the live screen

**Files:**

- Create: `src/app/live/AudioMix.tsx`, `src/app/live/audio-mix.ts`, `src/app/live/audio-mix.test.ts`
- Modify: `src/app/live/LiveScreen.tsx`, `src/app/app.css`, `src/app/i18n/english.ts`, `src/app/i18n/portuguese.ts`, `README.md`, `README.pt-BR.md`, `src-tauri/tauri.conf.json` (`shortDescription`)

**Interfaces:**

- Consumes: `LiveSnapshot.audioApps`, `bridge.setAudioApp`, `levelPercent`.
- Produces:
  - `mixRows(apps: AudioAppView[]): AudioAppView[]` in `audio-mix.ts`: included apps first, then by name; the same name with several processes keeps process order.
  - `<AudioMix apps={live.audioApps} />` rendered between `Monitor` and `SourceRow` only when `audioApps !== null`. Each row: checkbox (include), icon, name, level bar (`levelPercent`), range input 0–100 for volume. A slider keeps its local value while the pointer is down and sends `setAudioApp` on change; the next snapshot takes over after release. An empty list shows `audioMixEmpty`. The list scrolls past four rows, so the 560 px minimum window height still fits.
  - Copy: `audioMixTitle` "Som transmitido" / "Streamed sound", `audioMixEmpty` "Nenhum app tocando som" / "No app is playing sound", `includeApp(name)`, `volumeOf(name)`.
  - Docs: README sections mention whole-screen streaming and the per-app sound list with Discord left out by default; `shortDescription` "Streams a window or a whole screen, with only the sound you choose, to a local browser tab".

- [ ] **Step 1: Write the failing test** for `mixRows` ordering. **Step 2: Run** `pnpm test` → fails. **Step 3: Implement.** **Step 4: Run** all four frontend checks → pass.

- [ ] **Step 5: Commit and push; verify CI green**, then download the `telai-installer` artifact and hand it to the user to try: stream a screen, see the app list, uncheck/check an app and drag a volume while listening in a second, unmuted browser on another device or tab.

```bash
git add src README.md README.pt-BR.md src-tauri/tauri.conf.json
git commit -m "feat(app): choose and balance the apps the stream carries"
git push origin main
```
