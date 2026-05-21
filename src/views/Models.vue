<template>
  <div class="models-view">
    <n-space style="margin-bottom: 16px" justify="space-between" align="center">
      <n-h3 style="margin: 0">{{ t('models.title') }}</n-h3>
      <n-button type="primary" @click="showAddModal = true">{{ t('models.addRoute') }}</n-button>
    </n-space>

    <n-data-table
      :columns="columns"
      :data="routes"
      :bordered="false"
      :row-key="(row: ModelRoute) => row.id"
    />

    <n-modal v-model:show="showAddModal" :title="t('models.add')" preset="card" style="width: 560px">
      <n-form :model="newRoute" label-placement="left" label-width="140">
        <n-form-item :label="t('models.modelPattern')">
          <n-input v-model:value="newRoute.model_pattern" placeholder="e.g. gpt-4*" />
        </n-form-item>
        <n-form-item :label="t('models.alias')">
          <n-input v-model:value="newRoute.alias" placeholder="Optional alias" />
        </n-form-item>
        <n-form-item :label="t('models.title')">
          <n-select v-model:value="newRoute.provider_id" :options="providerOptions" placeholder="Select provider" />
        </n-form-item>
        <n-form-item :label="t('models.targetModel')">
          <n-input v-model:value="newRoute.target_model" placeholder="e.g. gpt-4-turbo" />
        </n-form-item>
        <n-form-item :label="t('models.targetFormat')">
          <n-select v-model:value="newRoute.target_format" :options="formatOptions" />
        </n-form-item>
        <n-form-item :label="t('models.fallbackProvider')">
          <n-select v-model:value="newRoute.fallback_provider_id" :options="providerOptionsWithNone" placeholder="None" clearable />
        </n-form-item>
        <n-form-item :label="t('common.priority')">
          <n-input-number v-model:value="newRoute.priority" :min="0" :max="1000" style="width: 100%" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-space>
          <n-button @click="showAddModal = false">{{ t('common.cancel') }}</n-button>
          <n-button type="primary" @click="handleCreateRoute" :loading="creating">{{ t('common.create') }}</n-button>
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
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { ModelRoute, Provider } from '../types'

const { t } = useI18n()
const message = useMessage()
const routes = ref<ModelRoute[]>([])
const providers = ref<Provider[]>([])
const showAddModal = ref(false)
const creating = ref(false)

const formatOptions = computed(() => [
  { label: t('format.completions'), value: 'completions' },
  { label: t('format.responses'), value: 'responses' },
  { label: t('format.anthropic'), value: 'anthropic' },
  { label: t('format.gemini'), value: 'gemini' },
])

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

const columns = computed<DataTableColumns<ModelRoute>>(() => [
  { title: t('models.pattern'), key: 'model_pattern', width: 180 },
  { title: t('models.alias'), key: 'alias', width: 140, render: (row) => row.alias ?? '-' },
  { title: t('models.title'), key: 'provider_id', width: 140, render: (row) => providerName(row.provider_id) },
  { title: t('models.targetModel'), key: 'target_model', width: 180 },
  { title: t('models.targetFormat'), key: 'target_format', width: 120, render: (row) => h(NTag, { size: 'small' }, { default: () => row.target_format }) },
  { title: t('models.fallback'), key: 'fallback_provider_id', width: 120, render: (row) => row.fallback_provider_id ? providerName(row.fallback_provider_id) : '-' },
  { title: t('common.priority'), key: 'priority', width: 80 },
  {
    title: t('common.actions'), key: 'actions', width: 100,
    render: (row) => h(NButton, { size: 'small', type: 'error', onClick: () => handleDeleteRoute(row.id) }, { default: () => t('common.delete') }),
  },
])

async function loadData() {
  try {
    const [r, p] = await Promise.all([
      invoke<ModelRoute[]>('get_routes'),
      invoke<Provider[]>('get_providers'),
    ])
    routes.value = r
    providers.value = p
  } catch (e) {
    message.error(t('models.loadFailed', { error: String(e) }))
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
    message.success(t('models.created'))
    showAddModal.value = false
    newRoute.value = { model_pattern: '', alias: null, provider_id: '', target_model: '', target_format: 'completions', fallback_provider_id: null, priority: 10 }
    await loadData()
  } catch (e) {
    message.error(t('models.createFailed', { error: String(e) }))
  } finally {
    creating.value = false
  }
}

async function handleDeleteRoute(id: string) {
  try {
    await invoke('delete_route', { id })
    message.success(t('models.deleted'))
    await loadData()
  } catch (e) {
    message.error(t('models.deleteFailed', { error: String(e) }))
  }
}

onMounted(loadData)
</script>
