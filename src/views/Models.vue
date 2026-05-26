<template>
  <n-space vertical size="large">
    <n-card title="模型总览">
      <template #header-extra>
        <n-text depth="3">模型在供应商中配置</n-text>
      </template>
      <n-data-table :columns="columns" :data="allModels" />
    </n-card>

    <n-modal v-model:show="showModal" preset="card" title="模型测试" style="width: 500px">
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
        <template v-else>
          <n-text depth="3">正在测试模型: {{ testingModel }}</n-text>
        </template>
      </n-spin>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, h } from 'vue'
import { NTag, NButton } from 'naive-ui'
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

interface TestResult {
  success: boolean
  message: string
  response_text: string | null
  duration_ms: number | null
  error: string | null
}

const showModal = ref(false)
const testing = ref(false)
const testingModel = ref('')
const testResult = ref<TestResult | null>(null)

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
  {
    title: '操作',
    key: 'actions',
    width: 80,
    render(row: FlatModel) {
      return h(NButton, {
        size: 'small',
        type: 'primary',
        secondary: true,
        onClick: () => handleTest(row.model_name),
      }, { default: () => '测试' })
    },
  },
]

async function handleTest(modelName: string) {
  testingModel.value = modelName
  testResult.value = null
  testing.value = true
  showModal.value = true

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
  }
}

async function loadData() {
  try {
    providers.value = await api<Provider[]>('/api/providers')
  } catch (e) {
    console.error('Failed to load providers:', e)
  }
}

onMounted(loadData)
</script>
