<template>
  <div class="settings-view">
    <n-h3 style="margin: 0 0 16px 0">Settings</n-h3>

    <n-grid :x-gap="16" :y-gap="16" :cols="1" style="max-width: 640px">
      <n-gi>
        <n-card title="Server Configuration" size="small">
          <n-form label-placement="left" label-width="140">
            <n-form-item label="Host">
              <n-input v-model:value="settings.host" placeholder="127.0.0.1" />
            </n-form-item>
            <n-form-item label="Port">
              <n-input-number v-model:value="settings.port" :min="1" :max="65535" style="width: 100%" />
            </n-form-item>
            <n-alert v-if="settings.host === '0.0.0.0'" title="Network Exposure Warning" type="warning" style="margin-bottom: 16px">
              Binding to 0.0.0.0 exposes the proxy to all network interfaces. Ensure you have proper firewall rules and authentication enabled.
            </n-alert>
          </n-form>
        </n-card>
      </n-gi>
      <n-gi>
        <n-card title="Logging" size="small">
          <n-form label-placement="left" label-width="140">
            <n-form-item label="Log Retention">
              <n-slider v-model:value="settings.logRetentionDays" :min="1" :max="365" :step="1" :marks="{ 1: '1d', 30: '30d', 90: '90d', 365: '1y' }" />
            </n-form-item>
            <n-form-item label="Retention Label">
              <span style="color: #999">{{ settings.logRetentionDays }} days</span>
            </n-form-item>
          </n-form>
        </n-card>
      </n-gi>
      <n-gi>
        <n-card title="Authentication" size="small">
          <n-form label-placement="left" label-width="140">
            <n-form-item label="Proxy Auth">
              <n-switch v-model:value="settings.proxyAuth" />
            </n-form-item>
            <n-form-item v-if="settings.proxyAuth" label="Auth Token">
              <n-input v-model:value="settings.authToken" type="password" show-password-on="click" placeholder="Enter auth token" />
            </n-form-item>
          </n-form>
        </n-card>
      </n-gi>
    </n-grid>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NGrid, NGi, NCard, NForm, NFormItem, NInput, NInputNumber, NSwitch, NSlider, NAlert } from 'naive-ui'

const settings = ref({
  host: '127.0.0.1',
  port: 7860,
  logRetentionDays: 30,
  proxyAuth: false,
  authToken: '',
})
</script>
