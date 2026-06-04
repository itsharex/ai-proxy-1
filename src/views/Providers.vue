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

      <n-spin :show="loading">
        <n-data-table
          :columns="columns"
          :data="providers"
          :row-key="(row: Provider) => row.id"
          :bordered="false"
        />
      </n-spin>
    </n-card>

    <n-modal
      v-model:show="showModal"
      preset="dialog"
      :title="isEditing ? '编辑供应商' : '添加供应商'"
      positive-text="确认"
      negative-text="取消"
      :loading="modalLoading"
      @positive-click="handleSubmit"
      style="width: 600px"
    >
      <n-form :model="form" label-placement="left" label-width="80">
        <n-form-item label="名称" required>
          <n-input v-model:value="form.name" placeholder="例如: OpenAI" :input-props="{ autocapitalize: 'off' }" />
        </n-form-item>
        <n-form-item label="Base URL" required>
          <n-input v-model:value="form.base_url" placeholder="例如: https://api.openai.com" :input-props="{ autocapitalize: 'off' }" />
        </n-form-item>
        <n-form-item label="格式">
          <n-select
            v-model:value="form.format"
            :options="formatOptions"
            placeholder="选择格式"
          />
        </n-form-item>
        <n-form-item label="Endpoint">
          <n-input
            v-model:value="form.endpoint_path"
            placeholder="留空使用默认路径，例如: /chat/completions"
            :input-props="{ autocapitalize: 'off' }"
          />
        </n-form-item>
        <n-form-item label="API Key" :required="!isEditing">
          <n-input
            v-model:value="form.api_key"
            type="password"
            show-password-on="click"
            :placeholder="isEditing ? '留空则不修改' : '输入 API Key'"
          />
        </n-form-item>

        <n-divider>模型列表</n-divider>

        <n-space vertical size="small">
          <div
            v-for="(_, index) in form.models"
            :key="index"
            style="display: flex; align-items: center; gap: 8px"
          >
            <n-input
              v-model:value="form.models[index].model_name"
              placeholder="模型名称"
              style="flex: 1"
            />
            <n-input-number
              v-model:value="form.models[index].context_window"
              placeholder="上下文窗口"
              :min="1000"
              :step="1000"
              style="width: 160px"
            >
              <template #suffix>tokens</template>
            </n-input-number>
            <n-button
              quaternary
              type="error"
              size="small"
              @click="removeModel(index)"
            >
              删除
            </n-button>
          </div>
          <n-button dashed size="small" @click="addModel">
            + 添加模型
          </n-button>
        </n-space>
      </n-form>
    </n-modal>

    <n-modal
      v-model:show="showTestModal"
      preset="card"
      :title="`测试模型 - ${testProviderName}`"
      style="width: 600px"
    >
      <n-spin :show="testing">
        <template v-if="testResult">
          <n-result :status="testResult.success ? 'success' : 'error'"
            :title="testResult.message"
          >
            <template #footer>
              <n-descriptions label-placement="left" bordered :column="1" size="small">
                <n-descriptions-item v-if="testResult.duration_ms != null" label="响应时间">
                  {{ testResult.duration_ms }}ms
                </n-descriptions-item>
                <n-descriptions-item v-if="testResult.response_text" label="模型回复">
                  {{ testResult.response_text }}
                </n-descriptions-item>
                <n-descriptions-item v-if="testResult.error" label="错误信息">
                  <n-text type="error">{{ testResult.error }}</n-text>
                </n-descriptions-item>
              </n-descriptions>
            </template>
          </n-result>
        </template>
        <n-space vertical v-if="testModels.length > 0">
          <div
            v-for="m in testModels"
            :key="m.model_name"
            style="display: flex; align-items: center; gap: 8px"
          >
            <n-tag size="small" style="min-width: 120px">{{ m.model_name }}<span v-if="m.context_window && m.context_window !== 272000" style="margin-left: 4px; opacity: 0.6">{{ (m.context_window / 1000).toFixed(0) }}K</span></n-tag>
            <n-button
              size="small"
              type="primary"
              secondary
              :loading="testingModel === m.model_name"
              @click="handleTestModel(m.model_name)"
            >
              测试
            </n-button>
          </div>
        </n-space>
        <n-empty v-else description="该供应商暂无模型" />
      </n-spin>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, h, onMounted } from 'vue'
import { NTag, NPopconfirm, NButton, NSpace, useMessage } from 'naive-ui'
import { api } from '../api'
import type { Provider } from '../types'

interface TestResult {
  success: boolean
  message: string
  response_text: string | null
  duration_ms: number | null
  error: string | null
}

const message = useMessage()
const loading = ref(false)
const providers = ref<Provider[]>([])
const showModal = ref(false)
const isEditing = ref(false)
const editingId = ref('')
const modalLoading = ref(false)

const showTestModal = ref(false)
const testProviderName = ref('')
const testModels = ref<Array<{ model_name: string; context_window: number | null }>>([])
const testing = ref(false)
const testingModel = ref('')
const testResult = ref<TestResult | null>(null)

const form = ref({
  name: '',
  base_url: '',
  format: 'completions' as string,
  endpoint_path: '',
  api_key: '',
  models: [] as Array<{ model_name: string; context_window: number | null }>,
})

