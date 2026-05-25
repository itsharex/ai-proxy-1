# System Tray Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement system tray with close-to-tray behavior, proxy control menu, and left-click window restore.

**Architecture:** Enable `tray-icon` feature on Tauri crate. Build tray programmatically in Rust via `TrayIconBuilder`. Add graceful shutdown to axum server using `tokio::sync::watch`. Intercept window close events to hide instead of quit.

**Tech Stack:** Tauri 2 (Rust), `tray-icon` feature, `tokio::sync::watch` for server shutdown

---

### Task 1: Enable tray-icon feature

**Files:**
- Modify: `src-tauri/Cargo.toml:16`

- [ ] **Step 1: Add tray-icon feature to tauri dependency**

In `src-tauri/Cargo.toml`, change line 16 from:

```toml
tauri = { version = "2", features = [] }
```

to:

```toml
tauri = { version = "2", features = ["tray-icon"] }
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "feat: enable tray-icon feature for system tray support"
```

---

### Task 2: Add proxy control and graceful shutdown

**Files:**
- Modify: `src-tauri/src/server/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Modify start_server to accept shutdown signal**

Replace the `start_server` function in `src-tauri/src/server/mod.rs` with:

```rust
pub async fn start_server(host: &str, port: u16, mut shutdown_rx: tokio::sync::watch::Receiver<bool>) {
    let app = create_server(host, port).await;

    let addr = SocketAddr::new(
        host.parse().unwrap_or_else(|_| "127.0.0.1".parse().unwrap()),
        port,
    );

    info!("Starting HTTP server on {}", addr);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind to {}: {}", addr, e);
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_rx.changed().await.ok();
        })
        .await
    {
        tracing::error!("HTTP server error: {}", e);
    }

    info!("HTTP server stopped");
}
```

- [ ] **Step 2: Add ProxyControl state to lib.rs**

Add the following after the `APP_RUNTIME` static in `src-tauri/src/lib.rs` (after line 16):

```rust
struct ProxyControl {
    running: bool,
    port: u16,
    host: String,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

static PROXY_CONTROL: Lazy<Mutex<ProxyControl>> = Lazy::new(|| {
    Mutex::new(ProxyControl {
        running: false,
        port: 7860,
        host: "127.0.0.1".to_string(),
        shutdown_tx: None,
    })
});
```

- [ ] **Step 3: Add start_proxy and stop_proxy functions to lib.rs**

Add these functions before the `run()` function in `src-tauri/src/lib.rs`:

```rust
fn start_proxy() {
    {
        let ctrl = PROXY_CONTROL.lock().unwrap();
        if ctrl.running {
            return;
        }
    }

    let handle = {
        let guard = APP_RUNTIME.lock().unwrap();
        guard.as_ref().expect("runtime not initialized").handle().clone()
    };

    let (host, port) = handle.block_on(async {
        let pool = db::get_pool().await;
        let host: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_host'")
            .fetch_one(pool)
            .await
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        let pool = db::get_pool().await;
        let port_str: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'http_port'")
            .fetch_one(pool)
            .await
            .unwrap_or_else(|_| "7860".to_string());
        (host, port_str.parse().unwrap_or(7860u16))
    });

    let (tx, rx) = tokio::sync::watch::channel(false);

    {
        let mut ctrl = PROXY_CONTROL.lock().unwrap();
        ctrl.running = true;
        ctrl.host = host.clone();
        ctrl.port = port;
        ctrl.shutdown_tx = Some(tx);
    }

    handle.spawn(async move {
        server::start_server(&host, port, rx).await;
        let mut ctrl = PROXY_CONTROL.lock().unwrap();
        ctrl.running = false;
    });
}

fn stop_proxy() {
    let mut ctrl = PROXY_CONTROL.lock().unwrap();
    if !ctrl.running {
        return;
    }
    if let Some(tx) = ctrl.shutdown_tx.take() {
        let _ = tx.send(true);
    }
    ctrl.running = false;
}
```

- [ ] **Step 4: Update the setup closure to use start_proxy**

Replace the spawn block and runtime storage in `setup` (lines 50-72) with:

```rust
            {
                let mut guard = APP_RUNTIME.lock().unwrap();
                *guard = Some(rt);
            }

