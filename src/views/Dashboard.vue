<template>
  <n-space vertical size="large">
    <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 14px;">
      <div class="stat-tile">
        <div class="stat-tile-top" style="background: var(--accent);" />
        <div class="stat-tile-value tabular-nums">{{ providerCount }}</div>
        <div class="stat-tile-label">供应商数量</div>
      </div>
      <div class="stat-tile">
        <div class="stat-tile-top" style="background: var(--info);" />
        <div class="stat-tile-value tabular-nums">{{ routeCount }}</div>
        <div class="stat-tile-label">模型数量</div>
      </div>
      <div class="stat-tile">
        <div class="stat-tile-top" style="background: var(--success);" />
        <div class="stat-tile-value tabular-nums">{{ formatNumber(todayRequests) }}</div>
        <div class="stat-tile-label">今日请求数</div>
      </div>
      <div class="stat-tile">
        <div class="stat-tile-top" style="background: var(--warning);" />
        <div class="stat-tile-value tabular-nums">{{ formatNumber(todayTokens) }}</div>
        <div class="stat-tile-label">今日 Token 用量</div>
      </div>
    </div>

    <n-card>
      <template #header>
        <n-space justify="space-between" align="center">
          <n-text strong>用量趋势</n-text>
          <n-radio-group v-model:value="timeRange" size="small" @update:value="fetchTrend">
            <n-radio-button value="today">今日</n-radio-button>
            <n-radio-button value="week">近7天</n-radio-button>
            <n-radio-button value="month">近30天</n-radio-button>
          </n-radio-group>
        </n-space>
      </template>
      <div ref="chartRef" style="height: 300px" />
    </n-card>

    <n-card>
      <template #header>
        <n-text strong>代理状态</n-text>
      </template>
      <n-space align="center" size="large">
        <n-space align="center" size="small" :style="{ color: serverRunning ? 'var(--success)' : 'var(--error)' }">
          <span class="status-dot" :class="serverRunning ? 'running' : 'stopped'" />
          <span style="font-weight: 600">{{ serverRunning ? '运行中' : '已停止' }}</span>
        </n-space>
        <n-text v-if="serverRunning" style="color: var(--text-2); font-family: var(--font-mono); font-size: 13px;">
          127.0.0.1:{{ proxyPort }}
        </n-text>
      </n-space>
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick } from 'vue'
import { api, apiState } from '../api'
import type { Provider } from '../types'
import * as echarts from 'echarts'
import { getEchartsTheme } from '../theme'
import { useTheme } from '../theme/use-theme'

const { isDark } = useTheme()
const serverRunning = computed(() => apiState.initialized)
const proxyPort = computed(() => apiState.proxyPort)
const providerCount = ref(0)
const routeCount = ref(0)
const todayRequests = ref(0)
const todayTokens = ref(0)
const timeRange = ref<'today' | 'week' | 'month'>('today')
const chartRef = ref<HTMLElement | null>(null)
let chart: echarts.ECharts | null = null

interface UsageResponse {
  stats: Array<{
    model: string
    provider_name: string
    total_tokens: number
    request_count: number
  }>
  total_requests: number
}

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return String(n)
}

function getDays(range: 'today' | 'week' | 'month'): number {
  switch (range) {
    case 'today': return 1
    case 'week': return 7
    case 'month': return 30
  }
}

function buildChartOptions(data: Array<{ date: string; model: string; total_tokens: number }>) {
  const theme = getEchartsTheme(isDark.value)

  if (data.length === 0) {
    return {
      backgroundColor: 'transparent',
      tooltip: { trigger: 'axis' },
      legend: { textStyle: { color: theme.text } },
      grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
      xAxis: { type: 'category', boundaryGap: false, data: [], axisLine: { lineStyle: { color: theme.border } }, axisLabel: { color: theme.text } },
      yAxis: { type: 'value', name: 'Tokens', axisLine: { lineStyle: { color: theme.border } }, splitLine: { lineStyle: { color: theme.border } }, axisLabel: { color: theme.text }, nameTextStyle: { color: theme.text } },
      series: [],
    }
  }

  const dateSet = new Set<string>()
  const modelSet = new Set<string>()
  data.forEach(item => {
    dateSet.add(item.date)
    modelSet.add(item.model)
  })
  const dates = Array.from(dateSet).sort()
  const models = Array.from(modelSet)

  const lookup = new Map<string, number>()
  data.forEach(item => {
    lookup.set(`${item.date}|${item.model}`, item.total_tokens)
  })

  return {
    backgroundColor: 'transparent',
    tooltip: { trigger: 'axis', backgroundColor: theme.bg, borderColor: theme.border, textStyle: { color: theme.text } },
    legend: { data: models, textStyle: { color: theme.text } },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: dates,
      axisLine: { lineStyle: { color: theme.border } },
      axisLabel: { color: theme.text, fontSize: 11 },
    },
    yAxis: {
      type: 'value',
      name: 'Tokens',
      axisLine: { lineStyle: { color: theme.border } },
      splitLine: { lineStyle: { color: theme.border } },
      axisLabel: { color: theme.text, fontSize: 11 },
      nameTextStyle: { color: theme.text },
    },
    series: models.map((model, i) => ({
      name: model,
      type: 'line',
      smooth: true,
      symbol: 'none',
      lineStyle: { width: 2, color: theme.palette[i % theme.palette.length] },
      areaStyle: { opacity: 0.08, color: theme.palette[i % theme.palette.length] },
      data: dates.map(date => lookup.get(`${date}|${model}`) ?? 0),
    })),
  }
}

async function fetchTrend() {
  try {
    const days = getDays(timeRange.value)
    const data = await api<Array<{ date: string; model: string; total_tokens: number }>>(`/api/usage/trend?days=${days}`)
    await nextTick()
    if (chart) {
      chart.setOption(buildChartOptions(data), true)
    }
  } catch {
    // silently fail
  }
}

function handleResize() {
  chart?.resize()
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

  await nextTick()
  if (chartRef.value) {
    chart = echarts.init(chartRef.value)
  }
  fetchTrend()
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  chart?.dispose()
})
</script>
