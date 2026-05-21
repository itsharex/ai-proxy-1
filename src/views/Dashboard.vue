<template>
  <div class="dashboard">
    <n-grid :x-gap="16" :y-gap="16" :cols="3" style="margin-bottom: 24px">
      <n-gi>
        <n-card :title="t('dashboard.serverStatus')" size="small">
          <n-space vertical>
            <n-tag :type="serverRunning ? 'success' : 'error'">
              {{ serverRunning ? t('dashboard.running') : t('dashboard.stopped') }}
            </n-tag>
            <span style="font-size: 13px; color: #999">localhost:{{ serverPort }}</span>
          </n-space>
        </n-card>
      </n-gi>
      <n-gi>
        <n-card :title="t('dashboard.activeProviders')" size="small">
          <div class="stat-value">{{ providerCount }}</div>
        </n-card>
      </n-gi>
      <n-gi>
        <n-card :title="t('dashboard.activeRoutes')" size="small">
          <div class="stat-value">{{ routeCount }}</div>
        </n-card>
      </n-gi>
    </n-grid>

    <n-card :title="t('dashboard.recentRequests')" size="small">
      <n-data-table
        :columns="recentColumns"
        :data="recentLogs"
        :bordered="false"
        size="small"
        :pagination="false"
      />
    </n-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, h } from 'vue'
import { NGrid, NGi, NCard, NTag, NDataTable, NSpace, type DataTableColumns } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { RequestLog, Provider, ModelRoute } from '../types'

const { t } = useI18n()
const serverRunning = ref(true)
const serverPort = ref(7860)
const providerCount = ref(0)
const routeCount = ref(0)
const recentLogs = ref<RequestLog[]>([])

const recentColumns = computed<DataTableColumns<RequestLog>>(() => [
  { title: t('dashboard.time'), key: 'created_at', width: 180 },
  { title: t('dashboard.model'), key: 'model', width: 200 },
  { title: t('dashboard.provider'), key: 'provider_name', width: 120 },
  { title: t('dashboard.status'), key: 'status_code', width: 80, render: (row) => h(NTag, { size: 'small', type: (row.status_code ?? 0) < 400 ? 'success' : 'error' }, { default: () => String(row.status_code ?? '-') }) },
  { title: t('dashboard.duration'), key: 'duration_ms', width: 100, render: (row) => row.duration_ms != null ? `${row.duration_ms}ms` : '-' },
  { title: t('dashboard.tokens'), key: 'total_tokens', width: 100 },
])

async function loadDashboard() {
  try {
    const providers = await invoke<Provider[]>('get_providers')
    providerCount.value = providers.length
    serverRunning.value = true
  } catch {
    serverRunning.value = false
  }

  try {
    const routes = await invoke<ModelRoute[]>('get_routes')
    routeCount.value = routes.length
  } catch {
    routeCount.value = 0
  }

  try {
    const logs = await invoke<RequestLog[]>('get_logs', { page: 1, limit: 10 })
    recentLogs.value = logs
  } catch {
    recentLogs.value = []
  }
}

onMounted(loadDashboard)
</script>

<style scoped>
.stat-value {
  font-size: 32px;
  font-weight: 700;
}
</style>
