<template>
  <n-space vertical size="large">
    <n-card>
      <template #header>
        <n-text strong>请求日志</n-text>
      </template>
      <template #header-extra>
        <n-space align="center">
          <n-input
            v-model:value="searchQuery"
            placeholder="按模型名搜索"
            clearable
            style="width: 220px"
            @keyup.enter="handleQuery"
          />
          <n-date-picker
            v-model:value="dateRange"
            type="daterange"
            clearable
            style="width: 280px"
          />
          <n-button type="primary" @click="handleQuery">查询</n-button>
          <n-button type="error" @click="handleClearLogs">清除日志</n-button>
        </n-space>
      </template>
      <n-data-table
        :columns="columns"
        :data="logs"
        :loading="loading"
        :bordered="false"
        :pagination="pagination"
        :row-key="(row: RequestLog) => row.id"
        @update:page="handlePageChange"
        @update:page-size="handlePageSizeChange"
      />
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, h } from 'vue'
import { api } from '../api'
import { NTag, NSpace } from 'naive-ui'
import type { RequestLog } from '../types'

const loading = ref(false)
const logs = ref<RequestLog[]>([])
const searchQuery = ref('')
const dateRange = ref<[number, number] | null>(null)
const currentPage = ref(1)
const pageSize = ref(10)
const total = ref(0)

const pagination = computed(() => ({
  page: currentPage.value,
  pageSize: pageSize.value,
  itemCount: total.value,
  remote: true,
  showSizePicker: true,
  pageSizes: [10, 20, 50, 100],
  prefix: ({ itemCount }: { itemCount: number }) => `共 ${itemCount} 条`,
}))

function statusCodeColor(code: number): 'success' | 'warning' | 'error' {
  if (code < 300) return 'success'
  if (code < 400) return 'warning'
  return 'error'
}

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

const columns = [
  {
    title: '时间',
    key: 'created_at',
    width: 180,
    render: (row: RequestLog) => formatUtcTime(row.created_at),
  },
  { title: '模型', key: 'model', width: 140 },
  { title: '供应商', key: 'provider_name', width: 100 },
  {
    title: '客户端格式',
    key: 'client_format',
    width: 100,
    render: (row: RequestLog) =>
      h(NTag, { size: 'small', type: 'info' }, () => row.client_format),
  },
  {
    title: '供应商格式',
    key: 'provider_format',
    width: 100,
    render: (row: RequestLog) =>
      h(NTag, { size: 'small', type: 'warning' }, () => row.provider_format),
  },
  {
    title: '状态码',
    key: 'status_code',
    width: 80,
    render: (row: RequestLog) => {
      const code = row.status_code ?? 0
      return h(NTag, { size: 'small', type: statusCodeColor(code) }, () => String(code))
    },
  },
  {
    title: '输入/缓存',
    key: 'prompt_tokens',
    width: 130,
    render: (row: RequestLog) =>
      h(NSpace, { size: 4 }, () => [
        h(NTag, { size: 'small' }, () => formatNumber(row.prompt_tokens)),
        h(NTag, { size: 'small', type: 'info' }, () => formatNumber(row.cached_tokens)),
      ]),
  },
  {
    title: '命中',
    key: 'cache_hit',
    width: 80,
    render: (row: RequestLog) => {
      const total = row.prompt_tokens + row.cached_tokens
      if (total === 0) return '-'
      const rate = Math.round(row.cached_tokens / total * 100)
      const type = rate >= 50 ? 'success' : rate > 0 ? 'warning' : 'default'
      return h(NTag, { size: 'small', type }, () => `${rate}%`)
    },
  },
  { title: '输出', key: 'completion_tokens', width: 100, render: (row: RequestLog) => formatNumber(row.completion_tokens) },
  {
    title: '用时/首字',
    key: 'ttft_ms',
    width: 130,
    render: (row: RequestLog) =>
      h(NSpace, { size: 4 }, () => [
        h(NTag, { size: 'small' }, () => row.duration_ms != null ? `${(row.duration_ms / 1000).toFixed(1)}s` : '-'),
        h(NTag, { size: 'small', type: 'info' }, () => row.ttft_ms != null ? `${(row.ttft_ms / 1000).toFixed(1)}s` : '-'),
      ]),
  },
  {
    title: '流式',
    key: 'stream',
    width: 60,
    render: (row: RequestLog) =>
      h(NTag, { size: 'small', type: row.stream ? 'success' : 'default' }, () => (row.stream ? '是' : '否')),
  },
]

function handlePageChange(page: number) {
  currentPage.value = page
  fetchLogs()
}

function handlePageSizeChange(size: number) {
  pageSize.value = size
  currentPage.value = 1
  fetchLogs()
}

function handleQuery() {
  currentPage.value = 1
  fetchLogs()
}

async function fetchLogs() {
  loading.value = true
  try {
    let url = `/api/logs?page=${currentPage.value}&limit=${pageSize.value}`
    if (searchQuery.value.trim()) {
      url += `&model=${encodeURIComponent(searchQuery.value.trim())}`
    }
    if (dateRange.value) {
      const start = new Date(dateRange.value[0])
      const end = new Date(dateRange.value[1])
      url += `&start_date=${formatDate(start)}`
      url += `&end_date=${formatDate(end)}`
    }
    const result = await api<{ logs: RequestLog[]; total: number }>(url)
    logs.value = result.logs
    total.value = result.total
  } catch (error) {
    console.error('Failed to load logs:', error)
  } finally {
    loading.value = false
  }
}

function formatDate(d: Date): string {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

async function handleClearLogs() {
  if (!window.confirm('确定要清除所有日志吗？此操作不可恢复。')) return
  try {
    await api<{ deleted: boolean }>('/api/logs', { method: 'DELETE' })
    logs.value = []
    total.value = 0
  } catch (error) {
    console.error('Failed to clear logs:', error)
  }
}

onMounted(() => {
  fetchLogs()
})
</script>
