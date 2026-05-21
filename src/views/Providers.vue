<template>
  <div class="providers-view">
    <n-space style="margin-bottom: 16px" justify="space-between" align="center">
      <n-h3 style="margin: 0">Providers</n-h3>
      <n-button type="primary" @click="showAddModal = true">Add Provider</n-button>
    </n-space>

    <n-data-table
      :columns="columns"
      :data="providers"
      :bordered="false"
      :row-key="(row: Provider) => row.id"
      :expanded-row-keys="expandedKeys"
      @update:expanded-row-keys="handleExpand"
    />

    <n-modal v-model:show="showAddModal" title="Add Provider" preset="card" style="width: 560px">
      <n-form :model="newProvider" label-placement="left" label-width="100">
        <n-form-item label="Name" path="name">
          <n-input v-model:value="newProvider.name" placeholder="e.g. OpenAI" />
        </n-form-item>
        <n-form-item label="Base URL" path="base_url">
          <n-input v-model:value="newProvider.base_url" placeholder="https://api.openai.com" />
        </n-form-item>
        <n-form-item label="Auth Type" path="auth_type">
          <n-select v-model:value="newProvider.auth_type" :options="authTypeOptions" />
        </n-form-item>
        <n-form-item label="Auth Header" path="auth_header">
          <n-input v-model:value="newProvider.auth_header" placeholder="Authorization" />
        </n-form-item>
        <n-form-item label="Endpoints">
          <div style="width: 100%">
            <div v-for="(ep, idx) in newProvider.endpoints" :key="idx" style="display: flex; gap: 8px; margin-bottom: 8px">
              <n-select v-model:value="ep.format" :options="formatOptions" style="width: 160px" placeholder="Format" />
              <n-input v-model:value="ep.path" placeholder="/v1/chat/completions" style="flex: 1" />
              <n-button quaternary type="error" @click="newProvider.endpoints.splice(idx, 1)">X</n-button>
            </div>
            <n-button dashed block @click="newProvider.endpoints.push({ format: '', path: '' })">+ Add Endpoint</n-button>
          </div>
        </n-form-item>
      </n-form>
      <template #action>
        <n-space>
          <n-button @click="showAddModal = false">Cancel</n-button>
          <n-button type="primary" @click="handleCreateProvider" :loading="creating">Create</n-button>
        </n-space>
      </template>
    </n-modal>

    <n-modal v-model:show="showKeyModal" title="Add API Key" preset="card" style="width: 420px">
      <n-form :model="newKey" label-placement="left" label-width="80">
        <n-form-item label="Label">
          <n-input v-model:value="newKey.label" placeholder="e.g. Production Key" />
        </n-form-item>
        <n-form-item label="API Key">
          <n-input v-model:value="newKey.plaintext_key" type="password" show-password-on="click" placeholder="sk-..." />
        </n-form-item>
      </n-form>
      <template #action>
        <n-space>
          <n-button @click="showKeyModal = false">Cancel</n-button>
          <n-button type="primary" @click="handleCreateKey" :loading="creatingKey">Add Key</n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, h, onMounted } from 'vue'
import {
  NDataTable, NButton, NModal, NForm, NFormItem, NInput, NSelect, NSpace, NTag,
  type DataTableColumns, type DataTableRowKey,
} from 'naive-ui'
import { useMessage } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import type { Provider } from '../types'

const message = useMessage()
const providers = ref<Provider[]>([])
const showAddModal = ref(false)
const showKeyModal = ref(false)
const creating = ref(false)
const creatingKey = ref(false)
const expandedKeys = ref<string[]>([])
const keyProviderId = ref('')

const authTypeOptions = [
  { label: 'Bearer Token', value: 'bearer' },
  { label: 'API Key', value: 'api_key' },
  { label: 'Custom Header', value: 'custom' },
]

const formatOptions = [
  { label: 'OpenAI Completions', value: 'completions' },
  { label: 'OpenAI Responses', value: 'responses' },
  { label: 'Anthropic', value: 'anthropic' },
  { label: 'Gemini', value: 'gemini' },
]

const newProvider = ref({
  name: '',
  base_url: '',
  auth_type: 'bearer',
  auth_header: 'Authorization',
  endpoints: [] as { format: string; path: string }[],
})

const newKey = ref({
  label: '',
  plaintext_key: '',
})

const columns: DataTableColumns<Provider> = [
  { title: 'Name', key: 'name', width: 160 },
  { title: 'Base URL', key: 'base_url', ellipsis: { tooltip: true } },
  { title: 'Auth', key: 'auth_type', width: 120, render: (row) => h(NTag, { size: 'small' }, { default: () => row.auth_type }) },
  { title: 'Endpoints', key: 'endpoints', width: 100, render: (row) => String(row.endpoints.length) },
  { title: 'Keys', key: 'keys', width: 80, render: (row) => String(row.api_keys.length) },
  {
    title: 'Actions', key: 'actions', width: 160,
    render: (row) => h(NSpace, { size: 'small' }, {
      default: () => [
        h(NButton, { size: 'small', onClick: () => openKeyModal(row.id) }, { default: () => '+ Key' }),
        h(NButton, { size: 'small', type: 'error', onClick: () => handleDeleteProvider(row.id) }, { default: () => 'Delete' }),
      ],
    }),
  },
]

function handleExpand(keys: DataTableRowKey[]) {
  expandedKeys.value = keys as string[]
}

function openKeyModal(providerId: string) {
  keyProviderId.value = providerId
  newKey.value = { label: '', plaintext_key: '' }
  showKeyModal.value = true
}

async function loadProviders() {
  try {
    providers.value = await invoke<Provider[]>('get_providers')
  } catch (e) {
    message.error(`Failed to load providers: ${e}`)
  }
}

async function handleCreateProvider() {
  creating.value = true
  try {
    await invoke('create_provider', {
      name: newProvider.value.name,
      baseUrl: newProvider.value.base_url,
      authType: newProvider.value.auth_type,
      authHeader: newProvider.value.auth_header,
      endpoints: newProvider.value.endpoints,
    })
    message.success('Provider created')
    showAddModal.value = false
    newProvider.value = { name: '', base_url: '', auth_type: 'bearer', auth_header: 'Authorization', endpoints: [] }
    await loadProviders()
  } catch (e) {
    message.error(`Failed: ${e}`)
  } finally {
    creating.value = false
  }
}

async function handleDeleteProvider(id: string) {
  try {
    await invoke('delete_provider', { id })
    message.success('Provider deleted')
    await loadProviders()
  } catch (e) {
    message.error(`Failed: ${e}`)
  }
}

async function handleCreateKey() {
  creatingKey.value = true
  try {
    await invoke('create_api_key', {
      providerId: keyProviderId.value,
      label: newKey.value.label,
      plaintextKey: newKey.value.plaintext_key,
    })
    message.success('API Key added')
    showKeyModal.value = false
    await loadProviders()
  } catch (e) {
    message.error(`Failed: ${e}`)
  } finally {
    creatingKey.value = false
  }
}

onMounted(loadProviders)
</script>
