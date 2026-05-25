<template>
  <n-space vertical size="large">
    <n-card>
      <template #header>
        <n-text strong>应用管理</n-text>
      </template>

      <n-spin :show="loading">
        <div class="app-grid">
          <n-card
            v-for="app in apps"
            :key="app.app_type"
            size="small"
            hoverable
          >
            <template #header>
              <n-space justify="space-between" align="center">
                <n-text strong>{{ displayName(app.app_type) }}</n-text>
                <n-button
                  quaternary
                  size="small"
                  @click="openPathModal(app)"
                >
                  <template #icon>
                    <n-icon><SettingsOutline /></n-icon>
                  </template>
                </n-button>
              </n-space>
            </template>

            <n-space vertical size="small">
              <n-tag
                :type="app.installed ? 'success' : 'error'"
                size="small"
              >
                {{ app.installed ? '已安装' : '未安装' }}
              </n-tag>

              <n-text
                v-if="app.install_path"
                depth="3"
                style="font-size: 12px; word-break: break-all"
              >
                {{ app.install_path }}
              </n-text>

              <n-tag v-if="app.model" size="small" type="info">
                {{ app.model }}
              </n-tag>

              <n-text
                v-if="app.launched_at"
                depth="3"
                style="font-size: 12px"
              >
                上次启动: {{ formatTime(app.launched_at) }}
              </n-text>

              <n-button
                type="primary"
                size="small"
                block
                :disabled="!app.installed"
                @click="openLaunchModal(app)"
              >
                打开
              </n-button>
            </n-space>
          </n-card>
        </div>
      </n-spin>
    </n-card>

    <n-modal
      v-model:show="showLaunchModal"
      preset="dialog"
      title="启动应用"
      positive-text="启动"
      negative-text="取消"
      :loading="launchLoading"
      @positive-click="handleLaunch"
      style="width: 480px"
    >
      <n-space vertical size="medium">
        <n-text>应用: {{ launchForm.appName }}</n-text>
        <n-select
          v-model:value="launchForm.model"
          :options="modelOptions"
          filterable
          tag
          placeholder="选择或输入模型"
        />
      </n-space>
    </n-modal>

    <n-modal
      v-model:show="showPathModal"
      preset="dialog"
      title="配置安装路径"
      positive-text="保存"
      negative-text="取消"
      :loading="pathLoading"
      @positive-click="handleSetPath"
      style="width: 480px"
    >
      <n-space vertical size="medium">
        <n-text>应用: {{ pathForm.appName }}</n-text>
        <n-input
          v-model:value="pathForm.install_path"
          placeholder="留空则自动检测"
        />
      </n-space>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useMessage } from 'naive-ui'
import { SettingsOutline } from '@vicons/ionicons5'
import { api } from '../api'
import type { AppConfig, AppType, Provider, ProviderModel } from '../types'

const message = useMessage()
const loading = ref(false)
const apps = ref<AppConfig[]>([])
const allModels = ref<ProviderModel[]>([])

const showLaunchModal = ref(false)
const launchLoading = ref(false)
const launchForm = ref({
  appType: '' as AppType,
  appName: '',
  model: '',
})

const showPathModal = ref(false)
const pathLoading = ref(false)
const pathForm = ref({
  appType: '' as AppType,
  appName: '',
  install_path: '',
})

const displayNameMap: Record<AppType, string> = {
  codex_cli: 'Codex CLI',
  codex_desktop: 'Codex Desktop',
  claude_cli: 'Claude CLI',
  claude_desktop: 'Claude Desktop',
}

function displayName(appType: AppType): string {
  return displayNameMap[appType] || appType
}

function formatTime(iso: string): string {
  return new Date(iso).toLocaleString('zh-CN')
}

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

async function fetchApps() {
  loading.value = true
  try {
    apps.value = await api<AppConfig[]>('/api/apps')
  } catch (err) {
    console.error('Failed to load apps:', err)
  } finally {
    loading.value = false
  }
}

async function fetchModels() {
  try {
    const providers = await api<Provider[]>('/api/providers')
    allModels.value = providers.flatMap((p) => p.models)
  } catch (err) {
    console.error('Failed to load models:', err)
  }
}

function openLaunchModal(app: AppConfig) {
  launchForm.value = {
    appType: app.app_type,
    appName: displayName(app.app_type),
    model: app.model || '',
  }
  showLaunchModal.value = true
}

async function handleLaunch() {
  if (!launchForm.value.model) {
    message.warning('请选择或输入模型')
    return false
  }

  launchLoading.value = true
  try {
    await api<void>('/api/apps/launch', {
      method: 'POST',
      body: JSON.stringify({
        app_type: launchForm.value.appType,
        model: launchForm.value.model,
      }),
    })
    message.success('应用启动成功')
    showLaunchModal.value = false
    await fetchApps()
  } catch (err) {
    message.error(`启动失败: ${err}`)
  } finally {
    launchLoading.value = false
  }
  return false
}

function openPathModal(app: AppConfig) {
  pathForm.value = {
    appType: app.app_type,
    appName: displayName(app.app_type),
    install_path: app.install_path || '',
  }
  showPathModal.value = true
}

async function handleSetPath() {
  pathLoading.value = true
  try {
    await api<void>(`/api/apps/${pathForm.value.appType}/path`, {
      method: 'PUT',
      body: JSON.stringify({
        install_path: pathForm.value.install_path,
      }),
    })
    message.success('路径配置已保存')
    showPathModal.value = false
    await fetchApps()
  } catch (err) {
    message.error(`保存失败: ${err}`)
  } finally {
    pathLoading.value = false
  }
  return false
}

onMounted(() => {
  fetchApps()
  fetchModels()
})
</script>

<style scoped>
.app-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}
</style>