            start_proxy();
```

The full `setup` closure should now be:

```rust
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");

            let db_path = app_data_dir.join("ai-proxy.db");
            let rt = tokio::runtime::Runtime::new().expect("failed to create runtime");
            rt.block_on(async {
                db::init::init_db(db_path.to_str().unwrap()).await
                    .expect("failed to initialize database");
            });

            {
                let mut guard = APP_RUNTIME.lock().unwrap();
                *guard = Some(rt);
            }

            start_proxy();

            Ok(())
        })
```

- [ ] **Step 5: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles without errors

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/server/mod.rs src-tauri/src/lib.rs
git commit -m "feat: add proxy control with graceful shutdown support"
```

---

### Task 3: Build tray icon with menu

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add tray imports**

Add these imports at the top of `src-tauri/src/lib.rs`:

```rust
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
```

- [ ] **Step 2: Build tray menu in setup closure**

Add the following code at the end of the `setup` closure, just before `Ok(())`:

```rust
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let status_text = {
                let ctrl = PROXY_CONTROL.lock().unwrap();
                if ctrl.running {
                    format!("Proxy :{} running", ctrl.port)
                } else {
                    "Proxy stopped".to_string()
                }
            };
            let status_item = MenuItem::with_id(app, "status", &status_text, false, None::<&str>)?;
            let toggle_text = {
                let ctrl = PROXY_CONTROL.lock().unwrap();
                if ctrl.running { "Stop Proxy" } else { "Start Proxy" }
            };
            let toggle_item = MenuItem::with_id(app, "toggle", toggle_text, true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_item, &status_item, &toggle_item, &separator, &quit_item])?;

            let status_for_handler = status_item.clone();
            let toggle_for_handler = toggle_item.clone();

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

            TrayIconBuilder::with_id(app, "main-tray")
                .icon(icon)
                .tooltip("AI Proxy")
                .menu(&menu)
                .menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "toggle" => {
                            let is_running = {
                                let ctrl = PROXY_CONTROL.lock().unwrap();
                                ctrl.running
                            };
                            if is_running {
                                stop_proxy();
                                let _ = toggle_for_handler.set_text("Start Proxy");
                                let _ = status_for_handler.set_text("Proxy stopped");
                            } else {
                                start_proxy();
                                let port = {
                                    let ctrl = PROXY_CONTROL.lock().unwrap();
                                    ctrl.port
                                };
                                let _ = toggle_for_handler.set_text("Stop Proxy");
                                let _ = status_for_handler.set_text(&format!("Proxy :{} running", port));
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;
```

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles without errors

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add system tray with menu and proxy control"
```

---

### Task 4: Add window close interception

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add on_window_event handler to builder**

Add `.on_window_event(...)` chain after the `.invoke_handler(...)` line and before `.run(...)` in `lib.rs`. The builder chain should become:

```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // ... existing setup code (tray, proxy, etc.) ...
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_api_config])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: intercept window close to hide instead of quit"
```

---

### Task 5: Build and verify

- [ ] **Step 1: Full build**

Run: `cd src-tauri && cargo build`
Expected: build succeeds

- [ ] **Step 2: Run in dev mode and verify**

Run: `cd /Users/mrhua/projects/aieditor/ai-proxy && pnpm tauri dev`

Verify manually:
1. App launches with system tray icon visible in menu bar
2. Right-click tray icon shows menu: Show Window, proxy status, Start/Stop Proxy, separator, Quit
3. Left-click tray icon shows and focuses the window
4. Click window close button (X) — window hides, app stays running in tray
5. Left-click tray icon — window reappears
6. Right-click tray → Stop Proxy — proxy status updates, toggle text changes
7. Right-click tray → Start Proxy — proxy restarts
8. Right-click tray → Quit — app exits completely
