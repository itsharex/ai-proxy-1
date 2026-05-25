<template>
  <n-space vertical size="large">
    <n-card>
      <template #header>
        <n-space justify="space-between" align="center">
          <n-text strong>请求日志</n-text>
          <n-input
            v-model:value="searchQuery"
            placeholder="按模型名搜索"
            clearable
            style="width: 260px"
          />
        </n-space>
      </template>
      <n-data-table
        :columns="columns"
        :data="filteredLogs"
        :loading="loading"
        :bordered="false"
        :pagination="pagination"
        :row-key="(row: RequestLog) => row.id"
        @update:page="handlePageChange"
      />
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, h } from 'vue'
import { api } from '../api'
import { NTag } from 'naive-ui'
import type { RequestLog } from '../types'

const loading = ref(false)
const logs = ref<RequestLog[]>([])
const searchQuery = ref('')
const currentPage = ref(1)
const pageSize = 20

const pagination = computed(() => ({
  page: currentPage.value,
  pageSize,
  pageCount: Math.ceil(filteredLogs.value.length / pageSize),
  showSizePicker: false,
}))

const filteredLogs = computed(() => {
  if (!searchQuery.value.trim()) {
    return logs.value
  }
  const query = searchQuery.value.toLowerCase()
  return logs.value.filter((log) => log.model.toLowerCase().includes(query))
})

function statusCodeColor(code: number): 'success' | 'warning' | 'error' {
  if (code < 300) return 'success'
  if (code < 400) return 'warning'
  return 'error'
}

const columns = [
  { title: '时间', key: 'created_at', width: 180 },
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
  { title: '输入Token', key: 'prompt_tokens', width: 90 },
  { title: '缓存Token', key: 'cached_tokens', width: 90 },
  { title: '输出Token', key: 'completion_tokens', width: 90 },
  {
    title: '用时/首字',
    key: 'ttft_ms',
    width: 100,
    render: (row: RequestLog) => row.ttft_ms != null ? `${row.ttft_ms}ms` : '-',
  },
  {
    title: '延迟(ms)',
    key: 'duration_ms',
    width: 90,
    render: (row: RequestLog) => row.duration_ms != null ? `${row.duration_ms}ms` : '-',
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
}

async function fetchLogs() {
  loading.value = true
  try {
    const result = await api<{ logs: RequestLog[]; total: number }>(
      `/api/logs?page=${currentPage.value}&limit=${pageSize}`
    )
    logs.value = result.logs
  } catch (error) {
    console.error('Failed to load logs:', error)
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  fetchLogs()
})
</script>
