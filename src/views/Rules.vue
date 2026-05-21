<template>
  <div class="rules-view">
    <n-space style="margin-bottom: 16px" justify="space-between" align="center">
      <n-h3 style="margin: 0">{{ t('rules.title') }}</n-h3>
      <n-button type="primary" @click="openAddModal">{{ t('rules.add') }}</n-button>
    </n-space>

    <n-data-table
      :columns="columns"
      :data="rules"
      :bordered="false"
      :row-key="(row: InterceptorRule) => row.id"
      size="small"
    />

    <n-modal v-model:show="showAddModal" :title="t('rules.addRule')" preset="card" style="width: 560px">
      <n-form :model="newRule" label-placement="left" label-width="120">
        <n-form-item :label="t('common.name')">
          <n-input v-model:value="newRule.name" placeholder="Rule name" />
        </n-form-item>
        <n-form-item :label="t('rules.phase')">
          <n-select v-model:value="newRule.phase" :options="phaseOptions" />
        </n-form-item>
        <n-form-item :label="t('rules.conditionType')">
          <n-select v-model:value="newRule.conditionType" :options="conditionTypeOptions" @update:value="resetCondition" />
        </n-form-item>
        <n-form-item v-if="newRule.conditionType === 'model_matches'" label="Pattern">
          <n-input v-model:value="newRule.conditionData.pattern" placeholder="gpt-4*" />
        </n-form-item>
        <n-form-item v-if="newRule.conditionType === 'path_contains'" label="Substring">
          <n-input v-model:value="newRule.conditionData.substring" placeholder="/v1/chat" />
        </n-form-item>
        <n-form-item v-if="newRule.conditionType === 'header_exists'" label="Header Name">
          <n-input v-model:value="newRule.conditionData.name" placeholder="X-Custom" />
        </n-form-item>
        <n-form-item :label="t('rules.actionType')">
          <n-select v-model:value="newRule.actionType" :options="actionTypeOptions" @update:value="resetAction" />
        </n-form-item>
        <n-form-item v-if="newRule.actionType === 'replace_model'" label="New Model">
          <n-input v-model:value="newRule.actionData.new_model" placeholder="gpt-4-turbo" />
        </n-form-item>
        <n-form-item v-if="newRule.actionType === 'set_header'" label="Header Name">
          <n-input v-model:value="newRule.actionData.name" placeholder="X-Custom" />
        </n-form-item>
        <n-form-item v-if="newRule.actionType === 'set_header'" label="Header Value">
          <n-input v-model:value="newRule.actionData.value" placeholder="value" />
        </n-form-item>
        <n-form-item v-if="newRule.actionType === 'remove_header'" label="Header Name">
          <n-input v-model:value="newRule.actionData.name" placeholder="X-Remove-Me" />
        </n-form-item>
        <n-form-item v-if="newRule.actionType === 'inject_system_prompt'" label="Prompt Text">
          <n-input v-model:value="newRule.actionData.text" type="textarea" placeholder="System prompt to inject" :rows="3" />
        </n-form-item>
        <n-form-item v-if="newRule.actionType === 'override_parameter'" label="Parameter Key">
          <n-input v-model:value="newRule.actionData.key" placeholder="temperature" />
        </n-form-item>
        <n-form-item v-if="newRule.actionType === 'override_parameter'" label="Parameter Value">
          <n-input v-model:value="newRule.actionData.value" placeholder="0.7" />
        </n-form-item>
        <n-form-item v-if="newRule.actionType === 'filter_response'" label="Pattern">
          <n-input v-model:value="newRule.actionData.pattern" placeholder="regex pattern" />
        </n-form-item>
        <n-form-item v-if="newRule.actionType === 'filter_response'" label="Replacement">
          <n-input v-model:value="newRule.actionData.replacement" placeholder="replacement text" />
        </n-form-item>
        <n-form-item :label="t('common.priority')">
          <n-input-number v-model:value="newRule.priority" :min="0" :max="1000" style="width: 100%" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-space>
          <n-button @click="showAddModal = false">{{ t('common.cancel') }}</n-button>
          <n-button type="primary" @click="handleCreateRule" :loading="creating">{{ t('common.create') }}</n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, h, computed, onMounted } from 'vue'
import {
  NDataTable, NButton, NModal, NForm, NFormItem, NInput, NInputNumber, NSelect, NSpace, NTag, NSwitch,
  type DataTableColumns,
} from 'naive-ui'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { InterceptorRule } from '../types'

const { t } = useI18n()
const message = useMessage()
const rules = ref<InterceptorRule[]>([])
const showAddModal = ref(false)
const creating = ref(false)

const phaseOptions = computed(() => [
  { label: t('rules.preRequest'), value: 'pre' },
  { label: t('rules.postResponse'), value: 'post' },
])

