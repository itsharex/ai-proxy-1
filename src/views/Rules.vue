<template>
  <n-space vertical size="large">
    <n-card>
      <template #header>
        <n-space justify="space-between" align="center">
          <n-text strong>拦截规则</n-text>
          <n-button type="primary" @click="openAddModal">
            添加规则
          </n-button>
        </n-space>
      </template>
      <n-data-table
        :columns="columns"
        :data="rules"
        :loading="loading"
        :bordered="false"
      />
    </n-card>

    <n-modal
      v-model:show="showModal"
      preset="dialog"
      :title="isEditing ? '编辑规则' : '添加规则'"
      positive-text="确认"
      negative-text="取消"
      @positive-click="handleSubmit"
      style="width: 520px"
    >
      <n-form label-placement="left" label-width="100">
        <n-form-item label="名称">
          <n-input v-model:value="formData.name" placeholder="规则名称" />
        </n-form-item>
        <n-form-item label="阶段">
          <n-select
            v-model:value="formData.phase"
            :options="phaseOptions"
            placeholder="选择阶段"
          />
        </n-form-item>

        <n-divider style="margin: 8px 0 12px">条件配置</n-divider>
        <n-form-item label="条件类型">
          <n-select
            v-model:value="formData.condition.type"
            :options="conditionTypeOptions"
          />
        </n-form-item>
        <n-form-item v-if="formData.condition.type === 'model_matches'" label="模型匹配">
          <n-input
            v-model:value="formData.condition.pattern"
            placeholder="例如: *sonnet*, *opus*, gpt-4o*, gpt-4o, *"
          />
        </n-form-item>
        <n-form-item v-if="formData.condition.type === 'path_contains'" label="路径包含">
          <n-input
            v-model:value="formData.condition.substring"
            placeholder="例如: /chat/completions"
          />
        </n-form-item>
        <n-form-item v-if="formData.condition.type === 'header_exists'" label="Header 名称">
          <n-input
            v-model:value="formData.condition.headerName"
            placeholder="例如: Authorization"
          />
        </n-form-item>
        <n-form-item v-if="formData.condition.type === 'always'" label=" ">
          <n-text depth="3">匹配所有请求，无需额外配置</n-text>
        </n-form-item>

        <n-divider style="margin: 8px 0 12px">动作配置</n-divider>
        <n-form-item label="动作类型">
          <n-select
            v-model:value="formData.action.type"
            :options="actionTypeOptions"
          />
        </n-form-item>
        <n-form-item v-if="formData.action.type === 'replace_model'" label="目标模型">
          <n-select
            v-model:value="formData.action.model"
            :options="modelOptions"
            placeholder="选择模型"
            filterable
            clearable
            tag
          />
        </n-form-item>
        <template v-if="formData.action.type === 'set_header'">
          <n-form-item label="Header 名称">
            <n-input v-model:value="formData.action.headerName" placeholder="例如: X-Custom" />
          </n-form-item>
          <n-form-item label="Header 值">
            <n-input v-model:value="formData.action.headerValue" placeholder="Header 值" />
          </n-form-item>
        </template>
        <n-form-item v-if="formData.action.type === 'remove_header'" label="Header 名称">
          <n-input v-model:value="formData.action.headerName" placeholder="例如: X-Unwanted" />
        </n-form-item>
        <n-form-item v-if="formData.action.type === 'inject_system_prompt'" label="系统提示">
          <n-input
            v-model:value="formData.action.prompt"
            type="textarea"
            :rows="3"
            placeholder="输入要注入的系统提示内容"
          />
        </n-form-item>
        <template v-if="formData.action.type === 'override_parameter'">
          <n-form-item label="参数名">
            <n-select
              v-model:value="formData.action.parameter"
              :options="parameterOptions"
            />
          </n-form-item>
          <n-form-item label="参数值">
            <n-switch
              v-if="formData.action.parameter === 'stream'"
              v-model:value="streamBool"
            />
            <n-input-number
              v-else-if="['temperature', 'top_p'].includes(formData.action.parameter)"
              v-model:value="paramNumber"
              :min="0"
              :max="2"
              :step="0.1"
              style="width: 100%"
            />
            <n-input-number
              v-else
              v-model:value="paramNumber"
              :min="1"
              :max="128000"
              :step="1"
              style="width: 100%"
            />
          </n-form-item>
        </template>
        <template v-if="formData.action.type === 'filter_response'">
          <n-form-item label="过滤模式">
            <n-space vertical size="small" style="width: 100%">
              <div
                v-for="(_, index) in formData.action.patterns"
                :key="index"
                style="display: flex; align-items: center; gap: 8px"
              >
                <n-input
                  v-model:value="formData.action.patterns[index]"
                  placeholder="过滤关键词"
                  style="flex: 1"
                />
                <n-button
                  quaternary
                  type="error"
                  size="small"
                  @click="formData.action.patterns.splice(index, 1)"
                >
                  删除
                </n-button>
              </div>
              <n-button dashed size="small" @click="formData.action.patterns.push('')">
                + 添加模式
              </n-button>
            </n-space>
          </n-form-item>
        </template>

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
import { ref, computed, watch, onMounted, h } from 'vue'
import { api } from '../api'
import { NTag, NPopconfirm, NButton, NSwitch, NSpace, useMessage } from 'naive-ui'
import type { InterceptorRule, RuleCondition, RuleAction, Provider } from '../types'

