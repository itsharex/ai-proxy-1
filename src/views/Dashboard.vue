<template>
  <n-space vertical size="large">
    <n-grid :cols="4" :x-gap="16" :y-gap="16" responsive="screen" item-responsive>
      <n-gi span="4 m:1">
        <n-card>
          <n-statistic label="供应商数量">
            <template #prefix>
              <n-icon :component="ServerOutline" />
            </template>
            {{ providerCount }}
          </n-statistic>
        </n-card>
      </n-gi>
      <n-gi span="4 m:1">
        <n-card>
          <n-statistic label="模型数量">
            <template #prefix>
              <n-icon :component="GitBranchOutline" />
            </template>
            {{ routeCount }}
          </n-statistic>
        </n-card>
      </n-gi>
      <n-gi span="4 m:1">
        <n-card>
          <n-statistic label="今日请求数">
            <template #prefix>
              <n-icon :component="DocumentTextOutline" />
            </template>
            {{ formatNumber(todayRequests) }}
          </n-statistic>
        </n-card>
      </n-gi>
      <n-gi span="4 m:1">
        <n-card>
          <n-statistic label="今日 Token 用量">
            <template #prefix>
              <n-icon :component="PulseOutline" />
            </template>
            {{ formatNumber(todayTokens) }}
          </n-statistic>
        </n-card>
      </n-gi>
    </n-grid>

    <n-card title="代理状态">
      <n-space align="center" size="large">
        <n-tag :type="serverRunning ? 'success' : 'error'" size="medium" round>
          {{ serverRunning ? '运行中' : '已停止' }}
        </n-tag>
        <n-text v-if="serverRunning">
          代理服务器运行在
          <n-text code>127.0.0.1:{{ proxyPort }}</n-text>
        </n-text>
      </n-space>
    </n-card>

    <n-card title="最近请求">
      <n-data-table
        v-if="recentLogs.length > 0"
        :columns="logColumns"
        :data="recentLogs"
        :bordered="false"
        size="small"
      />
      <n-empty v-else description="暂无请求记录" />
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, h, onMounted, computed } from 'vue'
import { NTag } from 'naive-ui'
import { api, apiState } from '../api'
import type { Provider } from '../types'
import {
  ServerOutline,
  GitBranchOutline,
  DocumentTextOutline,
  PulseOutline,
} from '@vicons/ionicons5'

const serverRunning = computed(() => apiState.initialized)
const proxyPort = computed(() => apiState.proxyPort)
const providerCount = ref(0)
const routeCount = ref(0)
const todayRequests = ref(0)
const todayTokens = ref(0)
const recentLogs = ref<Record<string, unknown>[]>([])

interface UsageResponse {
  stats: Array<{
    model: string
    provider_name: string
    total_tokens: number
    request_count: number
  }>
  total_requests: number
}

interface LogsResponse {
  logs: Array<Record<string, unknown>>
  total: number
}

onMounted(async () => {
  try {
    const providers = await api<Provider[]>('/api/providers')
    providerCount.value = providers.length
    const totalModels = providers.reduce((sum, p) => sum + p.models.length, 0)
    routeCount.value = totalModels
  } catch {
    // silently fail
  }

  try {
    const usage = await api<UsageResponse>('/api/usage?days=1')
    todayRequests.value = usage.total_requests
    todayTokens.value = usage.stats.reduce((sum, s) => sum + s.total_tokens, 0)
  } catch {
    // silently fail
  }

  try {
    const result = await api<LogsResponse>('/api/logs?page=1&limit=5')
    recentLogs.value = result.logs
  } catch {
    // silently fail
  }
})

function formatUtcTime(utcStr: string): string {
  const date = new Date(utcStr + 'Z')
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }).replace(/\//g, '-')
}

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return String(n)
}

const logColumns = [
  {
    title: '时间',
    key: 'created_at',
    width: 180,
    render: (row: Record<string, unknown>) => formatUtcTime(row.created_at as string),
  },
  { title: '模型', key: 'model', width: 160 },
  { title: '供应商', key: 'provider_name', width: 120 },
  {
    title: '状态',
    key: 'status_code',
    width: 80,
    render: (row: Record<string, unknown>) => {
      const code = row.status_code as number
      return h(NTag, { size: 'small', type: code < 400 ? 'success' : 'error' }, () => String(code))
    },
  },
  { title: '耗时', key: 'duration_ms', width: 100, render: (row: Record<string, unknown>) => `${((row.duration_ms as number) / 1000).toFixed(1)}s` },
]
</script>
