<template>
  <div class="models-view">
    <n-space style="margin-bottom: 16px" justify="space-between" align="center">
      <n-h3 style="margin: 0">Model Routes</n-h3>
      <n-button type="primary" @click="showAddModal = true">Add Route</n-button>
    </n-space>

    <n-data-table
      :columns="columns"
      :data="routes"
      :bordered="false"
      :row-key="(row: ModelRoute) => row.id"
    />

    <n-modal v-model:show="showAddModal" title="Add Model Route" preset="card" style="width: 560px">
      <n-form :model="newRoute" label-placement="left" label-width="140">
        <n-form-item label="Model Pattern">
          <n-input v-model:value="newRoute.model_pattern" placeholder="e.g. gpt-4*" />
        </n-form-item>
        <n-form-item label="Alias">
          <n-input v-model:value="newRoute.alias" placeholder="Optional alias" />
        </n-form-item>
        <n-form-item label="Provider">
          <n-select v-model:value="newRoute.provider_id" :options="providerOptions" placeholder="Select provider" />
        </n-form-item>
        <n-form-item label="Target Model">
          <n-input v-model:value="newRoute.target_model" placeholder="e.g. gpt-4-turbo" />
        </n-form-item>
        <n-form-item label="Target Format">
          <n-select v-model:value="newRoute.target_format" :options="formatOptions" />
        </n-form-item>
        <n-form-item label="Fallback Provider">
          <n-select v-model:value="newRoute.fallback_provider_id" :options="providerOptionsWithNone" placeholder="None" clearable />
        </n-form-item>
        <n-form-item label="Priority">
          <n-input-number v-model:value="newRoute.priority" :min="0" :max="1000" style="width: 100%" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-space>
          <n-button @click="showAddModal = false">Cancel</n-button>
          <n-button type="primary" @click="handleCreateRoute" :loading="creating">Create</n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, h, onMounted } from 'vue'
import {
  NDataTable, NButton, NModal, NForm, NFormItem, NInput, NInputNumber, NSelect, NSpace, NTag,
  type DataTableColumns,
} from 'naive-ui'
import { useMessage } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import type { ModelRoute, Provider } from '../types'

const message = useMessage()
const routes = ref<ModelRoute[]>([])
const providers = ref<Provider[]>([])
const showAddModal = ref(false)
const creating = ref(false)

const formatOptions = [
  { label: 'OpenAI Completions', value: 'completions' },
  { label: 'OpenAI Responses', value: 'responses' },
  { label: 'Anthropic', value: 'anthropic' },
  { label: 'Gemini', value: 'gemini' },
]

const providerOptions = computed(() =>
  providers.value.map((p) => ({ label: p.name, value: p.id }))
)

const providerOptionsWithNone = computed(() =>
  [{ label: 'None', value: '' }, ...providerOptions.value]
)

const newRoute = ref({
  model_pattern: '',
  alias: null as string | null,
  provider_id: '',
  target_model: '',
  target_format: 'completions',
  fallback_provider_id: null as string | null,
  priority: 10,
})

function providerName(id: string): string {
  const p = providers.value.find((p) => p.id === id)
  return p ? p.name : id
}

const columns: DataTableColumns<ModelRoute> = [
  { title: 'Pattern', key: 'model_pattern', width: 180 },
  { title: 'Alias', key: 'alias', width: 140, render: (row) => row.alias ?? '-' },
  { title: 'Provider', key: 'provider_id', width: 140, render: (row) => providerName(row.provider_id) },
  { title: 'Target Model', key: 'target_model', width: 180 },
  { title: 'Format', key: 'target_format', width: 120, render: (row) => h(NTag, { size: 'small' }, { default: () => row.target_format }) },
  { title: 'Fallback', key: 'fallback_provider_id', width: 120, render: (row) => row.fallback_provider_id ? providerName(row.fallback_provider_id) : '-' },
  { title: 'Priority', key: 'priority', width: 80 },
  {
    title: 'Actions', key: 'actions', width: 100,
    render: (row) => h(NButton, { size: 'small', type: 'error', onClick: () => handleDeleteRoute(row.id) }, { default: () => 'Delete' }),
  },
]

async function loadData() {
  try {
    const [r, p] = await Promise.all([
      invoke<ModelRoute[]>('get_routes'),
      invoke<Provider[]>('get_providers'),
    ])
    routes.value = r
    providers.value = p
  } catch (e) {
    message.error(`Failed to load data: ${e}`)
  }
}

async function handleCreateRoute() {
  creating.value = true
  try {
    await invoke('create_route', {
      modelPattern: newRoute.value.model_pattern,
      alias: newRoute.value.alias || null,
      providerId: newRoute.value.provider_id,
      targetModel: newRoute.value.target_model,
      targetFormat: newRoute.value.target_format,
      fallbackProviderId: newRoute.value.fallback_provider_id || null,
      priority: newRoute.value.priority,
    })
    message.success('Route created')
    showAddModal.value = false
    newRoute.value = { model_pattern: '', alias: null, provider_id: '', target_model: '', target_format: 'completions', fallback_provider_id: null, priority: 10 }
    await loadData()
  } catch (e) {
    message.error(`Failed: ${e}`)
  } finally {
    creating.value = false
  }
}

async function handleDeleteRoute(id: string) {
  try {
    await invoke('delete_route', { id })
    message.success('Route deleted')
    await loadData()
  } catch (e) {
    message.error(`Failed: ${e}`)
  }
}

onMounted(loadData)
</script>
