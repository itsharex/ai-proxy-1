import { reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const DEFAULT_PROXY_PORT = 7860

export const apiState = reactive({
  baseUrl: '',
  proxyPort: DEFAULT_PROXY_PORT,
  connectHost: '127.0.0.1',
  initialized: false,
})

function normalizeApiConfigUrl(configUrl: string): { baseUrl: string; port: number; host: string } {
  const normalizedInput = /^[a-zA-Z][a-zA-Z\d+\-.]*:\/\//.test(configUrl) ? configUrl : `http://${configUrl}`
  const url = new URL(normalizedInput)
  const host = url.hostname === '0.0.0.0' ? '127.0.0.1' : url.hostname
  const port = parseInt(url.port, 10) || DEFAULT_PROXY_PORT
  const baseUrl = `${url.protocol}//${host}:${port}`
  return { baseUrl, port, host }
}

function buildApiUrl(path: string): string {
  return apiState.baseUrl ? `${apiState.baseUrl}${path}` : path
}

function setApiState(baseUrl: string, port: number, host: string, initialized: boolean): void {
  apiState.baseUrl = baseUrl
  apiState.proxyPort = port
  apiState.connectHost = host
  apiState.initialized = initialized
}

async function tryFetchHealth(url: string): Promise<boolean> {
  try {
    const res = await fetch(url)
    return res.ok
  } catch {
    return false
  }
}

async function loadTauriConfig(): Promise<{ baseUrl: string; port: number; host: string } | null> {
  try {
    const configUrl = await invoke<string>('get_api_config')
    return normalizeApiConfigUrl(configUrl)
  } catch {
    return null
  }
}

export async function initApi(force = false): Promise<void> {
  if (apiState.initialized && !force) {
    return
  }

  if (force) {
    setApiState('', apiState.proxyPort || DEFAULT_PROXY_PORT, apiState.connectHost || '127.0.0.1', false)
  }

  // Try relative URL first (works through Vite proxy in dev mode)
  for (let i = 0; i < 10; i++) {
    if (await tryFetchHealth('/health')) {
      const config = await loadTauriConfig()
      if (config) {
        setApiState('', config.port, config.host, true)
      } else {
        setApiState('', apiState.proxyPort || DEFAULT_PROXY_PORT, apiState.connectHost || '127.0.0.1', true)
      }
      return
    }

    // Fallback: try absolute URL via Tauri config
    const config = await loadTauriConfig()
    if (config && await tryFetchHealth(buildApiUrlFromBase(config.baseUrl, '/health'))) {
      setApiState(config.baseUrl, config.port, config.host, true)
      return
    }

    await new Promise(r => setTimeout(r, 500))
  }

  throw new Error('Proxy server not reachable')
}

function buildApiUrlFromBase(baseUrl: string, path: string): string {
  return `${baseUrl}${path}`
}

export async function refreshApiConfig(): Promise<void> {
  await initApi(true)
}

async function ensureInitialized(): Promise<void> {
  if (apiState.initialized) return
  await initApi()
}

export async function api<T>(path: string, options?: RequestInit): Promise<T> {
  await ensureInitialized()

  const res = await fetch(buildApiUrl(path), {
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
  return apiState.baseUrl
}

export function getProxyPort(): number {
  return apiState.proxyPort
}

export function isInitialized(): boolean {
  return apiState.initialized
}
