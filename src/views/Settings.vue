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
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useMessage } from 'naive-ui'
import { api } from '../api'

const message = useMessage()

interface AppSettings {
  host: string
  port: number
  logRetentionDays: number
  recordRequestBody: boolean
  proxyAuthEnabled: boolean
  proxyAuthKey: string
}

const settings = ref<AppSettings>({
  host: '127.0.0.1',
  port: 7860,
  logRetentionDays: 30,
  recordRequestBody: false,
  proxyAuthEnabled: false,
  proxyAuthKey: '',
})

async function loadSettings() {
  try {
    const data = await api<{
      http_host: string
      http_port: string
      log_retention_days: string
      record_request_body: string
      proxy_auth_enabled: string
      proxy_auth_key: string
    }>('/api/settings')
    settings.value = {
      host: data.http_host,
      port: parseInt(data.http_port) || 7860,
      logRetentionDays: parseInt(data.log_retention_days) || 30,
      recordRequestBody: data.record_request_body === 'true',
      proxyAuthEnabled: data.proxy_auth_enabled === 'true',
      proxyAuthKey: data.proxy_auth_key,
    }
  } catch (error) {
    console.error('Failed to load settings:', error)
  }
}

async function handleSave() {
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
        log_retention_days: String(settings.value.logRetentionDays),
        record_request_body: String(settings.value.recordRequestBody),
        proxy_auth_enabled: String(settings.value.proxyAuthEnabled),
        proxy_auth_key: settings.value.proxyAuthKey,
      }),
    })
    message.success('设置已保存')
  } catch (error) {
    message.error(`保存失败: ${error}`)
  }
}

onMounted(() => {
  loadSettings()
})
</script>
