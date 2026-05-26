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

            <div class="app-card-content">
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
              </n-space>

              <div class="app-card-action">
                <n-button
                  type="primary"
                  size="small"
                  block
                  :disabled="!app.installed"
                  @click="openLaunchModal(app)"
                >
                  打开
                </n-button>
              </div>
            </div>
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
      style="width: 520px"
    >
      <n-space vertical size="medium">
        <n-text>应用: {{ launchForm.appName }}</n-text>

        <n-space vertical size="small" style="width: 100%">
          <n-text depth="3" style="font-size: 13px">默认模型</n-text>
          <n-select
            v-model:value="launchForm.model"
            :options="modelOptions"
            filterable
            tag
            placeholder="选择或输入模型"
          />
        </n-space>

        <template v-if="isClaudeApp(launchForm.appType)">
          <n-space vertical size="small" style="width: 100%">
            <n-text depth="3" style="font-size: 13px">Haiku 模型（可选）</n-text>
            <n-select
              v-model:value="launchForm.model_haiku"
              :options="modelOptionsWithEmpty"
              filterable
              tag
              clearable
              placeholder="选择或输入 Haiku 模型"
            />
          </n-space>

          <n-space vertical size="small" style="width: 100%">
            <n-text depth="3" style="font-size: 13px">Sonnet 模型（可选）</n-text>
            <n-select
              v-model:value="launchForm.model_sonnet"
              :options="modelOptionsWithEmpty"
              filterable
              tag
              clearable
              placeholder="选择或输入 Sonnet 模型"
            />
          </n-space>

          <n-space vertical size="small" style="width: 100%">
            <n-text depth="3" style="font-size: 13px">Opus 模型（可选）</n-text>
            <n-select
              v-model:value="launchForm.model_opus"
              :options="modelOptionsWithEmpty"
              filterable
              tag
              clearable
              placeholder="选择或输入 Opus 模型"
            />
          </n-space>
        </template>

        <template v-if="isCliApp(launchForm.appType)">
          <n-space vertical size="small" style="width: 100%">
            <n-text depth="3" style="font-size: 13px">工作目录</n-text>
            <n-input-group>
              <n-input
                v-model:value="launchForm.work_dir"
                placeholder="留空则使用默认目录"
                style="flex: 1"
              />
              <n-button @click="browseWorkDir">
                浏览
              </n-button>
            </n-input-group>
          </n-space>
        </template>
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
        <n-input-group>
          <n-input
            v-model:value="pathForm.install_path"
            placeholder="留空则自动检测"
            style="flex: 1"
          />
          <n-button @click="browseInstallPath">
            浏览
          </n-button>
        </n-input-group>
      </n-space>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useMessage } from 'naive-ui'
import { SettingsOutline } from '@vicons/ionicons5'
import { open } from '@tauri-apps/plugin-dialog'
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
  model_haiku: null as string | null,
  model_sonnet: null as string | null,
  model_opus: null as string | null,
  work_dir: '',
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

function isClaudeApp(appType: AppType): boolean {
  return appType === 'claude_cli' || appType === 'claude_desktop'
}

function isCliApp(appType: AppType): boolean {
  return appType === 'codex_cli' || appType === 'claude_cli'
}

async function browseInstallPath() {
  const selected = await open({ multiple: false })
  if (typeof selected === 'string') {
    pathForm.value.install_path = selected
  }
}

async function browseWorkDir() {
  const selected = await open({ directory: true, multiple: false })
  if (typeof selected === 'string') {
    launchForm.value.work_dir = selected
  }
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

const modelOptionsWithEmpty = computed(() => {
  return modelOptions.value
})

async function fetchApps() {
  loading.value = true
  try {
    apps.value = (await api<AppConfig[]>('/api/apps'))
      .filter(a => a.app_type !== 'claude_desktop')
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
    model_haiku: app.model_haiku || null,
    model_sonnet: app.model_sonnet || null,
    model_opus: app.model_opus || null,
    work_dir: app.work_dir || '',
  }
  showLaunchModal.value = true
}

async function handleLaunch() {
  if (!launchForm.value.model) {
    message.warning('请选择或输入默认模型')
    return false
  }

  launchLoading.value = true
  try {
    const body: Record<string, unknown> = {
      app_type: launchForm.value.appType,
      model: launchForm.value.model,
    }

    if (launchForm.value.model_haiku) {
      body.model_haiku = launchForm.value.model_haiku
    }
    if (launchForm.value.model_sonnet) {
      body.model_sonnet = launchForm.value.model_sonnet
    }
    if (launchForm.value.model_opus) {
      body.model_opus = launchForm.value.model_opus
    }
    if (launchForm.value.work_dir.trim()) {
      body.work_dir = launchForm.value.work_dir.trim()
    }

    await api<void>('/api/apps/launch', {
      method: 'POST',
      body: JSON.stringify(body),
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

.app-grid > :deep(.n-card) {
  display: flex;
  flex-direction: column;
}

.app-grid :deep(.n-card-content) {
  display: flex;
  flex-direction: column;
  flex: 1;
}

.app-card-content {
  display: flex;
  flex-direction: column;
  flex: 1;
}

.app-card-action {
  margin-top: auto;
  padding-top: 8px;
}
</style>
