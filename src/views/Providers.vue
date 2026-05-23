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
          <n-input v-model:value="form.name" placeholder="例如: OpenAI" />
        </n-form-item>
        <n-form-item label="Base URL" required>
          <n-input v-model:value="form.base_url" placeholder="例如: https://api.openai.com" />
        </n-form-item>
        <n-form-item label="格式">
          <n-select
            v-model:value="form.format"
            :options="formatOptions"
            placeholder="选择格式"
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
          <n-space
            v-for="(_, index) in form.models"
            :key="index"
            align="center"
            size="small"
          >
            <n-input
              v-model:value="form.models[index].model_name"
              placeholder="模型名称"
              style="width: 200px"
            />
            <n-input
              v-model:value="form.models[index].target_model"
              placeholder="上游模型 (可选)"
              style="width: 200px"
            />
            <n-button
              quaternary
              type="error"
              size="small"
              @click="removeModel(index)"
            >
              删除
            </n-button>
          </n-space>
          <n-button dashed size="small" @click="addModel">
            + 添加模型
          </n-button>
        </n-space>
      </n-form>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, h, onMounted } from 'vue'
import { NTag, NPopconfirm, NButton, NSpace, useMessage } from 'naive-ui'
import { api } from '../api'
import type { Provider } from '../types'

const message = useMessage()
const loading = ref(false)
const providers = ref<Provider[]>([])
const showModal = ref(false)
const isEditing = ref(false)
const editingId = ref('')
const modalLoading = ref(false)

const form = ref({
  name: '',
  base_url: '',
  format: 'completions' as string,
  api_key: '',
  models: [] as Array<{ model_name: string; target_model: string }>,
})

const formatOptions = [
  { label: 'OpenAI Completions', value: 'completions' },
  { label: 'OpenAI Responses', value: 'responses' },
  { label: 'Anthropic', value: 'anthropic' },
  { label: 'Google Gemini', value: 'gemini' },
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
    width: 180,
    render: (row: Provider) =>
      h(NSpace, { size: 4 }, () => [
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
    { model_name: '', target_model: '' },
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
    api_key: '',
    models: row.models.map((m) => ({
      model_name: m.model_name,
      target_model: m.target_model || '',
    })),
  }
  showModal.value = true
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
        models: form.value.models.map((m) => ({
          model_name: m.model_name,
          target_model: m.target_model || null,
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
          api_key: form.value.api_key,
          models: form.value.models.map((m) => ({
            model_name: m.model_name,
            target_model: m.target_model || null,
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
