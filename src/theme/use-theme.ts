import { ref, watch, computed } from 'vue'
import { darkTheme } from 'naive-ui'
import { lightThemeOverrides, darkThemeOverrides } from './index'

export type ThemeMode = 'light' | 'dark' | 'system'

const STORAGE_KEY = 'ai-proxy-theme'

function getStoredMode(): ThemeMode {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored === 'light' || stored === 'dark' || stored === 'system') return stored
  } catch {}
  return 'system'
}

function getSystemIsDark(): boolean {
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

const mode = ref<ThemeMode>(getStoredMode())
const systemDark = ref(getSystemIsDark())

if (typeof window !== 'undefined') {
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
    systemDark.value = e.matches
  })
}

const isDark = computed(() => {
  if (mode.value === 'system') return systemDark.value
  return mode.value === 'dark'
})

watch(isDark, (dark) => {
  document.documentElement.dataset.theme = dark ? 'dark' : 'light'
}, { immediate: true })

watch(mode, (m) => {
  try { localStorage.setItem(STORAGE_KEY, m) } catch {}
})

export function useTheme() {
  const naiveTheme = computed(() => isDark.value ? darkTheme : undefined)
  const themeOverrides = computed(() => isDark.value ? darkThemeOverrides : lightThemeOverrides)

  function setMode(m: ThemeMode) {
    mode.value = m
  }

  function toggleTheme() {
    if (mode.value === 'dark') mode.value = 'light'
    else if (mode.value === 'light') mode.value = 'dark'
    else mode.value = systemDark.value ? 'light' : 'dark'
  }

  return {
    isDark,
    mode,
    naiveTheme,
    themeOverrides,
    setMode,
    toggleTheme,
  }
}