const conditionTypeOptions = computed(() => [
  { label: t('rules.conditionModel'), value: 'model_matches' },
  { label: t('rules.conditionPath'), value: 'path_contains' },
  { label: t('rules.conditionHeader'), value: 'header_exists' },
  { label: t('rules.conditionAlways'), value: 'always' },
])

const actionTypeOptions = computed(() => [
  { label: t('rules.actionReplaceModel'), value: 'replace_model' },
  { label: t('rules.actionSetHeader'), value: 'set_header' },
  { label: t('rules.actionRemoveHeader'), value: 'remove_header' },
  { label: t('rules.actionInjectPrompt'), value: 'inject_system_prompt' },
  { label: t('rules.actionOverrideParam'), value: 'override_parameter' },
  { label: t('rules.actionFilterResponse'), value: 'filter_response' },
])

const newRule = ref({
  name: '',
  phase: 'pre',
  conditionType: 'model_matches',
  conditionData: {} as Record<string, string>,
  actionType: 'replace_model',
  actionData: {} as Record<string, string>,
  priority: 10,
})

function resetCondition() {
  newRule.value.conditionData = {}
}

function resetAction() {
  newRule.value.actionData = {}
}

function openAddModal() {
  newRule.value = {
    name: '',
    phase: 'pre',
    conditionType: 'model_matches',
    conditionData: {},
    actionType: 'replace_model',
    actionData: {},
    priority: 10,
  }
  showAddModal.value = true
}

function conditionSummary(rule: InterceptorRule): string {
  const c = rule.condition
  switch (c.type) {
    case 'model_matches': return t('rules.summaryModelPattern', { pattern: c.pattern })
    case 'path_contains': return t('rules.summaryPathContains', { substring: c.substring })
    case 'header_exists': return t('rules.summaryHeaderExists', { name: c.name })
    case 'always': return t('rules.summaryAlways')
  }
}

function actionSummary(rule: InterceptorRule): string {
  const a = rule.action
  switch (a.type) {
    case 'replace_model': return t('rules.summaryReplaceModel', { model: a.new_model })
    case 'set_header': return t('rules.summarySetHeader', { name: a.name })
    case 'remove_header': return t('rules.summaryRemoveHeader', { name: a.name })
    case 'inject_system_prompt': return t('rules.summaryInjectPrompt')
    case 'override_parameter': return t('rules.summaryOverrideParam', { key: a.key, value: String(a.value) })
    case 'filter_response': return t('rules.summaryFilterResponse', { pattern: a.pattern })
  }
}

const columns: DataTableColumns<InterceptorRule> = [
  { title: t('common.name'), key: 'name', width: 180 },
  { title: t('rules.phase'), key: 'phase', width: 80, render: (row) => h(NTag, { size: 'small', type: row.phase === 'pre' ? 'info' : 'warning' }, { default: () => row.phase }) },
  { title: t('rules.condition'), key: 'condition', render: (row) => conditionSummary(row) },
  { title: t('rules.action'), key: 'action', render: (row) => actionSummary(row) },
  { title: t('common.priority'), key: 'priority', width: 80 },
  {
    title: t('common.enabled'), key: 'enabled', width: 80,
    render: (row) => h(NSwitch, { value: row.enabled, onUpdateValue: (v: boolean) => handleToggle(row.id, v) }),
  },
  {
    title: '', key: 'actions', width: 80,
    render: (row) => h(NButton, { size: 'small', type: 'error', quaternary: true, onClick: () => handleDeleteRule(row.id) }, { default: () => t('common.delete') }),
  },
]

async function loadRules() {
  try {
    rules.value = await invoke<InterceptorRule[]>('get_rules')
  } catch (e) {
    message.error(t('rules.loadFailed', { error: String(e) }))
  }
}

async function handleToggle(id: string, enabled: boolean) {
  try {
    await invoke('update_rule', { id, enabled })
    await loadRules()
  } catch (e) {
    message.error(t('rules.createFailed', { error: String(e) }))
  }
}

async function handleDeleteRule(id: string) {
  try {
    await invoke('delete_rule', { id })
    message.success(t('rules.deleted'))
    await loadRules()
  } catch (e) {
    message.error(t('rules.deleteFailed', { error: String(e) }))
  }
}

async function handleCreateRule() {
  creating.value = true
  try {
    const conditionJson = JSON.stringify({ type: newRule.value.conditionType, ...newRule.value.conditionData })
    const actionJson = JSON.stringify({ type: newRule.value.actionType, ...newRule.value.actionData })
    await invoke('create_rule', {
      name: newRule.value.name,
      phase: newRule.value.phase,
      conditionJson,
      actionJson,
      priority: newRule.value.priority,
    })
    message.success(t('rules.created'))
    showAddModal.value = false
    await loadRules()
  } catch (e) {
    message.error(t('rules.createFailed', { error: String(e) }))
  } finally {
    creating.value = false
  }
}

onMounted(loadRules)
</script>
