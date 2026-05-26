# Desktop Relaunch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `Codex Desktop` and `Claude Desktop` restart before opening when they are already running, while keeping CLI launch behavior unchanged.

**Architecture:** Keep the change inside `src-tauri/src/apps/launcher.rs` so the existing `/api/apps/launch` endpoint and frontend flow stay unchanged. Add small helper functions for desktop app name resolution and macOS restart flow, then cover the decision logic with focused unit tests instead of relying on real desktop processes.

**Tech Stack:** Rust, Tokio process APIs, macOS `osascript`/`pgrep`, existing unit tests in `src-tauri/src/apps/launcher.rs`

---

## File Structure

| File | Responsibility |
|---|---|
| `docs/superpowers/specs/2026-05-26-desktop-relaunch-design.md` | Approved behavior and platform boundaries |
| `src-tauri/src/apps/launcher.rs` | Add desktop relaunch helpers, macOS quit/wait logic, and unit tests |

---

### Task 1: Add failing tests for desktop relaunch boundaries

**Files:**
- Modify: `src-tauri/src/apps/launcher.rs`
- Test: `src-tauri/src/apps/launcher.rs`

- [ ] **Step 1: Add helper-focused unit tests at the bottom of `launcher.rs`**

Append these tests inside the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn test_desktop_app_name_for_supported_desktop_apps() {
        assert_eq!(
            desktop_app_name(&AppType::CodexDesktop),
            Some("Codex")
        );
        assert_eq!(
            desktop_app_name(&AppType::ClaudeDesktop),
            Some("Claude")
        );
    }

    #[test]
    fn test_desktop_app_name_for_cli_apps_returns_none() {
        assert_eq!(desktop_app_name(&AppType::CodexCli), None);
        assert_eq!(desktop_app_name(&AppType::ClaudeCli), None);
    }

    #[test]
    fn test_is_desktop_relaunch_target_only_matches_desktop_apps() {
        assert!(!is_desktop_relaunch_target(&AppType::CodexCli));
        assert!(is_desktop_relaunch_target(&AppType::CodexDesktop));
        assert!(!is_desktop_relaunch_target(&AppType::ClaudeCli));
        assert!(is_desktop_relaunch_target(&AppType::ClaudeDesktop));
    }
```

- [ ] **Step 2: Run the focused test target to confirm the new tests fail**

Run:

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy/src-tauri && cargo test apps::launcher::tests::test_desktop_app_name_for_supported_desktop_apps -- --nocapture
```

Expected: FAIL with unresolved function errors for `desktop_app_name` and `is_desktop_relaunch_target`.

- [ ] **Step 3: Commit the failing-test checkpoint**

```bash
git add src-tauri/src/apps/launcher.rs
git commit -m "test(apps): cover desktop relaunch helpers"
```

---

### Task 2: Implement desktop relaunch helpers and macOS restart flow

**Files:**
- Modify: `src-tauri/src/apps/launcher.rs`
- Test: `src-tauri/src/apps/launcher.rs`

- [ ] **Step 1: Add shared helper functions above `launch()`**

Insert these helpers after `resolve_install_path()` and before `launch()`:

```rust
fn is_desktop_relaunch_target(app_type: &AppType) -> bool {
    matches!(app_type, AppType::CodexDesktop | AppType::ClaudeDesktop)
}

fn desktop_app_name(app_type: &AppType) -> Option<&'static str> {
    match app_type {
        AppType::CodexDesktop => Some("Codex"),
        AppType::ClaudeDesktop => Some("Claude"),
        _ => None,
    }
}
```

- [ ] **Step 2: Change `launch()` so desktop apps pass `app_type` into the desktop launcher**

Replace the desktop branch:

```rust
pub async fn launch(
    app_type: &AppType,
    install_path: &str,
    work_dir: Option<&str>,
) -> Result<(), String> {
    if app_type.is_cli() {
        launch_cli(install_path, work_dir).await
    } else {
        launch_desktop(app_type, install_path).await
    }
}
```

- [ ] **Step 3: Replace the existing `launch_desktop()` with a restart-aware version**

Replace the current `launch_desktop(install_path: &str)` function with this implementation:

