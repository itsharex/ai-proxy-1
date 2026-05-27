<template>
  <n-space vertical size="large">
    <n-card title="全局设置">
      <n-form label-placement="left" label-width="140" style="max-width: 520px">
        <n-form-item label="HTTP 主机">
          <n-input v-model:value="settings.host" placeholder="127.0.0.1" />
        </n-form-item>

        <n-alert
          v-if="settings.host === '0.0.0.0'"
          title="安全警告"
          type="warning"
          style="margin-bottom: 16px"
        >
          绑定到 0.0.0.0 将使代理对外部网络开放，请确保在受信任的网络环境中使用。
        </n-alert>

        <n-form-item label="HTTP 端口">
          <n-input-number
            v-model:value="settings.port"
            :min="1"
            :max="65535"
            style="width: 100%"
          />
        </n-form-item>
        <n-form-item label="请求超时（秒）">
          <n-input-number
            v-model:value="settings.requestTimeout"
            :min="10"
            :max="3600"
            style="width: 100%"
          />
        </n-form-item>
        <n-form-item label="连接超时（秒）">
          <n-input-number
            v-model:value="settings.connectTimeout"
            :min="1"
            :max="300"
            style="width: 100%"
          />
        </n-form-item>
        <n-form-item label="日志保留天数">
          <n-input-number
            v-model:value="settings.logRetentionDays"
            :min="1"
            :max="365"
            style="width: 100%"
          />
        </n-form-item>
        <n-form-item label="记录请求体">
          <n-switch v-model:value="settings.recordRequestBody" />
        </n-form-item>
        <n-form-item v-if="isTauri" label="开机启动">
          <n-switch v-model:value="autostartEnabled" @update:value="handleAutostartChange" />
        </n-form-item>

        <n-divider />

        <n-form-item label="代理入口认证">
          <n-switch v-model:value="settings.proxyAuthEnabled" />
        </n-form-item>
        <n-form-item v-if="settings.proxyAuthEnabled" label="代理 API Key">
          <n-input
            v-model:value="settings.proxyAuthKey"
            type="password"
            show-password-on="click"
            placeholder="设置 Agent 访问代理时使用的 API Key"
          />
        </n-form-item>
        <n-alert
          v-if="settings.proxyAuthEnabled && !settings.proxyAuthKey"
          title="请设置 API Key"
          type="warning"
          style="margin-bottom: 16px"
        >
          启用认证后，Agent 调用代理接口需在请求头携带 <n-text code>Authorization: Bearer &lt;your-key&gt;</n-text>。
        </n-alert>

        <n-form-item>
          <n-space>
            <n-button type="primary" @click="handleSave">
              保存设置
            </n-button>
          </n-space>
        </n-form-item>
      </n-form>
    </n-card>
    <n-card v-if="isTauri" title="检查更新">
      <n-form label-placement="left" label-width="140" style="max-width: 520px">
        <n-form-item label="当前版本">
          <n-text>{{ currentVersion }}</n-text>
        </n-form-item>
        <n-form-item>
          <n-button
            type="primary"
            :loading="checkingUpdate"
            @click="handleCheckUpdate"
          >
            检查更新
          </n-button>
        </n-form-item>
      </n-form>
    </n-card>
    <UpdateNotification ref="updateNotification" />
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useMessage } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart'
import { getVersion } from '@tauri-apps/api/app'
import { isTauri } from '../utils/env'
import { api, refreshApiConfig } from '../api'
import UpdateNotification from '../components/UpdateNotification.vue'

const message = useMessage()

interface AppSettings {
  host: string
  port: number
  requestTimeout: number
  connectTimeout: number
  logRetentionDays: number
  recordRequestBody: boolean
  proxyAuthEnabled: boolean
  proxyAuthKey: string
}

const settings = ref<AppSettings>({
  host: '127.0.0.1',
  port: 7860,
  requestTimeout: 1200,
  connectTimeout: 30,
  logRetentionDays: 30,
  recordRequestBody: false,
  proxyAuthEnabled: false,
  proxyAuthKey: '',
})

const savedNetworkConfig = ref({
  host: settings.value.host,
  port: settings.value.port,
})

const autostartEnabled = ref(false)
const currentVersion = ref('...')
const checkingUpdate = ref(false)
const updateNotification = ref<InstanceType<typeof UpdateNotification> | null>(null)

async function loadSettings() {
  try {
    const data = await api<{
      http_host: string
      http_port: string
      request_timeout: string
      connect_timeout: string
      log_retention_days: string
      record_request_body: string
      proxy_auth_enabled: string
      proxy_auth_key: string
    }>('/api/settings')
    settings.value = {
      host: data.http_host,
      port: parseInt(data.http_port) || 7860,
      requestTimeout: parseInt(data.request_timeout) || 1200,
      connectTimeout: parseInt(data.connect_timeout) || 30,
      logRetentionDays: parseInt(data.log_retention_days) || 30,
      recordRequestBody: data.record_request_body === 'true',
      proxyAuthEnabled: data.proxy_auth_enabled === 'true',
      proxyAuthKey: data.proxy_auth_key,
    }
    savedNetworkConfig.value = {
      host: settings.value.host,
      port: settings.value.port,
    }
  } catch (error) {
    console.error('Failed to load settings:', error)
  }
}

async function handleSave() {
  const previousHost = savedNetworkConfig.value.host
  const previousPort = savedNetworkConfig.value.port

  if (settings.value.proxyAuthEnabled && !settings.value.proxyAuthKey) {
    message.warning('启用认证后必须设置 API Key')
    return
  }
  try {
    await api('/api/settings', {
      method: 'PUT',
      body: JSON.stringify({
        http_host: settings.value.host,
        http_port: String(settings.value.port),
        request_timeout: String(settings.value.requestTimeout),
        connect_timeout: String(settings.value.connectTimeout),
        log_retention_days: String(settings.value.logRetentionDays),
        record_request_body: String(settings.value.recordRequestBody),
        proxy_auth_enabled: String(settings.value.proxyAuthEnabled),
        proxy_auth_key: settings.value.proxyAuthKey,
      }),
    })

    const hostChanged = settings.value.host !== previousHost
    const portChanged = settings.value.port !== previousPort

    if (isTauri && (hostChanged || portChanged)) {
      await invoke<string>('apply_proxy_config')
      await refreshApiConfig()
    }

    savedNetworkConfig.value = {
      host: settings.value.host,
      port: settings.value.port,
    }
    message.success('设置已保存')
  } catch (error) {
    message.error(`保存失败: ${error}`)
  }
}

async function handleAutostartChange(enabled: boolean) {
  try {
    if (enabled) {
      await enable()
    } else {
      await disable()
    }
    message.success(enabled ? '已启用开机启动' : '已关闭开机启动')
  } catch (error) {
    autostartEnabled.value = !enabled
    message.error(`设置失败: ${error}`)
  }
}

async function handleCheckUpdate() {
  checkingUpdate.value = true
  try {
    const result = await invoke<{
      version: string
      release_notes: string
      download_url: string
      published_at: string
    } | null>('check_for_update')
    if (result) {
      updateNotification.value?.show(result)
    } else {
      message.success('已是最新版本')
    }
  } catch (error) {
    message.error(`检查更新失败: ${error}`)
  } finally {
    checkingUpdate.value = false
  }
}

onMounted(async () => {
  await loadSettings()
  try {
    autostartEnabled.value = await isEnabled()
  } catch {
    autostartEnabled.value = false
  }
  if (isTauri) {
    try {
      currentVersion.value = await getVersion()
    } catch {
      currentVersion.value = 'unknown'
    }
  }
})
</script>
