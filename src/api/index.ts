import { invoke } from '@tauri-apps/api/core'

let baseUrl = ''
let proxyPort = 7860
let initialized = false

async function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

async function tryFetchHealth(url: string): Promise<boolean> {
  try {
    const res = await fetch(url)
    return res.ok
  } catch {
    return false
  }
}

export async function initApi(): Promise<void> {
  const configUrl = await invoke<string>('get_api_config')
  const parsedUrl = new URL(configUrl)
  proxyPort = parseInt(parsedUrl.port) || 7860

  for (let i = 0; i < 10; i++) {
    // Dev mode: relative URL through Vite proxy
    if (await tryFetchHealth('/health')) {
      baseUrl = ''
      initialized = true
      return
    }

    // Production mode: absolute URL
    if (await tryFetchHealth(`${configUrl}/health`)) {
      baseUrl = configUrl
      initialized = true
      return
    }

    if (i < 9) await sleep(500)
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
