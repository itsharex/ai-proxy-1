import { invoke } from '@tauri-apps/api/core'

let baseUrl = ''
let proxyPort = 7860
let initialized = false

async function tryFetchHealth(url: string): Promise<boolean> {
  try {
    const res = await fetch(url)
    return res.ok
  } catch {
    return false
  }
}

export async function initApi(): Promise<void> {
  // Try relative URL first (works through Vite proxy in dev mode)
  for (let i = 0; i < 10; i++) {
    if (await tryFetchHealth('/health')) {
      baseUrl = ''
      initialized = true
      // Try to get port from Tauri backend (non-blocking)
      try {
        const configUrl = await invoke<string>('get_api_config')
        proxyPort = parseInt(new URL(configUrl).port) || 7860
      } catch { /* ignore */ }
      return
    }

    // Fallback: try absolute URL via Tauri config
    try {
      const configUrl = await invoke<string>('get_api_config')
      if (await tryFetchHealth(`${configUrl}/health`)) {
        baseUrl = configUrl
        proxyPort = parseInt(new URL(configUrl).port) || 7860
        initialized = true
        return
      }
    } catch { /* invoke failed, skip */ }

    await new Promise(r => setTimeout(r, 500))
  }

  throw new Error('Proxy server not reachable')
}

async function ensureInitialized(): Promise<void> {
  if (initialized) return
  await initApi()
}

export async function api<T>(path: string, options?: RequestInit): Promise<T> {
  await ensureInitialized()

  const res = await fetch(`${baseUrl}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...options,
  })
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: `HTTP ${res.status}` }))
    throw new Error(body.error || `API error: ${res.status}`)
  }
  const body = await res.json()
  if (!body.success) throw new Error(body.error || 'Unknown error')
  return body.data as T
}

export function getBaseUrl(): string {
  return baseUrl
}

export function getProxyPort(): number {
  return proxyPort
}

export function isInitialized(): boolean {
  return initialized
}