const message = useMessage()
const loading = ref(false)
const rules = ref<InterceptorRule[]>([])
const showModal = ref(false)
const isEditing = ref(false)
const editingId = ref('')

const phaseOptions = [
  { label: 'Pre (请求前)', value: 'pre' },
  { label: 'Post (请求后)', value: 'post' },
]

const conditionTypeOptions = [
  { label: '始终匹配', value: 'always' },
  { label: '模型匹配', value: 'model_matches' },
  { label: '路径包含', value: 'path_contains' },
  { label: '请求头存在', value: 'header_exists' },
]

const actionTypeOptions = [
  { label: '替换模型', value: 'replace_model' },
  { label: '设置请求头', value: 'set_header' },
  { label: '移除请求头', value: 'remove_header' },
  { label: '注入系统提示', value: 'inject_system_prompt' },
  { label: '覆盖参数', value: 'override_parameter' },
  { label: '过滤响应', value: 'filter_response' },
]

const parameterOptions = [
  { label: 'temperature', value: 'temperature' },
  { label: 'top_p', value: 'top_p' },
  { label: 'max_tokens', value: 'max_tokens' },
  { label: 'stream', value: 'stream' },
]

const allModels = ref<{ model_name: string }[]>([])

const modelOptions = computed(() => {
  const seen = new Set<string>()
  const options: { label: string; value: string }[] = []
  for (const m of allModels.value) {
    if (!seen.has(m.model_name)) {
      seen.add(m.model_name)
      options.push({ label: m.model_name, value: m.model_name })
    }
  }
  return options
})

async function fetchModels() {
  try {
    const providers = await api<Provider[]>('/api/providers')
    allModels.value = providers.flatMap((p) => p.models)
  } catch (err) {
    console.error('Failed to load models:', err)
  }
}

const formData = ref({
  name: '',
  phase: 'pre' as 'pre' | 'post',
  condition: {
    type: 'always',
    pattern: '',
    substring: '',
    headerName: '',
  },
  action: {
    type: 'replace_model' as string,
    model: '',
    headerName: '',
    headerValue: '',
    prompt: '',
    parameter: 'temperature',
    parameterValue: '0.7',
    patterns: [''] as string[],
  },
  priority: 0,
})

const streamBool = computed({
  get: () => formData.value.action.parameterValue === 'true',
  set: (val: boolean) => { formData.value.action.parameterValue = String(val) },
})

const paramNumber = computed({
  get: () => {
    const raw = formData.value.action.parameterValue
    const num = parseFloat(raw)
    return isNaN(num) ? 0 : num
  },
  set: (val: number | null) => {
    formData.value.action.parameterValue = String(val ?? 0)
  },
})

