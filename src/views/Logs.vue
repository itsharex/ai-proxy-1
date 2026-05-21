<template>
  <div class="logs-view">
    <n-space style="margin-bottom: 16px" align="center">
      <n-input v-model:value="filterModel" placeholder="Filter by model" clearable style="width: 200px" @update:value="loadLogs" />
      <n-input-number v-model:value="filterStatus" placeholder="Status code" clearable style="width: 140px" :min="100" :max="599" @update:value="loadLogs" />
      <n-button @click="loadLogs">Refresh</n-button>
    </n-space>

    <n-data-table
      :columns="columns"
      :data="filteredLogs"
      :bordered="false"
      :row-key="(row: RequestLog) => row.id"
      size="small"
      @update:page="handlePageChange"
      :pagination="pagination"
    />

    <n-drawer v-model:show="showDetail" :width="480">
      <n-drawer-content title="Request Detail">
        <template v-if="selectedLog">
          <n-descriptions bordered :column="1" label-placement="left" size="small">
            <n-descriptions-item label="Request ID">{{ selectedLog.request_id }}</n-descriptions-item>
            <n-descriptions-item label="Time">{{ selectedLog.created_at }}</n-descriptions-item>
            <n-descriptions-item label="Model">{{ selectedLog.model }}</n-descriptions-item>
            <n-descriptions-item label="Client Format">{{ selectedLog.client_format }}</n-descriptions-item>
            <n-descriptions-item label="Provider">{{ selectedLog.provider_name }}</n-descriptions-item>
            <n-descriptions-item label="Provider Format">{{ selectedLog.provider_format }}</n-descriptions-item>
            <n-descriptions-item label="Stream">{{ selectedLog.stream ? 'Yes' : 'No' }}</n-descriptions-item>
            <n-descriptions-item label="Status Code">{{ selectedLog.status_code ?? '-' }}</n-descriptions-item>
            <n-descriptions-item label="Duration">{{ selectedLog.duration_ms != null ? `${selectedLog.duration_ms}ms` : '-' }}</n-descriptions-item>
            <n-descriptions-item label="Prompt Tokens">{{ selectedLog.prompt_tokens }}</n-descriptions-item>
            <n-descriptions-item label="Completion Tokens">{{ selectedLog.completion_tokens }}</n-descriptions-item>
            <n-descriptions-item label="Total Tokens">{{ selectedLog.total_tokens }}</n-descriptions-item>
          </n-descriptions>
          <n-card v-if="selectedLog.error_message" title="Error" size="small" style="margin-top: 16px">
            <pre style="white-space: pre-wrap; color: #e88080">{{ selectedLog.error_message }}</pre>
          </n-card>
        </template>
      </n-drawer-content>
    </n-drawer>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, h, onMounted } from 'vue'
import {
  NDataTable, NSpace, NInput, NInputNumber, NButton, NTag, NDrawer, NDrawerContent,
  NDescriptions, NDescriptionsItem, NCard,
  type DataTableColumns,
} from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import type { RequestLog } from '../types'

const logs = ref<RequestLog[]>([])
const filterModel = ref('')
const filterStatus = ref<number | null>(null)
const showDetail = ref(false)
const selectedLog = ref<RequestLog | null>(null)
const currentPage = ref(1)
const pageSize = 20

const pagination = computed(() => ({
  page: currentPage.value,
  pageSize,
}))

const filteredLogs = computed(() => {
  let result = logs.value
  if (filterModel.value) {
    const q = filterModel.value.toLowerCase()
    result = result.filter((l) => l.model.toLowerCase().includes(q))
  }
  if (filterStatus.value != null) {
    result = result.filter((l) => l.status_code === filterStatus.value)
  }
  return result
})

const columns: DataTableColumns<RequestLog> = [
  { title: 'Time', key: 'created_at', width: 180 },
  { title: 'Model', key: 'model', width: 180 },
  { title: 'Client', key: 'client_format', width: 110 },
  { title: 'Provider', key: 'provider_name', width: 120 },
  {
    title: 'Status', key: 'status_code', width: 80,
    render: (row) => {
      const code = row.status_code ?? 0
      const type = code < 400 ? 'success' : code < 500 ? 'warning' : 'error'
      return h(NTag, { size: 'small', type }, { default: () => String(row.status_code ?? '-') })
    },
  },
  { title: 'Duration', key: 'duration_ms', width: 100, render: (row) => row.duration_ms != null ? `${row.duration_ms}ms` : '-' },
  { title: 'Tokens', key: 'total_tokens', width: 90 },
  {
    title: '', key: 'detail', width: 70,
    render: (row) => h(NButton, { size: 'small', quaternary: true, onClick: () => openDetail(row) }, { default: () => 'Detail' }),
  },
]

function openDetail(log: RequestLog) {
  selectedLog.value = log
  showDetail.value = true
}

function handlePageChange(page: number) {
  currentPage.value = page
  loadLogs()
}

async function loadLogs() {
  try {
    logs.value = await invoke<RequestLog[]>('get_logs', { page: currentPage.value, limit: pageSize })
  } catch {
    logs.value = []
  }
}

onMounted(loadLogs)
</script>
