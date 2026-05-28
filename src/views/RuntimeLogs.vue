<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed } from 'vue'
import { NTag, NButton, NButtonGroup, NSpace, NSwitch, NIcon, NEmpty } from 'naive-ui'
import { PauseOutline, PlayOutline, TrashOutline } from '@vicons/ionicons5'
import { api, getBaseUrl } from '../api'

interface LogEntry {
  timestamp: string
  level: string
  target: string
  message: string
}

const logs = ref<LogEntry[]>([])
const levelFilter = ref<string>('INFO')
const paused = ref(false)
const autoScroll = ref(true)
const logContainer = ref<HTMLElement | null>(null)
let ws: WebSocket | null = null

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

function scrollToBottom() {
  if (!autoScroll.value || !logContainer.value) return
  nextTick(() => {
    logContainer.value!.scrollTop = logContainer.value!.scrollHeight
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
      const entry: LogEntry = JSON.parse(event.data)
      logs.value.push(entry)
      if (logs.value.length > 5000) {
        logs.value = logs.value.slice(-4000)
      }
      scrollToBottom()
    } catch {}
  }

  ws.onclose = () => {
    setTimeout(connectWs, 3000)
  }
}

onMounted(async () => {
  try {
    const history = await api<LogEntry[]>('/api/runtime-logs')
    logs.value = history
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
    <n-space align="center" justify="space-between" style="margin-bottom: 12px;">
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

    <div
      ref="logContainer"
      style="flex: 1; overflow-y: auto; background: #1e1e1e; border-radius: 4px; padding: 8px; font-family: monospace; font-size: 12px; line-height: 1.6;"
    >
      <div v-if="filteredLogs.length === 0" style="display: flex; justify-content: center; align-items: center; height: 100%;">
        <n-empty description="暂无日志" />
      </div>
      <div
        v-for="(log, i) in filteredLogs"
        :key="i"
        style="display: flex; gap: 8px; padding: 1px 0;"
      >
        <n-tag :type="levelColor(log.level)" size="small" :bordered="false" style="min-width: 56px; justify-content: center; font-size: 11px;">
          {{ log.level }}
        </n-tag>
        <span style="color: #888; white-space: nowrap;">{{ log.timestamp.replace('T', ' ').replace(/\+.*/, '') }}</span>
        <span style="color: #6a9fb5; white-space: nowrap;">{{ log.target }}</span>
        <span :style="{ color: log.level === 'ERROR' ? '#f44' : log.level === 'WARN' ? '#ffa500' : '#d4d4d4', wordBreak: 'break-all' }">{{ log.message }}</span>
      </div>
    </div>
  </div>
</template>
