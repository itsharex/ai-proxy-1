<template>
  <div class="statistics-view">
    <n-space style="margin-bottom: 16px" align="center">
      <n-button-group>
        <n-button :type="days === 7 ? 'primary' : 'default'" @click="days = 7">{{ t('statistics.days7') }}</n-button>
        <n-button :type="days === 30 ? 'primary' : 'default'" @click="days = 30">{{ t('statistics.days30') }}</n-button>
        <n-button :type="days === 90 ? 'primary' : 'default'" @click="days = 90">{{ t('statistics.days90') }}</n-button>
      </n-button-group>
    </n-space>

    <n-grid :x-gap="16" :y-gap="16" :cols="1">
      <n-gi>
        <n-card :title="t('statistics.tokenUsage')" size="small">
          <div style="height: 400px">
            <v-chart :option="chartOption" autoresize style="height: 100%" />
          </div>
        </n-card>
      </n-gi>
      <n-gi>
        <n-card :title="t('statistics.costBreakdown')" size="small">
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
import { useI18n } from 'vue-i18n'
import VChart from 'vue-echarts'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { BarChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import { invoke } from '@tauri-apps/api/core'
import type { UsageSummary } from '../types'

use([CanvasRenderer, BarChart, GridComponent, TooltipComponent, LegendComponent])

const { t } = useI18n()

const days = ref(30)
const usageData = ref<UsageSummary[]>([])

const costColumns: DataTableColumns<UsageSummary> = [
  { title: t('statistics.model'), key: 'model', width: 200 },
  { title: t('statistics.provider'), key: 'provider_name', width: 140 },
  { title: t('statistics.requests'), key: 'request_count', width: 100 },
  { title: t('statistics.promptTokens'), key: 'total_prompt_tokens', width: 140, render: (row) => row.total_prompt_tokens.toLocaleString() },
  { title: t('statistics.completionTokens'), key: 'total_completion_tokens', width: 160, render: (row) => row.total_completion_tokens.toLocaleString() },
  { title: t('statistics.totalTokens'), key: 'total_tokens', width: 130, render: (row) => row.total_tokens.toLocaleString() },
  { title: t('statistics.estCost'), key: 'total_cost', width: 120, render: (row) => `$${row.total_cost.toFixed(4)}` },
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
    legend: { data: [t('statistics.promptTokens'), t('statistics.completionTokens')] },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'category', data: models, axisLabel: { rotate: 30 } },
    yAxis: { type: 'value' },
    series: [
      { name: t('statistics.promptTokens'), type: 'bar', stack: 'tokens', data: promptData },
      { name: t('statistics.completionTokens'), type: 'bar', stack: 'tokens', data: completionData },
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
