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
      title="添加规则"
      positive-text="确认"
      negative-text="取消"
      @positive-click="handleAdd"
    >
      <n-form label-placement="left" label-width="100">
        <n-form-item label="名称">
          <n-input
            v-model:value="formData.name"
            placeholder="规则名称"
          />
        </n-form-item>
        <n-form-item label="阶段">
          <n-select
            v-model:value="formData.phase"
            :options="phaseOptions"
            placeholder="选择阶段"
          />
        </n-form-item>
        <n-form-item label="条件 JSON">
          <n-input
            v-model:value="formData.conditionJson"
            type="textarea"
            :rows="3"
            placeholder='{"type":"model_matches","pattern":"gpt-*"}'
          />
        </n-form-item>
        <n-form-item label="动作 JSON">
          <n-input
            v-model:value="formData.actionJson"
            type="textarea"
            :rows="3"
            placeholder='{"type":"replace_model","new_model":"gpt-4o"}'
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
import { api } from '../api'
import { NTag, NPopconfirm, NButton, NSwitch, NSpace, useMessage } from 'naive-ui'
import type { InterceptorRule, RuleCondition, RuleAction } from '../types'

const message = useMessage()
const loading = ref(false)
const rules = ref<InterceptorRule[]>([])
const showModal = ref(false)

const phaseOptions = [
  { label: 'Pre (请求前)', value: 'pre' },
  { label: 'Post (请求后)', value: 'post' },
]

const formData = ref({
  name: '',
  phase: 'pre' as 'pre' | 'post',
  conditionJson: '{"type":"always"}',
  actionJson: '{"type":"replace_model","new_model":""}',
  priority: 0,
})

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
    width: 140,
    render: (row: InterceptorRule) => row.condition.type,
  },
  {
    title: '动作类型',
    key: 'action_type',
    width: 140,
    render: (row: InterceptorRule) => row.action.type,
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
    width: 100,
    render: (row: InterceptorRule) =>
      h(NPopconfirm, { onPositiveClick: () => handleDelete(row.id) }, {
        trigger: () => h(NButton, { size: 'small', type: 'error', quaternary: true }, () => '删除'),
        default: () => '确认删除此规则？',
      }),
  },
]

function openAddModal() {
  formData.value = {
    name: '',
    phase: 'pre',
    conditionJson: '{"type":"always"}',
    actionJson: '{"type":"replace_model","new_model":""}',
    priority: 0,
  }
  showModal.value = true
}

async function handleAdd() {
  let condition: RuleCondition
  let action: RuleAction
  try {
    condition = JSON.parse(formData.value.conditionJson)
    action = JSON.parse(formData.value.actionJson)
  } catch {
    message.error('JSON 格式无效，请检查条件和动作的 JSON 格式')
    return false
  }

  try {
    await api('/api/rules', {
      method: 'POST',
      body: JSON.stringify({
        name: formData.value.name,
        phase: formData.value.phase,
        condition,
        action,
        priority: formData.value.priority,
      }),
    })
    message.success('规则添加成功')
    await fetchRules()
  } catch (error) {
    message.error(`添加失败: ${error}`)
  }
  return true
}

async function handleToggle(id: string, enabled: boolean) {
  try {
    await api(`/api/rules/${id}`, {
      method: 'PUT',
      body: JSON.stringify({ enabled }),
    })
    const target = rules.value.find((r) => r.id === id)
    if (target) {
      rules.value = rules.value.map((r) =>
        r.id === id ? { ...r, enabled } : r
      )
    }
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
