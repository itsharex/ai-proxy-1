<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed } from 'vue'
import { NTag, NButton, NButtonGroup, NSpace, NSwitch, NIcon, NEmpty } from 'naive-ui'
import { PauseOutline, PlayOutline, TrashOutline } from '@vicons/ionicons5'
import { api, getBaseUrl } from '../api'

const MAX_DISPLAY_LEN = 2000

interface LogEntry {
  timestamp: string
  level: string
  target: string
  message: string
  _uid: string
  displayMessage: string
}

const logs = ref<LogEntry[]>([])
const levelFilter = ref<string>('INFO')
const paused = ref(false)
const autoScroll = ref(true)
const scroller = ref<HTMLElement | null>(null)
let ws: WebSocket | null = null
let uidCounter = 0

const levels = ['INFO', 'WARN', 'ERROR']

const filteredLogs = computed(() => {
  if (levelFilter.value === 'ALL') return logs.value
  return logs.value.filter(l => l.level === levelFilter.value)
})

const levelColor = (level: string) => {
  switch (level) {
    case 'ERROR': return 'error'
    case 'WARN': return 'warning'
    case 'INFO': return 'info'
    default: return 'default'
  }
}

function processEntry(raw: any): LogEntry {
  const message = raw.message || ''
  const displayMessage = message.length > MAX_DISPLAY_LEN
    ? message.slice(0, MAX_DISPLAY_LEN) + `... (truncated, ${message.length} chars total)`
    : message
  return {
    timestamp: raw.timestamp || '',
    level: raw.level || 'INFO',
    target: raw.target || '',
    message,
    _uid: `log-${uidCounter++}`,
    displayMessage,
  }
}

function scrollToBottom() {
  if (!autoScroll.value || !scroller.value) return
  nextTick(() => {
    if (scroller.value) scroller.value.scrollTop = scroller.value.scrollHeight
  })
}

function clearLogs() {
  logs.value = []
}

function connectWs() {
  const base = getBaseUrl()
  if (!base) return

  const wsBase = base.replace(/^http/, 'ws')
  ws = new WebSocket(`${wsBase}/api/runtime-logs/stream`)

  ws.onmessage = (event) => {
    if (paused.value) return
    try {
      const entry = processEntry(JSON.parse(event.data))
      logs.value.push(entry)
      if (logs.value.length > 1000) {
        logs.value = logs.value.slice(-800)
      }
      scrollToBottom()
    } catch {}
  }

  ws.onclose = () => {
    setTimeout(connectWs, 3000)
  }
}

function formatTimestamp(ts: string): string {
  const d = new Date(ts)
  if (isNaN(d.getTime())) return ts.replace('T', ' ').replace(/[Z+].*/, '')
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

function messageColor(level: string): string {
  if (level === 'ERROR') return 'var(--error)'
  if (level === 'WARN') return 'var(--warning)'
  return 'var(--text-2)'
}

onMounted(async () => {
  try {
    const history = await api<any[]>('/api/runtime-logs')
    logs.value = history.map(processEntry)
    scrollToBottom()
  } catch {}
  connectWs()
})

onUnmounted(() => {
  if (ws) {
    ws.onclose = null
    ws.close()
    ws = null
  }
})
</script>

<template>
  <div style="display: flex; flex-direction: column; height: calc(100vh - 100px);">
    <div class="terminal-toolbar">
      <n-space align="center" justify="space-between">
        <n-space align="center">
          <n-button-group size="small">
            <n-button
              v-for="lvl in levels"
              :key="lvl"
              :type="levelFilter === lvl ? 'primary' : 'default'"
              @click="levelFilter = lvl"
            >
              {{ lvl }}
            </n-button>
          </n-button-group>
          <n-button size="small" @click="paused = !paused">
            <template #icon>
              <n-icon><component :is="paused ? PlayOutline : PauseOutline" /></n-icon>
            </template>
            {{ paused ? '恢复' : '暂停' }}
          </n-button>
        </n-space>
        <n-space align="center">
          <n-switch v-model:value="autoScroll" size="small">
            <template #checked>自动滚动</template>
            <template #unchecked>自动滚动</template>
          </n-switch>
          <n-button size="small" type="error" @click="clearLogs">
            <template #icon>
              <n-icon><TrashOutline /></n-icon>
            </template>
            清空
          </n-button>
        </n-space>
      </n-space>
    </div>

    <div
      v-if="filteredLogs.length > 0"
      ref="scroller"
      class="terminal-bg log-scroller"
      style="flex: 1; padding: 8px 12px; overflow-y: auto;"
    >
      <div v-for="log in filteredLogs" :key="log._uid" style="display: flex; gap: 8px; padding: 1px 0;">
        <n-tag :type="levelColor(log.level)" size="small" :bordered="false" style="min-width: 56px; justify-content: center; font-size: 11px;">
          {{ log.level }}
        </n-tag>
        <span style="color: var(--text-3); white-space: nowrap;">{{ formatTimestamp(log.timestamp) }}</span>
        <span style="color: var(--accent); white-space: nowrap;">{{ log.target }}</span>
        <span :style="{ color: messageColor(log.level), wordBreak: 'break-word' }">{{ log.displayMessage }}</span>
      </div>
    </div>
    <div
      v-else
      class="terminal-bg"
      style="flex: 1; padding: 8px 12px;"
    >
      <div style="display: flex; justify-content: center; align-items: center; height: 100%;">
        <n-empty description="暂无日志" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.log-scroller {
  overflow-y: auto;
}
</style>