watch(
  () => formData.value.action.parameter,
  (param) => {
    if (param === 'stream') {
      formData.value.action.parameterValue = 'false'
    } else if (param === 'max_tokens') {
      formData.value.action.parameterValue = '4096'
    } else {
      formData.value.action.parameterValue = '0.7'
    }
  },
)

const conditionTypeLabels: Record<string, string> = {
  always: '始终匹配',
  model_matches: '模型匹配',
  path_contains: '路径包含',
  header_exists: '请求头存在',
}

const actionTypeLabels: Record<string, string> = {
  replace_model: '替换模型',
  set_header: '设置请求头',
  remove_header: '移除请求头',
  inject_system_prompt: '注入系统提示',
  override_parameter: '覆盖参数',
  filter_response: '过滤响应',
}

const columns = [
  { title: '名称', key: 'name', width: 160 },
  {
    title: '阶段',
    key: 'phase',
    width: 100,
    render: (row: InterceptorRule) =>
      h(NTag, { size: 'small', type: row.phase === 'pre' ? 'info' : 'warning' }, () => row.phase),
  },
  {
    title: '条件类型',
    key: 'condition_type',
    width: 120,
    render: (row: InterceptorRule) => conditionTypeLabels[row.condition.type] ?? row.condition.type,
  },
  {
    title: '动作类型',
    key: 'action_type',
    width: 120,
    render: (row: InterceptorRule) => actionTypeLabels[row.action.type] ?? row.action.type,
  },
  { title: '优先级', key: 'priority', width: 80 },
  {
    title: '启用',
    key: 'enabled',
    width: 80,
    render: (row: InterceptorRule) =>
      h(NSwitch, {
        value: row.enabled,
        onUpdateValue: (val: boolean) => handleToggle(row.id, val),
      }),
  },
  {
    title: '操作',
    key: 'actions',
    width: 140,
    render: (row: InterceptorRule) =>
      h(NSpace, { size: 4 }, () => [
        h(NButton, {
          size: 'small',
          quaternary: true,
          type: 'primary',
          onClick: () => openEditModal(row),
        }, () => '编辑'),
        h(NPopconfirm, { onPositiveClick: () => handleDelete(row.id) }, {
          trigger: () => h(NButton, { size: 'small', type: 'error', quaternary: true }, () => '删除'),
          default: () => '确认删除此规则？',
        }),
      ]),
  },
]

function buildCondition(): RuleCondition {
  const c = formData.value.condition
  switch (c.type) {
    case 'model_matches': return { type: 'model_matches', pattern: c.pattern }
    case 'path_contains': return { type: 'path_contains', substring: c.substring }
    case 'header_exists': return { type: 'header_exists', name: c.headerName }
    default: return { type: 'always' }
  }
}

function buildAction(): RuleAction {
  const a = formData.value.action
  switch (a.type) {
    case 'replace_model':
      return { type: 'replace_model', model: a.model }
    case 'set_header':
      return { type: 'set_header', name: a.headerName, value: a.headerValue }
    case 'remove_header':
      return { type: 'remove_header', name: a.headerName }
    case 'inject_system_prompt':
      return { type: 'inject_system_prompt', prompt: a.prompt }
    case 'override_parameter':
      return {
        type: 'override_parameter',
        parameter: a.parameter,
        value: parseParameterValue(a.parameter, a.parameterValue),
      }
    case 'filter_response':
      return { type: 'filter_response', patterns: a.patterns.filter(p => p.trim()) }
    default:
      return { type: 'replace_model', model: a.model }
  }
}

function parseParameterValue(parameter: string, raw: string): unknown {
  if (parameter === 'stream') return raw === 'true'
  if (parameter === 'temperature' || parameter === 'top_p') {
    const num = parseFloat(raw)
    return isNaN(num) ? 0 : num
  }
  if (parameter === 'max_tokens') {
    const num = parseInt(raw, 10)
    return isNaN(num) ? 0 : num
  }
  try { return JSON.parse(raw) } catch { return raw }
}

