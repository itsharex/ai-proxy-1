<template>
  <div class="logs-view">
    <n-space style="margin-bottom: 16px" align="center">
      <n-input v-model:value="filterModel" :placeholder="t('logs.filterModel')" clearable style="width: 200px" @update:value="loadLogs" />
      <n-input-number v-model:value="filterStatus" :placeholder="t('logs.filterStatus')" clearable style="width: 140px" :min="100" :max="599" @update:value="loadLogs" />
      <n-button @click="loadLogs">{{ t('common.refresh') }}</n-button>
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
      <n-drawer-content :title="t('logs.requestDetail')">
        <template v-if="selectedLog">
          <n-descriptions bordered :column="1" label-placement="left" size="small">
            <n-descriptions-item :label="t('logs.requestId')">{{ selectedLog.request_id }}</n-descriptions-item>
            <n-descriptions-item :label="t('logs.time')">{{ selectedLog.created_at }}</n-descriptions-item>
            <n-descriptions-item :label="t('logs.model')">{{ selectedLog.model }}</n-descriptions-item>
            <n-descriptions-item :label="t('logs.clientFormat')">{{ selectedLog.client_format }}</n-descriptions-item>
            <n-descriptions-item :label="t('logs.providerName')">{{ selectedLog.provider_name }}</n-descriptions-item>
            <n-descriptions-item :label="t('logs.providerFormat')">{{ selectedLog.provider_format }}</n-descriptions-item>
            <n-descriptions-item :label="t('logs.stream')">{{ selectedLog.stream ? t('common.yes') : t('common.no') }}</n-descriptions-item>
            <n-descriptions-item :label="t('logs.statusCode')">{{ selectedLog.status_code ?? '-' }}</n-descriptions-item>
            <n-descriptions-item :label="t('logs.duration')">{{ selectedLog.duration_ms != null ? `${selectedLog.duration_ms}ms` : '-' }}</n-descriptions-item>
            <n-descriptions-item :label="t('logs.promptTokens')">{{ selectedLog.prompt_tokens }}</n-descriptions-item>
            <n-descriptions-item :label="t('logs.completionTokens')">{{ selectedLog.completion_tokens }}</n-descriptions-item>
            <n-descriptions-item :label="t('logs.totalTokens')">{{ selectedLog.total_tokens }}</n-descriptions-item>
          </n-descriptions>
          <n-card v-if="selectedLog.error_message" :title="t('logs.error')" size="small" style="margin-top: 16px">
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
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { RequestLog } from '../types'

const { t } = useI18n()

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
  { title: t('logs.time'), key: 'created_at', width: 180 },
  { title: t('logs.model'), key: 'model', width: 180 },
  { title: t('logs.client'), key: 'client_format', width: 110 },
  { title: t('logs.providerName'), key: 'provider_name', width: 120 },
  {
    title: t('logs.statusCode'), key: 'status_code', width: 80,
    render: (row) => {
      const code = row.status_code ?? 0
      const type = code < 400 ? 'success' : code < 500 ? 'warning' : 'error'
      return h(NTag, { size: 'small', type }, { default: () => String(row.status_code ?? '-') })
    },
  },
  { title: t('logs.duration'), key: 'duration_ms', width: 100, render: (row) => row.duration_ms != null ? `${row.duration_ms}ms` : '-' },
  { title: t('logs.totalTokens'), key: 'total_tokens', width: 90 },
  {
    title: '', key: 'detail', width: 70,
    render: (row) => h(NButton, { size: 'small', quaternary: true, onClick: () => openDetail(row) }, { default: () => t('common.detail') }),
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