const formatOptions = [
  { label: 'OpenAI Completions', value: 'completions' },
  { label: 'OpenAI Responses', value: 'responses' },
  { label: 'Anthropic', value: 'anthropic' },
]

const formatColorMap: Record<string, string> = {
  completions: 'success',
  responses: 'info',
  anthropic: 'warning',
  gemini: 'purple',
}

const columns = [
  { title: '名称', key: 'name', width: 160 },
  { title: 'Base URL', key: 'base_url', ellipsis: { tooltip: true } },
  {
    title: '格式',
    key: 'format',
    width: 140,
    render: (row: Provider) => {
      const color = formatColorMap[row.format] || 'default'
      return h(NTag, { size: 'small', type: color as never }, () => row.format)
    },
  },
  {
    title: '模型数',
    key: 'models_count',
    width: 80,
    render: (row: Provider) => row.models.length,
  },
  {
    title: '操作',
    key: 'actions',
    width: 220,
    render: (row: Provider) =>
      h(NSpace, { size: 4 }, () => [
        h(
          NButton,
          { size: 'small', quaternary: true, type: 'info', onClick: () => openTestModal(row) },
          () => '测试'
        ),
        h(
          NButton,
          { size: 'small', quaternary: true, type: 'primary', onClick: () => openEditModal(row) },
          () => '编辑'
        ),
        h(
          NPopconfirm,
          { onPositiveClick: () => handleDelete(row.id) },
          {
            trigger: () =>
              h(NButton, { size: 'small', quaternary: true, type: 'error' }, () => '删除'),
            default: () => '确认删除此供应商？',
          }
        ),
      ]),
  },
]

function addModel() {
  form.value.models = [
    ...form.value.models,
    { model_name: '', context_window: null },
  ]
}

function removeModel(index: number) {
  form.value.models = form.value.models.filter((_, i) => i !== index)
}

function openCreateModal() {
  isEditing.value = false
  editingId.value = ''
  form.value = {
    name: '',
    base_url: '',
    format: 'completions',
    endpoint_path: '',
    api_key: '',
    models: [],
  }
  showModal.value = true
}

function openEditModal(row: Provider) {
  isEditing.value = true
  editingId.value = row.id
  form.value = {
    name: row.name,
    base_url: row.base_url,
    format: row.format,
    endpoint_path: row.endpoint_path || '',
    api_key: '',
    models: row.models.map((m) => ({
      model_name: m.model_name,
      context_window: m.context_window ?? null,
    })),
  }
  showModal.value = true
}

function openTestModal(row: Provider) {
  testProviderName.value = row.name
  testModels.value = row.models.map((m) => ({ model_name: m.model_name, context_window: m.context_window ?? null }))
  testResult.value = null
  testingModel.value = ''
  showTestModal.value = true
}

async function handleTestModel(modelName: string) {
  testingModel.value = modelName
  testResult.value = null
  testing.value = true

  try {
    testResult.value = await api<TestResult>('/api/models/test', {
      method: 'POST',
      body: JSON.stringify({ model_name: modelName }),
    })
  } catch (e) {
    testResult.value = {
      success: false,
      message: '请求失败',
      response_text: null,
      duration_ms: null,
      error: e instanceof Error ? e.message : String(e),
    }
  } finally {
    testing.value = false
    testingModel.value = ''
  }
}

async function handleSubmit() {
  if (!form.value.name || !form.value.base_url) {
    message.warning('请填写必填字段')
    return false
  }
  if (!isEditing.value && !form.value.api_key) {
    message.warning('请输入 API Key')
    return false
  }

  modalLoading.value = true
  try {
    if (isEditing.value) {
      const body: Record<string, unknown> = {
        name: form.value.name,
        base_url: form.value.base_url,
        format: form.value.format,
        endpoint_path: form.value.endpoint_path || null,
        models: form.value.models.map((m) => ({
          model_name: m.model_name,
          target_model: null,
          context_window: m.context_window,
        })),
      }
      if (form.value.api_key) {
        body.api_key = form.value.api_key
      }
      await api(`/api/providers/${editingId.value}`, {
        method: 'PUT',
        body: JSON.stringify(body),
      })
      message.success('供应商更新成功')
    } else {
      await api('/api/providers', {
        method: 'POST',
        body: JSON.stringify({
          name: form.value.name,
          base_url: form.value.base_url,
          format: form.value.format,
          endpoint_path: form.value.endpoint_path || null,
          api_key: form.value.api_key,
          models: form.value.models.map((m) => ({
            model_name: m.model_name,
            target_model: null,
          })),
        }),
      })
      message.success('供应商创建成功')
    }
    showModal.value = false
    await fetchProviders()
  } catch (err) {
    message.error(`${isEditing.value ? '更新' : '创建'}失败: ${err}`)
  } finally {
    modalLoading.value = false
  }
  return false
}

async function handleDelete(id: string) {
  try {
    await api(`/api/providers/${id}`, { method: 'DELETE' })
    message.success('供应商已删除')
    await fetchProviders()
  } catch (err) {
    message.error(`删除失败: ${err}`)
  }
}

async function fetchProviders() {
  loading.value = true
  try {
    providers.value = await api<Provider[]>('/api/providers')
  } catch (err) {
    console.error('Failed to load providers:', err)
  } finally {
    loading.value = false
  }
}

onMounted(fetchProviders)
</script>
