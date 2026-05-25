# computer-use — Claude Code Skill

A Claude Code skill that enables Claude to control your Mac: take screenshots, move the mouse, click, type text, and open apps — using only native macOS tools, no Docker, no VM.

[中文说明](#中文说明) · [Install](#install) · [Usage](#usage) · [How it works](#how-it-works)

---

## Install

```bash
claude skill install oil-oil/computer-use-skill
```

Or manually copy `SKILL.md` into `~/.claude/skills/computer-use/SKILL.md`.

## Prerequisites

**1. Install cliclick**
```bash
brew install cliclick
```

**2. Grant permissions** (System Settings → Privacy & Security)

| Permission | Required for | Grant to |
|------------|-------------|----------|
| Screen Recording | Taking screenshots | Terminal / iTerm2 |
| Accessibility | Mouse clicks, keyboard input | Terminal / iTerm2 |

## Usage

Once installed, just ask Claude naturally:

- "帮我打开 Safari 搜索 wolfcha"
- "截个屏看看现在屏幕上是什么"
- "帮我点击屏幕上的确认按钮"
- "Open Safari and search for wolfcha"
- "Take a screenshot and tell me what's on screen"
- "Click the confirm button on screen"

## How it works

No MCP server needed. This skill uses three native macOS tools:

| Tool | Purpose |
|------|---------|
| `screencapture` | Screenshot (built-in) |
| `cliclick` | Mouse move / click (via Homebrew) |
| `osascript` | Keyboard input, app activation (built-in) |

The core loop is: **screenshot → analyze → act → screenshot to confirm**.

Key insight: always use `tell process "AppName"` inside `System Events` when sending keystrokes — not bare `keystroke` — otherwise input goes to the wrong focused app.

```applescript
tell application "System Events"
    tell process "Safari"   -- ← specify the process, not just System Events
        keystroke "hello"
    end tell
end tell
```

---

## 中文说明

这是一个 [Claude Code](https://claude.ai/code) skill，让 Claude 可以直接操控你的 Mac：截图、移动鼠标、点击、输入文字、打开应用。

**不需要 Docker，不需要虚拟机**，只用 macOS 内置工具 + `cliclick`。

### 为什么不用 computer-use MCP？

`computer-use-mcp` 等 MCP server 的工具无法在 Claude Code 中调用（Claude Code 出于安全考虑过滤了桌面控制类工具）。这个 skill 用 Bash 工具直接调用 macOS 原生命令绕过了这个限制，在 Claude Code 里实现了完整的 computer use 能力。

### 安装

```bash
brew install cliclick
```

将 `SKILL.md` 复制到 `~/.claude/skills/computer-use/SKILL.md`，重启 Claude Code 即生效。

### 使用

直接用自然语言告诉 Claude：

- "帮我打开 Safari 搜索 wolfcha"
- "截个屏看看现在屏幕上有什么"
- "点击屏幕右上角的关闭按钮"

---

## License

MIT
