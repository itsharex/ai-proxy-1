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
          <n-statistic label="模型路由数">
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
            {{ todayRequests }}
          </n-statistic>
        </n-card>
      </n-gi>
      <n-gi span="4 m:1">
        <n-card>
          <n-statistic label="今日 Token 用量">
            <template #prefix>
              <n-icon :component="PulseOutline" />
            </template>
            {{ todayTokens }}
          </n-statistic>
        </n-card>
      </n-gi>
    </n-grid>

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

    <n-card title="最近请求">
      <n-data-table
        v-if="recentLogs.length > 0"
        :columns="logColumns"
        :data="recentLogs"
        :bordered="false"
        size="small"
      />
      <n-empty v-else description="暂无请求记录" />
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, h } from 'vue'
import { NTag } from 'naive-ui'
import {
  ServerOutline,
  GitBranchOutline,
  DocumentTextOutline,
  PulseOutline,
} from '@vicons/ionicons5'

const serverRunning = ref(true)
const proxyPort = ref(7860)
const providerCount = ref(0)
const routeCount = ref(0)
const todayRequests = ref(0)
const todayTokens = ref(0)
const recentLogs = ref<Record<string, unknown>[]>([])

const logColumns = [
  { title: '时间', key: 'created_at', width: 180 },
  { title: '模型', key: 'model', width: 160 },
  { title: '供应商', key: 'provider_name', width: 120 },
  {
    title: '状态',
    key: 'status_code',
    width: 80,
    render: (row: Record<string, unknown>) => {
      const code = row.status_code as number
      return h(NTag, { size: 'small', type: code < 400 ? 'success' : 'error' }, () => String(code))
    },
  },
  { title: '耗时', key: 'duration_ms', width: 100, render: (row: Record<string, unknown>) => `${row.duration_ms}ms` },
]
</script>
