<template>
  <n-space vertical size="large">
    <n-card>
      <template #header>
        <n-space justify="space-between" align="center">
          <n-text strong>供应商管理</n-text>
          <n-button type="primary" @click="openCreateModal">
            添加供应商
          </n-button>
        </n-space>
      </template>

      <n-spin :show="store.loading">
        <n-data-table
          :columns="columns"
          :data="store.providers"
          :row-key="(row: Provider) => row.id"
          :bordered="false"
          :expandable="expandable"
        />
      </n-spin>
    </n-card>

    <!-- Add Provider Modal -->
    <n-modal
      v-model:show="showCreateModal"
      preset="dialog"
      title="添加供应商"
      positive-text="确认"
      negative-text="取消"
      :loading="createLoading"
      @positive-click="handleCreateProvider"
    >
      <n-form ref="createFormRef" :model="createForm" :rules="createRules" label-placement="left" label-width="80">
        <n-form-item label="名称" path="name">
          <n-input v-model:value="createForm.name" placeholder="例如: OpenAI" />
        </n-form-item>
        <n-form-item label="Base URL" path="base_url">
          <n-input v-model:value="createForm.base_url" placeholder="例如: https://api.openai.com" />
        </n-form-item>
        <n-form-item label="认证类型" path="auth_type">
          <n-select
            v-model:value="createForm.auth_type"
            :options="authTypeOptions"
            placeholder="选择认证类型"
          />
        </n-form-item>
        <n-form-item v-if="createForm.auth_type === 'api-key'" label="Header" path="auth_header">
          <n-input v-model:value="createForm.auth_header" placeholder="例如: X-API-Key" />
        </n-form-item>
        <n-form-item label="支持格式" path="formats">
          <n-checkbox-group v-model:value="createForm.formats">
            <n-space>
              <n-checkbox value="completions" label="Completions" />
              <n-checkbox value="responses" label="Responses" />
              <n-checkbox value="anthropic" label="Anthropic" />
              <n-checkbox value="gemini" label="Gemini" />
            </n-space>
          </n-checkbox-group>
        </n-form-item>
      </n-form>
    </n-modal>

    <!-- Add API Key Modal -->
    <n-modal
      v-model:show="showKeyModal"
      preset="dialog"
      title="添加 API Key"
      positive-text="确认"
      negative-text="取消"
      :loading="keyLoading"
      @positive-click="handleCreateApiKey"
    >
      <n-form ref="keyFormRef" :model="keyForm" :rules="keyRules" label-placement="left" label-width="80">
        <n-form-item label="标签" path="label">
          <n-input v-model:value="keyForm.label" placeholder="例如: production-key" />
        </n-form-item>
        <n-form-item label="API Key" path="plaintextKey">
          <n-input
            v-model:value="keyForm.plaintextKey"
            type="password"
            show-password-on="click"
            placeholder="输入 API Key"
          />
        </n-form-item>
      </n-form>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, reactive, h, onMounted } from 'vue'
import { NButton, NSpace, NTag, NPopconfirm, useMessage } from 'naive-ui'
import { useProvidersStore } from '../stores/providers'
import type { Provider } from '../types'

const store = useProvidersStore()
const message = useMessage()

onMounted(() => {
  store.fetchProviders()
})

// --- Expandable row config ---
const expandable = { children: 'api_keys' }