function loadConditionToForm(condition: RuleCondition) {
  const c = formData.value.condition
  c.type = condition.type
  c.pattern = ''
  c.substring = ''
  c.headerName = ''
  switch (condition.type) {
    case 'model_matches': c.pattern = condition.pattern; break
    case 'path_contains': c.substring = condition.substring; break
    case 'header_exists': c.headerName = condition.name; break
  }
}

function loadActionToForm(action: RuleAction) {
  const a = formData.value.action
  a.type = action.type
  a.model = ''
  a.headerName = ''
  a.headerValue = ''
  a.prompt = ''
  a.parameter = 'temperature'
  a.parameterValue = '0.7'
  a.patterns = ['']
  switch (action.type) {
    case 'replace_model': a.model = action.model; break
    case 'set_header': a.headerName = action.name; a.headerValue = action.value; break
    case 'remove_header': a.headerName = action.name; break
    case 'inject_system_prompt': a.prompt = action.prompt; break
    case 'override_parameter':
      a.parameter = action.parameter
      a.parameterValue = formatParameterValue(action.parameter, action.value)
      break
    case 'filter_response':
      a.patterns = action.patterns.length > 0 ? [...action.patterns] : ['']
      break
  }
}

function formatParameterValue(parameter: string, value: unknown): string {
  if (parameter === 'stream') return String(value)
  return String(value ?? '')
}

function getDefaultForm() {
  return {
    name: '',
    phase: 'pre' as 'pre' | 'post',
    condition: { type: 'always', pattern: '', substring: '', headerName: '' },
    action: {
      type: 'replace_model',
      model: '',
      headerName: '',
      headerValue: '',
      prompt: '',
      parameter: 'temperature',
      parameterValue: '0.7',
      patterns: [''],
    },
    priority: 0,
  }
}

function openAddModal() {
  isEditing.value = false
  editingId.value = ''
  formData.value = getDefaultForm()
  fetchModels()
  showModal.value = true
}

function openEditModal(row: InterceptorRule) {
  isEditing.value = true
  editingId.value = row.id
  formData.value = getDefaultForm()
  formData.value.name = row.name
  formData.value.phase = row.phase
  formData.value.priority = row.priority
  loadConditionToForm(row.condition)
  loadActionToForm(row.action)
  fetchModels()
  showModal.value = true
}

async function handleSubmit() {
  const condition = buildCondition()
  const action = buildAction()

  try {
    const body = JSON.stringify({
      name: formData.value.name,
      phase: formData.value.phase,
      condition,
      action,
      priority: formData.value.priority,
    })
    if (isEditing.value) {
      await api(`/api/rules/${editingId.value}`, { method: 'PUT', body })
      message.success('规则更新成功')
    } else {
      await api('/api/rules', { method: 'POST', body })
      message.success('规则添加成功')
    }
    await fetchRules()
  } catch (error) {
    message.error(`${isEditing.value ? '更新' : '添加'}失败: ${error}`)
  }
  return true
}

async function handleToggle(id: string, enabled: boolean) {
  try {
    await api(`/api/rules/${id}`, {
      method: 'PUT',
      body: JSON.stringify({ enabled }),
    })
    rules.value = rules.value.map((r) =>
      r.id === id ? { ...r, enabled } : r
    )
  } catch (error) {
    message.error(`更新失败: ${error}`)
  }
}

async function handleDelete(id: string) {
  try {
    await api(`/api/rules/${id}`, { method: 'DELETE' })
    message.success('规则已删除')
    await fetchRules()
  } catch (error) {
    message.error(`删除失败: ${error}`)
  }
}

async function fetchRules() {
  loading.value = true
  try {
    rules.value = await api<InterceptorRule[]>('/api/rules')
  } catch (error) {
    console.error('Failed to load rules:', error)
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  fetchRules()
})
</script>
