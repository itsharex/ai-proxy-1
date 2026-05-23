<template>
  <n-space vertical size="large">
    <n-card title="模型总览">
      <template #header-extra>
        <n-text depth="3">模型在供应商中配置</n-text>
      </template>
      <n-data-table :columns="columns" :data="allModels" />
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, h } from 'vue'
import { NTag } from 'naive-ui'
import { api } from '../api'
import type { Provider } from '../types'

const providers = ref<Provider[]>([])

interface FlatModel {
  model_name: string
  target_model: string | null
  provider_name: string
  provider_format: string
  enabled: boolean
}

const allModels = computed<FlatModel[]>(() => {
  const models: FlatModel[] = []
  for (const p of providers.value) {
    for (const m of p.models) {
      models.push({
        model_name: m.model_name,
        target_model: m.target_model,
        provider_name: p.name,
        provider_format: p.format,
        enabled: m.enabled,
      })
    }
  }
  return models
})

const formatColors: Record<string, string> = {
  completions: 'success',
  responses: 'info',
  anthropic: 'warning',
  gemini: 'purple',
}

const columns = [
  { title: '模型名称', key: 'model_name' },
  { title: '上游模型', key: 'target_model' },
  { title: '供应商', key: 'provider_name' },
  {
    title: '格式',
    key: 'provider_format',
    render(row: FlatModel) {
      const color = formatColors[row.provider_format] || 'default'
      return h(NTag, { type: color as never, size: 'small' }, { default: () => row.provider_format })
    },
  },
  {
    title: '状态',
    key: 'enabled',
    render(row: FlatModel) {
      return h(NTag, { type: row.enabled ? 'success' : 'error', size: 'small' }, {
        default: () => row.enabled ? '启用' : '禁用',
      })
    },
  },
]

async function loadData() {
  try {
    providers.value = await api<Provider[]>('/api/providers')
  } catch (e) {
    console.error('Failed to load providers:', e)
  }
}

onMounted(loadData)
</script>
