# claude-swap-tray

A Windows desktop GUI app that manages multiple Claude Code accounts across **Windows native + every WSL distro on the same host**, from a single install.

Inspired by [realiti4/claude-swap](https://github.com/realiti4/claude-swap), but rewritten in **Rust + iced** with a Windows-first design: one window to manage accounts, switch them, monitor usage, and orchestrate `claude login` flows for any install location.

> **Status: early scaffold (v0.1).** GUI shell renders, modules are stubbed. Not usable yet. PRs welcome.

## Why

`claude-swap` (Python) is great but treats Windows and WSL as **isolated platforms** — separate installs, separate account pools, separate switch operations. If you run Claude Code in both (most Windows devs do), you manage everything twice.

`claude-swap-tray` treats them as **one host** with multiple install locations:

- **Windows native** install — `%USERPROFILE%\.claude\`
- **Each WSL distro** — accessed via `\\wsl$\<distro>\home\<user>\.claude\` (UNC path; Windows can read/write the WSL filesystem)

One pool of accounts. One window. Switch in all locations or scope to one.

## Features

- [x] Project scaffold (Rust 2024, iced 0.14, Windows-only)
- [x] Location discovery (Windows native + WSL distros via `wsl -l -q`)
- [x] iced GUI shell — Accounts / Add account / Settings screens
- [x] Account model + persistence (Windows Credential Manager + JSON manifest)
- [x] OAuth token decode + refresh against `platform.claude.com/v1/oauth/token`
- [x] `.credentials.json` parser with serde flatten for unknown fields (round-trip safe)
- [x] Add-account flow: `claude login` runs on Windows, credentials auto-sync to all WSL distros
- [x] Replicate credentials across all locations on switch (atomic writes via temp+rename)
- [x] Anthropic usage API client + tolerant parser for varying response shapes
- [x] Background monitor: poll usage, anti-spam (30 min suppression), threshold alert
- [x] Native Windows toast (tauri-winrt-notification) with sound
- [x] System tray with Show / Hide / Quit menu; window X minimizes to tray
- [x] Settings persistence (threshold, poll interval, sound, auto-rotate)
- [ ] Auto-rotate wired (setting exists, not yet acted on)
- [ ] Per-account quick switch from tray menu
- [ ] Designed `assets/icon.ico` (procedural orange disc placeholder for now)
- [ ] MSIX/winget distribution

## Architecture

```
src/
  main.rs                 # entry, init tracing, run iced::application
  app.rs                  # iced Application: Message, App, update, view, subscription
  screens/
    mod.rs                # Screen enum, root_view dispatcher, nav bar
    accounts.rs           # accounts list + switch/remove
    add_account.rs        # location picker + login orchestration UI
    settings.rs           # threshold, poll, sound, auto-rotate
  account.rs              # Account + OAuthCredentials models
  store.rs                # account persistence (keyring + JSON)
  oauth.rs                # token decode, refresh
  usage.rs                # Anthropic /api/oauth/usage client
  switcher.rs             # high-level account ops (apply across all locations)
  login.rs                # spawn `claude login`, watch creds file, parse
  config.rs               # user settings (Settings struct)
  platform/
    mod.rs                # Location enum, discover_locations()
    windows.rs            # native install discovery
    wsl.rs                # WSL distro enumeration + UNC path build
  monitor.rs              # background polling loop (Windows-only)
  notify.rs               # tauri-winrt-notification wrapper (Windows-only)
  tray.rs                 # system tray (Windows-only, v0.2)
```

## Stack

| Layer | Choice |
|---|---|
| GUI | [iced](https://iced.rs) 0.14 — pure Rust, Elm architecture |
| Async runtime | tokio (multi-threaded, used by iced) |
| HTTP | reqwest + rustls |
| Tray (v0.2) | tray-icon 0.24 |
| Toast | tauri-winrt-notification 0.7 |
| Credentials | keyring 3 with windows-native backend |
| Single-instance | single-instance 0.3 |

No web technologies. Single static binary.

## Build

Requires Rust 1.85+ (edition 2024).

```powershell
# On Windows:
cargo build --release
# Binary: target/release/claude-swap-tray.exe (~10MB stripped)
```

The crate also `cargo check`s on Linux for editing convenience, but the runnable build target is Windows MSVC. CI builds on `windows-latest`.

## Usage (target — most flows stubbed)

Double-click `claude-swap-tray.exe` or run from terminal — opens the main window. There is no CLI. Power users wanting headless ops can use upstream [`claude-swap`](https://github.com/realiti4/claude-swap).

Screens:

- **Accounts** — list managed accounts, see usage, click Switch to make one active across all locations.
- **Add account** — pick a Claude Code install location, click Start login, complete the OAuth flow in the browser. The new account appears in the list.
- **Settings** — usage threshold, poll interval, sound, optional auto-rotate.

## How switching works

`claude-swap-tray` does **not** hot-reload running Claude Code processes — Anthropic's CLI caches the OAuth token in memory at startup. After a switch:

1. Credentials are written to every install location
2. Toast notifies you which Claude Code processes need restart
3. You close + reopen them, then `claude --resume <session>` if you want to continue

Same constraint upstream `claude-swap` documents.

## ToS posture

Each account must be legitimately yours. Switching between your own multiple accounts is fine. Using this to share one license between people, or to evade rate limits on a single account, is not — and Anthropic's terms forbid it.

## License

MIT — see [LICENSE](LICENSE).
