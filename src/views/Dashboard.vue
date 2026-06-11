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
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick } from 'vue'
import { api, apiState } from '../api'
import type { Provider } from '../types'
import * as echarts from 'echarts'
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
  if (data.length === 0) {
    return {
      tooltip: { trigger: 'axis' },
      legend: {},
      grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
      xAxis: { type: 'category', boundaryGap: false, data: [] },
      yAxis: { type: 'value', name: 'Tokens' },
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
    tooltip: { trigger: 'axis' },
    legend: { data: models },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: dates,
    },
    yAxis: { type: 'value', name: 'Tokens' },
    series: models.map(model => ({
      name: model,
      type: 'line',
      smooth: true,
      areaStyle: { opacity: 0.15 },
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
