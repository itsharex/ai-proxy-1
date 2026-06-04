# AI Proxy 服务化部署指南

## 快速开始

### Docker Compose 部署

```bash
# 1. 复制环境变量模板
cp .env.example .env
# 编辑 .env 设置密码和密钥

# 2. 启动服务
docker-compose up -d

# 3. 查看日志（首次启动会打印生成的管理员密码）
docker-compose logs -f

# 4. 访问服务
# API: http://localhost:7860
# 默认管理员: admin / (首次启动时自动生成的密码)
```

### 二进制部署

```bash
# 编译
cd src-tauri
cargo build --release --features server --no-default-features

# 运行
./target/release/ai-proxy-server \
  --host 0.0.0.0 \
  --port 7860 \
  --data-dir /var/lib/ai-proxy
```

## 编译模式

| 模式 | 编译命令 |
|---|---|
| **桌面版** | `cargo build --release` |
| **服务版** | `cargo build --release --features server --no-default-features` |

## 命令行参数

```
ai-proxy-server [OPTIONS]

Options:
  -h, --host <HOST>              监听地址 [default: 0.0.0.0]
  -p, --port <PORT>              监听端口 [default: 7860]
  -d, --data-dir <DATA_DIR>      数据目录 [default: /var/lib/ai-proxy]
      --admin-password <PASSWORD>  初始管理员密码
      --static-dir <DIR>         前端静态文件目录
```

## 环境变量

| 变量名 | 说明 | 必需 |
|---|---|---|
| `AI_PROXY_MASTER_KEY` | 数据库加密主密钥，多实例共享数据库时必须一致 | 是 |
| `AI_PROXY_ADMIN_PASSWORD` | 初始管理员密码 | 否（首次启动随机生成）|
| `AI_PROXY_JWT_SECRET` | JWT 签名密钥 | 是 |
| `AI_PROXY_DATA_DIR` | 数据目录（也可通过 --data-dir 指定）| 否 |
| `AI_PROXY_STATIC_DIR` | 前端静态文件目录 | 否 |

## API 认证

### 登录获取 Token

```bash
curl -X POST http://localhost:7860/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your-password"}'
```

响应：

```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_in": 86400,
    "user": {
      "id": "...",
      "username": "admin",
      "role": "admin"
    }
  }
}
```

### 使用 Token 访问管理 API

```bash
curl -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
  http://localhost:7860/api/providers
```

### 代理 API 认证

代理端点 (`/v1/*`) 使用 API Key 认证（通过 Web UI 配置）：

```bash
curl -H "Authorization: Bearer your-proxy-api-key" \
  http://localhost:7860/v1/chat/completions
```

## 架构差异

| 功能 | 桌面版 | 服务版 |
|---|---|---|
| 系统托盘 | ✅ | ❌ |
| 窗口管理 | ✅ | ❌ |
| 自动启动 | ✅ | 用 systemd/Docker |
| 应用更新检查 | ✅ | ❌ |
| App 启动器 | ✅ | ❌ |
| MCP 导入/导出 | ✅ | ❌ |
| 日志实时推送 | Tauri Emitter | WebSocket |
| 数据库路径 | Tauri API | 环境变量/参数 |
| 管理端认证 | 不需要 | JWT 登录 |
