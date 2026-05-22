<template>
  <n-space vertical size="large">
    <n-card>
      <template #header>
        <n-space justify="space-between" align="center">
          <n-text strong>模型路由配置</n-text>
          <n-button type="primary" @click="openAddModal">
            添加路由
          </n-button>
        </n-space>
      </template>
      <n-data-table
        :columns="columns"
        :data="routes"
        :loading="loading"
        :bordered="false"
      />
    </n-card>

    <n-modal
      v-model:show="showModal"
      preset="dialog"
      title="添加路由"
      positive-text="确认"
      negative-text="取消"
      @positive-click="handleAdd"
    >
      <n-form label-placement="left" label-width="100">
        <n-form-item label="模型模式">
          <n-input
            v-model:value="formData.model_pattern"
            placeholder="gpt-* 或 deepseek-chat"
          />
        </n-form-item>
        <n-form-item label="别名">
          <n-input
            v-model:value="formData.alias"
            placeholder="可选，用于显示名称"
          />
        </n-form-item>
        <n-form-item label="供应商">
          <n-select
            v-model:value="formData.provider_id"
            :options="providerOptions"
            placeholder="选择供应商"
          />
        </n-form-item>
        <n-form-item label="目标模型">
          <n-input
            v-model:value="formData.target_model"
            placeholder="目标模型名称"
          />
        </n-form-item>
        <n-form-item label="目标格式">
          <n-select
            v-model:value="formData.target_format"
            :options="formatOptions"
            placeholder="选择目标格式"
          />
        </n-form-item>
        <n-form-item label="优先级">
          <n-input-number
            v-model:value="formData.priority"
            :min="0"
            :max="100"
            style="width: 100%"
          />
        </n-form-item>
      </n-form>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { NTag, NPopconfirm, NButton, NSpace, useMessage } from 'naive-ui'
import type { ModelRoute, Provider } from '../types'

const message = useMessage()
const loading = ref(false)
const routes = ref<ModelRoute[]>([])
const providers = ref<Provider[]>([])
const showModal = ref(false)

const formData = ref({
  model_pattern: '',
  alias: '',
  provider_id: '',
  target_model: '',
  target_format: 'completions',
  priority: 0,
})

const formatOptions = [
  { label: 'Completions', value: 'completions' },
  { label: 'Responses', value: 'responses' },
  { label: 'Anthropic', value: 'anthropic' },
  { label: 'Gemini', value: 'gemini' },
]

const providerOptions = ref<{ label: string; value: string }[]>([])

const formatColorMap: Record<string, 'default' | 'info' | 'success' | 'warning' | 'error'> = {
  completions: 'info',
  responses: 'success',
  anthropic: 'warning',
  gemini: 'error',
}

const columns = [
  { title: '模型模式', key: 'model_pattern', width: 160 },
  { title: '别名', key: 'alias', width: 120, render: (row: ModelRoute) => row.alias || '-' },
  { title: '供应商 ID', key: 'provider_id', width: 140 },
  { title: '目标模型', key: 'target_model', width: 160 },
  {
    title: '目标格式',
    key: 'target_format',
    width: 120,
    render: (row: ModelRoute) =>
      h(NTag, { size: 'small', type: formatColorMap[row.target_format] || 'default' }, () => row.target_format),
  },
  { title: '优先级', key: 'priority', width: 80 },
  {
    title: '操作',
    key: 'actions',
    width: 100,
    render: (row: ModelRoute) =>
      h(NPopconfirm, { onPositiveClick: () => handleDelete(row.id) }, {
        trigger: () => h(NButton, { size: 'small', type: 'error', quaternary: true }, () => '删除'),
        default: () => '确认删除此路由？',
      }),
  },
]

function openAddModal() {
  formData.value = {
    model_pattern: '',
    alias: '',
    provider_id: '',
    target_model: '',
    target_format: 'completions',
    priority: 0,
  }
  showModal.value = true
}

async function handleAdd() {
  try {
    await invoke('create_route', {
      modelPattern: formData.value.model_pattern,
      alias: formData.value.alias || null,
      providerId: formData.value.provider_id,
      targetModel: formData.value.target_model,
      targetFormat: formData.value.target_format,
      priority: formData.value.priority,
    })
    message.success('路由添加成功')
    await fetchRoutes()
  } catch (error) {
    message.error(`添加失败: ${error}`)
  }
}

async function handleDelete(id: string) {
  try {
    await invoke('delete_route', { id })
    message.success('路由已删除')
    await fetchRoutes()
  } catch (error) {
    message.error(`删除失败: ${error}`)
  }
}

async function fetchRoutes() {
  loading.value = true
  try {
    routes.value = await invoke<ModelRoute[]>('get_routes')
  } catch (error) {
    console.error('Failed to load routes:', error)
  } finally {
    loading.value = false
  }
}

async function fetchProviders() {
  try {
    providers.value = await invoke<Provider[]>('get_providers')
    providerOptions.value = providers.value.map((p) => ({
      label: p.name || p.id,
      value: p.id,
    }))
  } catch (error) {
    console.error('Failed to load providers:', error)
  }
}

onMounted(() => {
  fetchRoutes()
  fetchProviders()
})
</script>