// --- Table columns ---
const columns = [
  {
    title: '名称',
    key: 'name',
    width: 160,
  },
  {
    title: 'Base URL',
    key: 'base_url',
    ellipsis: { tooltip: true },
  },
  {
    title: '认证类型',
    key: 'auth_type',
    width: 120,
    render: (row: Provider) =>
      h(NTag, { size: 'small', type: row.auth_type === 'bearer' ? 'info' : 'warning' }, () => row.auth_type),
  },
  {
    title: '支持格式',
    key: 'endpoints',
    width: 280,
    render: (row: Provider) =>
      h(NSpace, { size: 4 }, () =>
        row.endpoints.map((ep) =>
          h(NTag, { size: 'small', bordered: false, type: 'success' }, () => ep.format)
        )
      ),
  },
  {
    title: 'API Keys',
    key: 'api_keys_count',
    width: 100,
    render: (row: Provider) => String(row.api_keys.length),
  },
  {
    title: '操作',
    key: 'actions',
    width: 200,
    render: (row: Provider) =>
      h(NSpace, { size: 4 }, () => [
        h(
          NButton,
          { size: 'small', quaternary: true, onClick: () => openKeyModal(row.id) },
          () => '添加 Key'
        ),
        h(
          NPopconfirm,
          { onPositiveClick: () => handleDeleteProvider(row.id) },
          {
            trigger: () =>
              h(NButton, { size: 'small', quaternary: true, type: 'error' }, () => '删除'),
            default: () => '确认删除此供应商？',
          }
        ),
      ]),
  },
]

// --- Create Provider ---
const showCreateModal = ref(false)
const createLoading = ref(false)
const createFormRef = ref<unknown>(null)
const createForm = reactive({
  name: '',
  base_url: '',
  auth_type: 'bearer',
  auth_header: 'Authorization',
  formats: ['completions'] as string[],
})

const authTypeOptions = [
  { label: 'Bearer Token', value: 'bearer' },
  { label: 'API Key (自定义 Header)', value: 'api-key' },
]

const createRules = {
  name: { required: true, message: '请输入名称', trigger: 'blur' },
  base_url: { required: true, message: '请输入 Base URL', trigger: 'blur' },
  auth_type: { required: true, message: '请选择认证类型', trigger: 'change' },
}

function openCreateModal() {
  createForm.name = ''
  createForm.base_url = ''
  createForm.auth_type = 'bearer'
  createForm.auth_header = 'Authorization'
  createForm.formats = ['completions']
  showCreateModal.value = true
}

async function handleCreateProvider() {
  if (!createForm.name || !createForm.base_url) {
    message.warning('请填写必填字段')
    return false
  }

  createLoading.value = true
  try {
    await store.createProvider({
      name: createForm.name,
      base_url: createForm.base_url,
      auth_type: createForm.auth_type,
      auth_header: createForm.auth_type === 'bearer' ? 'Authorization' : createForm.auth_header,
      endpoints: createForm.formats.map((f) => ({ format: f, path: getDefaultPath(f) })),
    })
    message.success('供应商创建成功')
    showCreateModal.value = false
  } catch (err) {
    message.error(`创建失败: ${err}`)
  } finally {
    createLoading.value = false
  }
  return false
}

function getDefaultPath(format: string): string {
  const paths: Record<string, string> = {
    completions: '/v1/chat/completions',
    responses: '/v1/responses',
    anthropic: '/v1/messages',
    gemini: '/v1beta/models',
  }
  return paths[format] ?? '/v1/chat/completions'
}

async function handleDeleteProvider(id: string) {
  try {
    await store.deleteProvider(id)
    message.success('供应商已删除')
  } catch (err) {
    message.error(`删除失败: ${err}`)
  }
}

// --- Create API Key ---
const showKeyModal = ref(false)
const keyLoading = ref(false)
const keyFormRef = ref<unknown>(null)
const currentProviderId = ref('')

const keyForm = reactive({
  label: '',
  plaintextKey: '',
})

const keyRules = {
  label: { required: true, message: '请输入标签', trigger: 'blur' },
  plaintextKey: { required: true, message: '请输入 API Key', trigger: 'blur' },
}

function openKeyModal(providerId: string) {
  currentProviderId.value = providerId
  keyForm.label = ''
  keyForm.plaintextKey = ''
  showKeyModal.value = true
}

async function handleCreateApiKey() {
  if (!keyForm.label || !keyForm.plaintextKey) {
    message.warning('请填写必填字段')
    return false
  }

  keyLoading.value = true
  try {
    await store.createApiKey(currentProviderId.value, keyForm.label, keyForm.plaintextKey)
    message.success('API Key 添加成功')
    showKeyModal.value = false
  } catch (err) {
    message.error(`添加失败: ${err}`)
  } finally {
    keyLoading.value = false
  }
  return false
}
</script>
