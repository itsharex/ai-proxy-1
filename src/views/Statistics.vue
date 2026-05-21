<template>
  <div class="statistics-view">
    <n-space style="margin-bottom: 16px" align="center">
      <n-button-group>
        <n-button :type="days === 7 ? 'primary' : 'default'" @click="days = 7">7 Days</n-button>
        <n-button :type="days === 30 ? 'primary' : 'default'" @click="days = 30">30 Days</n-button>
        <n-button :type="days === 90 ? 'primary' : 'default'" @click="days = 90">90 Days</n-button>
      </n-button-group>
    </n-space>

    <n-grid :x-gap="16" :y-gap="16" :cols="1">
      <n-gi>
        <n-card title="Token Usage by Model" size="small">
          <div style="height: 400px">
            <v-chart :option="chartOption" autoresize style="height: 100%" />
          </div>
        </n-card>
      </n-gi>
      <n-gi>
        <n-card title="Cost Breakdown" size="small">
          <n-data-table
            :columns="costColumns"
            :data="usageData"
            :bordered="false"
            size="small"
            :pagination="false"
          />
        </n-card>
      </n-gi>
    </n-grid>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { NGrid, NGi, NCard, NDataTable, NButtonGroup, NButton, NSpace, type DataTableColumns } from 'naive-ui'
import VChart from 'vue-echarts'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { BarChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import { invoke } from '@tauri-apps/api/core'
import type { UsageSummary } from '../types'

use([CanvasRenderer, BarChart, GridComponent, TooltipComponent, LegendComponent])

const days = ref(30)
const usageData = ref<UsageSummary[]>([])

const costColumns: DataTableColumns<UsageSummary> = [
  { title: 'Model', key: 'model', width: 200 },
  { title: 'Provider', key: 'provider_name', width: 140 },
  { title: 'Requests', key: 'request_count', width: 100 },
  { title: 'Prompt Tokens', key: 'total_prompt_tokens', width: 140, render: (row) => row.total_prompt_tokens.toLocaleString() },
  { title: 'Completion Tokens', key: 'total_completion_tokens', width: 160, render: (row) => row.total_completion_tokens.toLocaleString() },
  { title: 'Total Tokens', key: 'total_tokens', width: 130, render: (row) => row.total_tokens.toLocaleString() },
  { title: 'Est. Cost', key: 'total_cost', width: 120, render: (row) => `$${row.total_cost.toFixed(4)}` },
]

const chartOption = computed(() => {
  const models = [...new Set(usageData.value.map((d) => d.model))]
  const promptData = models.map((m) =>
    usageData.value.filter((d) => d.model === m).reduce((s, d) => s + d.total_prompt_tokens, 0)
  )
  const completionData = models.map((m) =>
    usageData.value.filter((d) => d.model === m).reduce((s, d) => s + d.total_completion_tokens, 0)
  )

  return {
    tooltip: { trigger: 'axis' },
    legend: { data: ['Prompt Tokens', 'Completion Tokens'] },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'category', data: models, axisLabel: { rotate: 30 } },
    yAxis: { type: 'value' },
    series: [
      { name: 'Prompt Tokens', type: 'bar', stack: 'tokens', data: promptData },
      { name: 'Completion Tokens', type: 'bar', stack: 'tokens', data: completionData },
    ],
  }
})

async function loadUsage() {
  try {
    usageData.value = await invoke<UsageSummary[]>('get_usage_stats', { days: days.value })
  } catch {
    usageData.value = []
  }
}

watch(days, loadUsage)
onMounted(loadUsage)
</script>
