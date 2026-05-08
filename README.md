# claude-swap-tray

A Windows tray app that manages multiple Claude Code accounts across **Windows native + every WSL distro on the same host**, from a single install.

Inspired by [realiti4/claude-swap](https://github.com/realiti4/claude-swap), but rewritten in Rust with a Windows-first design: one process to switch, monitor usage, and notify across all your Claude Code installs.

> **Status: early scaffold.** Architecture is in place; most modules are stubbed. Not usable yet. PRs welcome.

## Why

`claude-swap` is great but treats Windows and WSL as **isolated platforms**. If you run Claude Code in both (which most Windows devs do), you're managing two separate account pools, switching twice, never seeing unified usage.

`claude-swap-tray` treats them as **one host** with multiple install locations:

- **Windows native** install — `%USERPROFILE%\.claude\`
- **Each WSL distro** — accessed via `\\wsl$\<distro>\home\<user>\.claude\` (UNC path; Windows can read/write the WSL filesystem)

One pool of accounts, one tray icon, one set of switch operations. Apply to all locations or scope to one.

## Features (target)

- [x] Project scaffold (Rust, Windows-only, async)
- [x] Location discovery (Windows native + WSL distros via `wsl -l -q`)
- [ ] Account model + persistence (Windows Credential Manager + JSON manifest)
- [ ] OAuth token decode + refresh
- [ ] Anthropic usage API client
- [ ] CLI: `add`, `list`, `switch`, `switch-to`, `remove`, `status`, `locations`
- [ ] Tray icon with menu (current account, usage %, switch list, settings, quit)
- [ ] Background monitor — poll usage, fire toast at threshold
- [ ] Native Windows toast with action buttons + sound
- [ ] Single-instance enforcement
- [ ] Settings persistence (threshold, poll interval, sound)
- [ ] MSIX/winget distribution

## Architecture

```
src/
  main.rs           # entry, init tracing + tokio runtime
  cli.rs            # clap subcommands
  account.rs        # Account + OAuthCredentials models
  store.rs          # account persistence (keyring + JSON)
  oauth.rs          # JWT decode, refresh
  usage.rs          # Anthropic /api/oauth/usage client
  switcher.rs       # add/remove/list/switch operations across locations
  config.rs         # user settings
  platform/
    mod.rs          # Location enum, discover_locations()
    windows.rs      # native install discovery
    wsl.rs          # WSL distro enumeration + UNC path build
  monitor.rs        # background polling loop (Windows-only)
  notify.rs         # winrt-toast wrapper (Windows-only)
  tray.rs           # tray-icon + tao event loop (Windows-only)
```

## Build

Requires Rust 1.85+ (edition 2024).

```powershell
# On Windows:
cargo build --release
# Binary: target/release/claude-swap-tray.exe
```

The crate also `cargo check`s on Linux for editing convenience, but the runnable build target is Windows MSVC. CI builds on `windows-latest`.

## Usage (target — most subcommands stubbed)

```powershell
claude-swap-tray              # launch tray + monitor
claude-swap-tray start        # same
claude-swap-tray stop         # tell running instance to exit
claude-swap-tray status       # current account, locations, monitor state
claude-swap-tray locations    # list all detected install locations
claude-swap-tray add          # capture currently-logged-in account
claude-swap-tray list         # all managed accounts with usage
claude-swap-tray switch       # rotate to next account
claude-swap-tray switch-to user@example.com
claude-swap-tray remove user@example.com
```

## How switching works

`claude-swap-tray` does **not** hot-reload running Claude Code processes — Anthropic's CLI caches the OAuth token in memory at startup. After a switch:

1. Credentials are written to every install location
2. Toast notifies you which Claude Code processes need restart
3. You close + reopen them, then `claude --resume <session>` if you want to continue

This is the same constraint upstream `claude-swap` documents.

## ToS posture

Each account must be legitimately yours. Switching between your own multiple accounts is fine. Using this to share one license between people, or to evade rate limits on a single account, is not — and Anthropic's terms forbid it.

## License

MIT — see [LICENSE](LICENSE).
