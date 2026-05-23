<template>
  <n-space vertical size="large">
    <n-card>
      <template #header>
        <n-space justify="space-between" align="center">
          <n-text strong>用量统计</n-text>
          <n-radio-group v-model:value="timeRange" size="small" @update:value="handleRangeChange">
            <n-radio-button value="today">今日</n-radio-button>
            <n-radio-button value="week">本周</n-radio-button>
            <n-radio-button value="month">本月</n-radio-button>
          </n-radio-group>
        </n-space>
      </template>

      <n-grid :cols="2" :x-gap="24" :y-gap="24">
        <n-gi>
          <n-card title="Token 用量趋势" size="small">
            <div ref="lineChartRef" style="height: 360px" />
          </n-card>
        </n-gi>
        <n-gi>
          <n-card title="模型费用分布" size="small">
            <div ref="pieChartRef" style="height: 360px" />
          </n-card>
        </n-gi>
      </n-grid>
    </n-card>

    <n-card title="用量汇总">
      <n-data-table
        :columns="summaryColumns"
        :data="stats"
        :bordered="false"
        size="small"
      />
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import { api } from '../api'
import * as echarts from 'echarts'

interface UsageStat {
  model: string
  provider_name: string
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
  cost_estimate: number
  request_count: number
}

interface UsageResponse {
  stats: UsageStat[]
  total_cost: number
  total_requests: number
}

const timeRange = ref<'today' | 'week' | 'month'>('month')
const stats = ref<UsageStat[]>([])

const lineChartRef = ref<HTMLElement | null>(null)
const pieChartRef = ref<HTMLElement | null>(null)
let lineChart: echarts.ECharts | null = null
let pieChart: echarts.ECharts | null = null

const summaryColumns = [
  { title: '模型', key: 'model', width: 160 },
  { title: '供应商', key: 'provider_name', width: 140 },
  { title: '请求次数', key: 'request_count', width: 100 },
  {
    title: 'Prompt Tokens',
    key: 'prompt_tokens',
    width: 140,
    render: (row: UsageStat) => row.prompt_tokens.toLocaleString(),
  },
  {
    title: 'Completion Tokens',
    key: 'completion_tokens',
    width: 160,
    render: (row: UsageStat) => row.completion_tokens.toLocaleString(),
  },
  {
    title: '总 Tokens',
    key: 'total_tokens',
    width: 120,
    render: (row: UsageStat) => row.total_tokens.toLocaleString(),
  },
  {
    title: '费用',
    key: 'cost_estimate',
    width: 100,
    render: (row: UsageStat) => `$${row.cost_estimate.toFixed(4)}`,
  },
]

function getDaysFromRange(range: 'today' | 'week' | 'month'): number {
  switch (range) {
    case 'today': return 1
    case 'week': return 7
    case 'month': return 30
  }
}

function buildLineChartOptions(data: UsageStat[]) {
  const modelMap = new Map<string, { date: string; tokens: number }[]>()
  data.forEach((item) => {
    if (!modelMap.has(item.model)) {
      modelMap.set(item.model, [])
    }
  })

  const models = Array.from(modelMap.keys())
  const dates = data.length > 0
    ? [data[0].model]
    : []

  return {
    tooltip: { trigger: 'axis' },
    legend: { data: models },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: dates.length > 0 ? ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'] : [],
    },
    yAxis: { type: 'value', name: 'Tokens' },
    series: models.map((model) => ({
      name: model,
      type: 'line',
      smooth: true,
      data: data
        .filter((d) => d.model === model)
        .map((d) => d.total_tokens),
    })),
  }
}

function buildPieChartOptions(data: UsageStat[]) {
  const costData = data.map((item) => ({
    name: item.model,
    value: Number(item.cost_estimate.toFixed(4)),
  }))

  return {
    tooltip: { trigger: 'item', formatter: '{b}: ${c} ({d}%)' },
    legend: { orient: 'vertical', left: 'left' },
    series: [
      {
        type: 'pie',
        radius: ['40%', '70%'],
        avoidLabelOverlap: false,
        itemStyle: { borderRadius: 10, borderColor: '#fff', borderWidth: 2 },
        label: { show: false, position: 'center' },
        emphasis: {
          label: { show: true, fontSize: 16, fontWeight: 'bold' },
        },
        labelLine: { show: false },
        data: costData,
      },
    ],
  }
}

function updateCharts(data: UsageStat[]) {
  if (lineChart) {
    lineChart.setOption(buildLineChartOptions(data))
  }
  if (pieChart) {
    pieChart.setOption(buildPieChartOptions(data))
  }
}

async function fetchStats() {
  try {
    const days = getDaysFromRange(timeRange.value)
    const result = await api<UsageResponse>(`/api/usage?days=${days}`)
    stats.value = result.stats
    await nextTick()
    updateCharts(stats.value)
  } catch (error) {
    console.error('Failed to load usage stats:', error)
  }
}

function handleRangeChange() {
  fetchStats()
}

function handleResize() {
  lineChart?.resize()
  pieChart?.resize()
}

onMounted(async () => {
  await nextTick()
  if (lineChartRef.value) {
    lineChart = echarts.init(lineChartRef.value)
  }
  if (pieChartRef.value) {
    pieChart = echarts.init(pieChartRef.value)
  }
  await fetchStats()
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  lineChart?.dispose()
  pieChart?.dispose()
})
</script>
