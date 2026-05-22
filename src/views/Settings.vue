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
          <n-switch v-model:value="settings.logRequestBody" />
        </n-form-item>
        <n-form-item label="代理入口认证">
          <n-switch v-model:value="settings.requireAuth" />
        </n-form-item>
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

const message = useMessage()

const STORAGE_KEY = 'ai-proxy-settings'

interface AppSettings {
  host: string
  port: number
  logRetentionDays: number
  logRequestBody: boolean
  requireAuth: boolean
}

const settings = ref<AppSettings>({
  host: '127.0.0.1',
  port: 7860,
  logRetentionDays: 30,
  logRequestBody: false,
  requireAuth: false,
})

function loadSettings() {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored) {
      const parsed = JSON.parse(stored) as Partial<AppSettings>
      settings.value = {
        ...settings.value,
        ...parsed,
      }
    }
  } catch (error) {
    console.error('Failed to load settings from localStorage:', error)
  }
}

async function handleSave() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings.value))
    message.success('设置已保存')
  } catch (error) {
    message.error(`保存失败: ${error}`)
  }
}

onMounted(() => {
  loadSettings()
})
</script>