```rust
async fn launch_desktop(app_type: &AppType, install_path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if is_desktop_relaunch_target(app_type) {
            if let Some(app_name) = desktop_app_name(app_type) {
                let running = is_macos_app_running(app_name).await.unwrap_or(false);
                if running {
                    quit_macos_app(app_name).await?;
                    wait_for_macos_app_exit(app_name).await?;
                }
            }
        }

        tracing::info!("Launching macOS desktop app at {}", install_path);
        Command::new("open")
            .arg(install_path)
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", install_path, e))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        let _ = app_type;
        tracing::info!("Launching Linux desktop app at {}", install_path);
        Command::new("xdg-open")
            .arg(install_path)
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", install_path, e))?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let _ = app_type;
        tracing::info!("Launching Windows desktop app at {}", install_path);
        Command::new("cmd")
            .args(["/C", "start", "", install_path])
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", install_path, e))?;
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = (app_type, install_path);
        Err("Unsupported platform for desktop app launch".to_string())
    }
}
```

- [ ] **Step 4: Add macOS helper functions directly above `launch_desktop()`**

Insert these functions:

```rust
#[cfg(target_os = "macos")]
async fn is_macos_app_running(app_name: &str) -> Result<bool, String> {
    let output = Command::new("pgrep")
        .args(["-x", app_name])
        .output()
        .await
        .map_err(|e| format!("Failed to check {} process: {}", app_name, e))?;

    Ok(output.status.success())
}

#[cfg(target_os = "macos")]
async fn quit_macos_app(app_name: &str) -> Result<(), String> {
    Command::new("osascript")
        .args(["-e", &format!("tell application \"{}\" to quit", app_name)])
        .output()
        .await
        .map_err(|e| format!("Failed to quit {}: {}", app_name, e))
        .and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Failed to quit {}: {}",
                    app_name,
                    String::from_utf8_lossy(&output.stderr).trim()
                ))
            }
        })
}

#[cfg(target_os = "macos")]
async fn wait_for_macos_app_exit(app_name: &str) -> Result<(), String> {
    const MAX_ATTEMPTS: usize = 20;
    const SLEEP_MS: u64 = 300;

    for _ in 0..MAX_ATTEMPTS {
        if !is_macos_app_running(app_name).await.unwrap_or(false) {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(SLEEP_MS)).await;
    }

    Err(format!("{} did not exit in time", app_name))
}
```

- [ ] **Step 5: Run the targeted unit tests**

Run:

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy/src-tauri && cargo test apps::launcher::tests -- --nocapture
```

Expected: PASS for all `apps::launcher::tests`.

- [ ] **Step 6: Commit the minimal implementation**

```bash
git add src-tauri/src/apps/launcher.rs
git commit -m "feat(apps): relaunch desktop apps on open"
```

---

### Task 3: Format and run regression verification

**Files:**
- Modify: `src-tauri/src/apps/launcher.rs`

- [ ] **Step 1: Format the Rust file**

Run:

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy/src-tauri && cargo fmt
```

Expected: command succeeds with no output or only formatting changes.

- [ ] **Step 2: Run a focused cargo check**

Run:

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy/src-tauri && cargo check
```

Expected: PASS with no compile errors.

- [ ] **Step 3: Re-run the launcher test module after formatting**

Run:

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy/src-tauri && cargo test apps::launcher::tests
```

Expected: PASS.

- [ ] **Step 4: Review the final diff**

Run:

```bash
cd /Users/mrhua/projects/aieditor/ai-proxy && git diff -- src-tauri/src/apps/launcher.rs
```

Expected: diff only shows helper additions, the updated desktop branch, and unit tests for the new behavior boundaries.

- [ ] **Step 5: Commit the verification pass**

```bash
git add src-tauri/src/apps/launcher.rs
git commit -m "chore(apps): verify desktop relaunch flow"
```

---

## Self-Review

- Spec coverage: only desktop apps restart; CLI remains unchanged; frontend/API stay unchanged; macOS gets the full quit/wait/start flow; Windows/Linux remain direct-launch.
- Placeholder scan: no `TODO`, `TBD`, or vague “handle later” language remains.
- Type consistency: helper names are consistent across tests and implementation: `is_desktop_relaunch_target`, `desktop_app_name`, `is_macos_app_running`, `quit_macos_app`, `wait_for_macos_app_exit`, `launch_desktop`.
