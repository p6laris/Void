# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0-beta.1] - 2026-07-13

### 🚀 Features

- *(ui)* Detach dashboard task view from tasks tab filters
- *(db)* Add cli export and import support for full database state
- *(model)* Add streak freeze and rest day fields
- *(db)* Persist streak freeze and rest day settings
- *(streak)* Add rest days and streak freeze logic
- *(ui)* Display streak freeze count in summary panel
- *(settings)* Add rest days toggle to settings
- *(subtask)* Add reorder, rename, and edit subtask support

### 🐛 Bug Fixes

- *(ui)* Improve task filter behavior and header count display
- *(ui)* Remove inactive filter keybindings from dashboard view
- *(db)* Preserve full session metadata in export and import
- *(keys)* Remove duplicate bulk-mode Esc handler
- *(storage)* Exclude archived tasks from auto-pick
- *(storage)* Only adjust today focus for same-day sessions
- *(db)* Return errors from parse_datetime instead of silently replacing
- *(storage)* Correct weekly and monthly streaks across year boundaries
- *(stats)* Avoid panics when selecting stats sessions
- *(ui)* Guard heatmap month labels and empty week chart
- *(ui)* Give Help and About separate scroll state
- *(storage)* Return error when update_task id is missing
- *(cli)* Reject invalid task ids and handle import flush errors
- *(storage)* Assign unique subtask ids on recurring spawn
- *(ci)* Harden publish release workflow push and token handling
- *(ui)* Prevent empty bar chart rendering and rename weekly panel
- *(stats)* Refresh tag analytics on chart dirty flag
- *(ui)* Always show streak freeze count in top bar and summary
- *(ui)* Move streak freezes to dedicated summary row
- *(ui)* Show freeze count as available/max in top bar
- *(model)* Default to 3/3 streak freezes for new users
- *(subtask)* Guard x/- behind focus mode and fix Tab fallthrough
- *(test)* Resolve clippy warnings for struct initialization in tests

### 🚜 Refactor

- *(popups)* Centralize confirmed task delete handling
- Add today_str helper for local date formatting
- Centralize task lookup and static bool settings
- *(model)* Add TimerMode::is_break method
- *(sound)* Send clips directly to the audio worker
- *(data)* Drop in-memory session_history from AppData
- *(theme)* Remove ThemeTokens in favor of Theme
- *(app)* Split App into UiState, InputState, TaskUiState, StatsState
- *(db)* Extract timer mode encode/decode to encoding.rs
- *(db)* Centralize FocusSessionRecord row mapping
- *(app)* Dedupe stats session refresh after heatmap edits
- Dedupe mark-done, themed_panel, and task status colors
- *(settings)* Dedupe persisted timer setting adjustments
- Centralize open-task checks and date formatting
- *(ui)* Share streak, goal, and session chips in chrome
- *(ui)* Share footer layout between normal and zen modes
- *(ui)* Centralize inline subtask line rendering
- *(timer)* Simplify mode toggle to Focus/Custom only

### 📚 Documentation

- Update screenshots and add Zen Mode to README
- Add custom theme documentation to README
- Add data import and export section to README
- Update clippy command to include --all-targets
- *(help)* Update subtask key bindings documentation

### ⚡ Performance

- *(db)* Bulk-load task tags, subtasks, and blockers
- *(db)* Bulk-load session tags for session queries
- *(app)* Split bump helpers and drop duplicate cache recompute
- *(db)* Load chart focus minutes with one grouped query
- *(db)* Load session mode counts with one grouped query
- *(db)* Persist session stats in one transaction
- *(db)* Reuse prepared statement when saving settings
- *(storage)* Batch auto-archive task writes in one transaction
- *(app)* Avoid allocating dashboard task list each call
- *(ui)* Cache today date and focus minutes per frame
- *(app)* Cache sorted task tags for tag filter cycling
- *(db)* Reuse prepared statement in sync_sort_orders
- *(app)* Throttle terminal window title updates
- *(app)* Cache task blocked status in recompute_task_caches
- *(ui)* Avoid cloning theme in draw_tasks each frame
- *(ui)* Clone popup only when one is open
- *(ui)* Cache settings label strings between redraws
- *(ui)* Reuse date key buffer in heatmap grid build
- *(ui)* Avoid cloning task for subtask panel each frame
- *(db)* Run PRAGMA optimize after migrations and imports
- *(model)* Store tasks in IndexMap for O(1) ID lookups
- Optimize dev profile to reduce debug memory usage
- Initialize audio lazily and reduce render tick rate

### 🎨 Styling

- Fix clippy warnings and unused imports
- Apply cargo fmt
- *(zen)* Anchor break tip to the bottom edge of the canvas
- Format code to pass CI

### 🧪 Testing

- Fix clippy test warnings for struct defaults
- *(streak)* Add tests for rest days and streak freezes


