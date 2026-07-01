# Void — Roadmap, Packaging & Release Strategy

> Living document covering feature improvements, code quality, packaging for
> Winget / Homebrew, GitHub Actions CI/CD, and branch management.

---

## Table of Contents

1. [Focus App Improvements](#1-focus-app-improvements)
2. [General Improvements](#2-general-improvements)
3. [Code Quality & Refactors](#3-code-quality--refactors)
4. [Packaging Strategy](#4-packaging-strategy)
5. [GitHub Actions CI/CD](#5-github-actions-cicd)
6. [Branch Management](#6-branch-management)

---

## 1. Focus App Improvements

### 1.1 Tasking

| Area | Current State | Improvement |
|------|--------------|-------------|
| **Subtasks** | Tasks are flat | Add subtask/checklist support — each `Task` gets an optional `Vec<Subtask>` with title + done flag |
| **Recurring tasks** | Not supported | Add `recurrence` field (`daily`, `weekly`, `weekdays`, `custom`) — auto-recreate on completion |
| **Task dependencies** | None | Optional `blocked_by: Vec<u64>` — blocked tasks can't be set active |
| **Task archiving** | Done tasks stay in DB forever | Add `archive` command / auto-archive after N days, with `void archive list` CLI |
| **Bulk operations** | One task at a time | Multi-select with `v` key, then bulk done/delete/priority/tag |
| **Drag-reorder UX** | `Ctrl+j/k` works but no visual feedback | Show a "moving ↕" indicator and ghost-row while reordering |
| **Due date reminders** | Overdue shown in red, no proactive alert | On app launch, show notification if any tasks are due today or overdue |
| **Task notes** | Plain text single-line | Allow multi-line notes with a scrollable editor popup |

### 1.2 Sessions & Timer

| Area | Current State | Improvement |
|------|--------------|-------------|
| **Session notes** | Sessions have no notes | Add optional `note` field to `FocusSessionRecord` — quick log of what you did |
| **Session tagging** | Sessions inherit task_id only | Let users tag sessions (e.g., "deep work", "meetings") |
| **Interruption tracking** | Not tracked | Track pause count & total pause duration per session |
| **Pomodoro streaks** | `completed_focus_sessions` resets daily | Add weekly/monthly streak tracking with visual badges |
| **Timer presets** | Only Focus/Short/Long/Custom | Named presets (e.g., "Deep Work 50/10", "Quick 15/3") stored in settings |
| **Session history pagination** | Limited to 15 recent | Add pagination or infinite scroll in Stats tab |
| **Daily session timeline** | Not visualized | Horizontal timeline showing session blocks across the day |
| **Focus score** | No productivity metric | Calculate daily score based on focus-to-break ratio, goal completion, and consistency |

### 1.3 Timer UX

| Area | Improvement |
|------|-------------|
| **Countdown in terminal title** | Already done ✓ — but add option to disable it |
| **Session end warning** | Flash border or play sound at 1-minute remaining |
| **Auto-pause on idle** | Detect terminal inactivity, auto-pause after configurable timeout |
| **Extended timer display** | Show total elapsed today alongside current countdown |

---

## 2. General Improvements

### 2.1 CLI Enhancements

The current CLI (`add`, `list`, `done`, `start`, `help`) is solid. Additions:

```
void stats                # Print today/week/month stats to stdout
void edit <id> --title    # Edit task fields from CLI
void delete <id>          # Delete without TUI
void export [--csv]       # Export sessions as CSV
void import <file>        # Import from JSON backup
void config <key> <value> # Set settings from CLI
void version              # Print version + build info
void completions <shell>  # Generate shell completions (bash/zsh/fish/powershell)
```

> **Implementation**: Use `clap` crate for proper argument parsing instead of manual `args` matching. This also gives us `--help` auto-generation and shell completions for free.

### 2.2 UI Improvements

- **Resizable panels**: Dashboard timer/task split should adapt to terminal width
- **Mouse support**: Optional click-to-select in task lists (crossterm supports it)
- **Color-blind mode**: Alternative palette using patterns/shapes instead of color-only differentiation
- **Task detail sidebar**: Expand the Tasks tab detail pane with session history for that task
- **Notification badge on tab**: Show count of overdue tasks on the Tasks tab label
- **Unicode fallback**: Detect terminal capability, fall back to ASCII if Nerd Fonts unavailable

### 2.3 Stats & Analytics

- **Monthly/yearly view toggle** in the stats tab
- **Exportable reports**: Markdown or HTML summary of weekly productivity
- **Productivity trends**: Compare this week vs. last week
- **Tag-based analytics**: Focus time breakdown by tag
- **Peak hours chart**: Visualize the `most_productive_hour_label` data as a bar chart

### 2.4 Settings

- **Import settings**: `void config import <file>` for sharing configs across machines
- **Reset to defaults**: One-click reset in Settings tab
- **Profile presets**: Save/load named setting profiles

---

## 3. Code Quality & Refactors

### 3.1 Split `app.rs` (2,084 lines)

This is the most urgent refactor. The file handles state, key bindings, settings, popups, and business logic.

```
src/
├── app/
│   ├── mod.rs          # App struct, new(), bump_data(), public API
│   ├── keys.rs         # handle_key(), handle_dashboard_key(), etc.
│   ├── settings.rs     # SettingsState, adjust_setting(), settings key handlers
│   ├── popups.rs       # handle_popup_key(), submit_popup(), popup input logic
│   ├── timer_ops.rs    # start/pause/reset/skip/cycle_mode/adjust_minutes
│   ├── task_ops.rs     # set_active_task, mark_done, cycle_status, task selection
│   └── theme.rs        # Theme struct and all palette definitions
```

### 3.2 Split `ui/mod.rs` (1,449 lines)

```
src/ui/
├── mod.rs              # render() dispatch only
├── dashboard.rs        # draw_dashboard(), draw_compact_timer_block()
├── tasks.rs            # draw_tasks(), build_task_detail()
├── settings.rs         # draw_settings()
├── help.rs             # draw_help()
├── popups.rs           # draw_popup(), draw_input()
├── zen.rs              # draw_zen_dashboard()
├── chrome.rs           # (existing) header/footer/tabs
├── stats.rs            # (existing)
├── heatmap.rs          # (existing)
├── widgets.rs          # (existing)
└── icons.rs            # (existing)
```

### 3.3 Error Handling

- **Current**: `persist()` silently shows status bar errors — failures are easy to miss
- **Improve**: Add structured error types with `thiserror` crate
- **Logging**: Add optional file logging with `tracing` crate, enabled via `VOID_LOG=debug`

```rust
#[derive(Debug, thiserror::Error)]
pub enum VoidError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Export failed: {0}")]
    Export(#[from] std::io::Error),
}
```

### 3.4 Deduplication

| Duplicate | Location | Fix |
|-----------|----------|-----|
| `decode_timer_mode()` | `db/mod.rs` + `db/export.rs` | Move to shared `db/encoding.rs` |
| Task selection clamping | `submit_popup()` + `confirm_delete()` | Extract `clamp_task_selection_after_mutation()` |
| Settings persist pattern | Every `SettingsItem` arm in `adjust_setting()` | Extract `toggle_bool_setting()` and `adjust_numeric_setting()` helpers |
| Sort comparator | `sorted_pending_tasks()` + `dashboard_tasks()` | Single `task_sort_key()` function |

### 3.5 Testing

- **Current**: Zero tests (the only test file is an empty `test_cal.rs`)
- **Unit tests**: Timer logic, task sorting, storage functions, date normalization
- **Integration tests**: Database round-trip (create → read → update → delete)
- **Snapshot tests**: UI rendering with `insta` crate for Ratatui frame snapshots
- **CI gate**: Tests must pass before merge (already in `rust.yml`, just need actual tests)

### 3.6 Other Quality Items

- [ ] Add `#![deny(clippy::all)]` and `#![warn(clippy::pedantic)]` to `lib.rs`
- [ ] Add doc comments to all public functions
- [ ] Remove `test_cal.rs` from repo root
- [ ] Add `rust-toolchain.toml` to pin MSRV (e.g., `1.75.0`)
- [ ] Add `.editorconfig` for consistent formatting
- [ ] Run `cargo fmt --check` in CI
- [ ] Add `CHANGELOG.md` (keep-a-changelog format)
- [ ] Add `CONTRIBUTING.md`

---

## 4. Packaging Strategy

### 4.1 Target Package Managers

| Manager | Platform | Format | Priority |
|---------|----------|--------|----------|
| **Homebrew** | macOS + Linux | Formula (Ruby DSL) | ★★★ High |
| **Winget** | Windows 10/11 | YAML manifest | ★★★ High |
| **Cargo** | Cross-platform | `crates.io` | ★★★ High |
| **AUR** | Arch Linux | `PKGBUILD` | ★☆☆ Low (community) |

### 4.2 Preparation Checklist

Before publishing to any package manager:

- [x] `Cargo.toml` has `name`, `version`, `description`, `license`, `authors`
- [x] Add `repository = "https://github.com/p6laris/Void"` to `Cargo.toml`
- [x] Add `readme = "README.md"` to `Cargo.toml`
- [x] Add `keywords` and `categories` to `Cargo.toml`
- [x] Add `homepage` URL to `Cargo.toml`
- [x] Add `exclude` patterns (`.github/`, `docs/`, `assets/`, `.idea/`)
- [x] Create `CHANGELOG.md` — required by Winget and good practice
- [x] Decide on binary name: currently `void` — check for conflicts on each manager
- [ ] Tag releases with semver: `v0.3.0-alpha.1`, `v0.3.0`, etc.
- [ ] Create GitHub Releases with pre-built binaries attached

### 4.3 Cargo (crates.io)

**Easiest** — publish the Rust crate directly.

```bash
# One-time setup
cargo login <your-token>

# Publish
cargo publish --dry-run   # verify first
cargo publish
```

Users install with:
```bash
cargo install void-focus
```

The installed command is `void`.

### 4.4 Homebrew

Create a tap repository: `github.com/p6laris/homebrew-tap`

**Formula file** (`Formula/void.rb`):
```ruby
class Void < Formula
  desc "Terminal focus timer with task tracking"
  homepage "https://github.com/p6laris/Void"
  url "https://github.com/p6laris/Void/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "Void CLI", shell_output("#{bin}/void --help")
  end
end
```

Users install with:
```bash
brew tap p6laris/tap
brew install void
```

**Pre-built bottle approach** (faster install): Build binaries in CI, upload as bottles.

### 4.5 Winget

Create a manifest and submit to `microsoft/winget-pkgs`.

**Directory**: `manifests/p/p6laris/Void/0.2.0/`

**`p6laris.Void.yaml`**:
```yaml
PackageIdentifier: p6laris.Void
PackageVersion: 0.2.0
PackageName: Void
Publisher: p6laris
License: MIT
ShortDescription: Terminal focus timer with task tracking
PackageUrl: https://github.com/p6laris/Void
Installers:
  - Architecture: x64
    InstallerType: portable
    InstallerUrl: https://github.com/p6laris/Void/releases/download/v0.2.0/void-x86_64-pc-windows-msvc.zip
    InstallerSha256: PLACEHOLDER
ManifestType: singleton
ManifestVersion: 1.6.0
```

Users install with:
```powershell
winget install p6laris.Void
```

> **Tip**: Use `wingetcreate` tool to auto-generate manifests from GitHub releases.

---

## 5. GitHub Actions CI/CD

### 5.1 Current State

The existing `rust.yml` only runs `cargo build` and `cargo test` on `ubuntu-latest` for pushes to `main`. This needs significant expansion.

### 5.2 Proposed Workflow Architecture

```
.github/workflows/
├── ci.yml              # Lint + test on every PR and push
├── release.yml         # Build binaries + publish on tag push
└── packaging.yml       # Update Homebrew/Winget on release
```

### 5.3 CI Workflow (`ci.yml`)

Replaces the current `rust.yml`:

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-Dwarnings"

jobs:
  check:
    name: Check & Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --verbose

  build:
    name: Build (${{ matrix.os }})
    needs: check
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release --target ${{ matrix.target }}
```

### 5.4 Release Workflow (`release.yml`)

Triggered when you push a version tag:

```yaml
name: Release

on:
  push:
    tags: ["v*"]

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.target }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: void-linux-amd64
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: void-macos-amd64
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: void-macos-arm64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: void-windows-amd64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2

      - name: Build release binary
        run: cargo build --release --target ${{ matrix.target }}

      - name: Package (Unix)
        if: runner.os != 'Windows'
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../${{ matrix.artifact }}.tar.gz void
          cd ../../..
          sha256sum ${{ matrix.artifact }}.tar.gz > ${{ matrix.artifact }}.tar.gz.sha256

      - name: Package (Windows)
        if: runner.os == 'Windows'
        shell: pwsh
        run: |
          Compress-Archive -Path target/${{ matrix.target }}/release/void.exe `
            -DestinationPath ${{ matrix.artifact }}.zip
          (Get-FileHash ${{ matrix.artifact }}.zip -Algorithm SHA256).Hash | `
            Out-File -FilePath ${{ matrix.artifact }}.zip.sha256

      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: |
            *.tar.gz
            *.zip
            *.sha256

  publish:
    name: Create GitHub Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          merge-multiple: true

      - name: Create Release
        uses: softprops/action-gh-release@v2
        with:
          generate_release_notes: true
          files: |
            *.tar.gz
            *.zip
            *.sha256

  crates-io:
    name: Publish to crates.io
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo publish
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

### 5.5 Packaging Workflow (`packaging.yml`)

Runs after a GitHub Release is published:

```yaml
name: Update Package Managers

on:
  release:
    types: [published]

jobs:
  homebrew:
    name: Update Homebrew Tap
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          repository: p6laris/homebrew-tap
          token: ${{ secrets.TAP_GITHUB_TOKEN }}

      - name: Update formula
        run: |
          VERSION="${{ github.event.release.tag_name }}"
          VERSION="${VERSION#v}"
          URL="https://github.com/p6laris/Void/archive/refs/tags/${{ github.event.release.tag_name }}.tar.gz"
          SHA=$(curl -sL "$URL" | sha256sum | cut -d' ' -f1)
          sed -i "s|url \".*\"|url \"$URL\"|" Formula/void.rb
          sed -i "s|sha256 \".*\"|sha256 \"$SHA\"|" Formula/void.rb
          sed -i "s|version \".*\"|version \"$VERSION\"|" Formula/void.rb

      - name: Push update
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Formula/void.rb
          git commit -m "Update void to ${{ github.event.release.tag_name }}"
          git push

  winget:
    name: Submit to Winget
    runs-on: windows-latest
    steps:
      - name: Submit manifest
        run: |
          iwr https://aka.ms/wingetcreate/latest -OutFile wingetcreate.exe
          .\wingetcreate.exe update p6laris.Void `
            --version ${{ github.event.release.tag_name }} `
            --urls "https://github.com/p6laris/Void/releases/download/${{ github.event.release.tag_name }}/void-windows-amd64.zip" `
            --submit --token ${{ secrets.WINGET_PAT }}
```

### 5.6 Required GitHub Secrets

| Secret | Purpose |
|--------|---------|
| `CARGO_REGISTRY_TOKEN` | crates.io API token |
| `TAP_GITHUB_TOKEN` | PAT with repo access to `homebrew-tap` |
| `WINGET_PAT` | GitHub PAT for winget-pkgs PRs |

---

## 6. Branch Management

### 6.1 Branch Strategy

Use a simplified **Git Flow** model:

- `main` → production releases only, tagged with semver
- `develop` → integration branch for the next release
- `feature/*` → individual features branched from develop
- `release/x.y.z` → release prep (version bump, changelog, fixes)
- `hotfix/*` → critical production fixes

### 6.2 Branch Roles

| Branch | Purpose | Protected | Merges to |
|--------|---------|-----------|-----------|
| `main` | Production-ready releases only | ✅ Yes | — |
| `develop` | Integration branch for next release | ✅ Yes | `main` (via release branch) |
| `feature/*` | Individual features or improvements | No | `develop` |
| `release/x.y.z` | Release prep (version bump, changelog) | No | `main` + `develop` |
| `hotfix/*` | Critical production fixes | No | `main` + `develop` |

### 6.3 Branch Protection Rules

**For `main`**:
- Require PR with at least 1 approval (even self-approval for solo dev)
- Require CI status checks to pass (`ci.yml`)
- No direct pushes
- Require linear history (squash merge)

**For `develop`**:
- Require CI status checks to pass
- Allow direct pushes for small fixes
- Merge commits allowed

### 6.4 Release Workflow (Step by Step)

```bash
# 1. Create release branch from develop
git checkout develop && git pull
git checkout -b release/0.3.0

# 2. Bump version in Cargo.toml, update CHANGELOG.md
git commit -am "chore: prepare v0.3.0 release"

# 3. Merge to main
git checkout main
git merge --no-ff release/0.3.0
git tag v0.3.0

# 4. Push tag — triggers release.yml + packaging.yml
git push origin main --tags

# 5. Back-merge to develop
git checkout develop
git merge release/0.3.0
git push origin develop

# 6. Clean up
git branch -d release/0.3.0
```

### 6.5 Existing Branches (Cleanup)

You have 4 stale feature branches that appear to be merged:

| Branch | Action |
|--------|--------|
| `feature/session-management` | Delete if merged |
| `feature/sqlite` | Delete (SQLite is in main) |
| `feature/task-status-ui` | Delete |
| `feature/ui-polish` | Delete |

```bash
git branch -d feature/sqlite feature/task-status-ui feature/ui-polish feature/session-management
git push origin --delete feature/sqlite feature/task-status-ui feature/ui-polish feature/session-management
```

### 6.6 Versioning

Follow **Semantic Versioning** (`MAJOR.MINOR.PATCH`):

- `0.2.0` → Current
- `0.3.0` → Code quality refactors + CLI improvements
- `0.4.0` → Packaging live (Homebrew + Winget + crates.io)
- `0.5.0` → Subtasks + recurring tasks
- `1.0.0` → Feature-complete and stable

---

## Quick-Start Priority Order

Recommended execution order for maximum impact:

1. **Add `repository`, `keywords`, `categories` to `Cargo.toml`** — 5 min, unblocks crates.io
2. **Clean up stale branches** — 5 min
3. **Replace `rust.yml` with `ci.yml`** — cross-platform CI with clippy/fmt
4. **Split `app.rs`** — biggest code quality win
5. **Add unit tests** for timer, storage, date normalization
6. **Create `CHANGELOG.md`** — needed for all package managers
7. **Publish to crates.io** — easiest distribution channel
8. **Set up Homebrew tap** — biggest user reach on macOS/Linux
9. **Submit to Winget** — Windows users
