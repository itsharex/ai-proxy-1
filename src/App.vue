<template>
  <n-config-provider :locale="zhCN" :date-locale="dateZhCN">
    <n-message-provider>
    <n-layout style="height: 100vh" has-sider>
      <n-layout-sider
        bordered
        collapse-mode="width"
        :collapsed-width="64"
        :width="200"
        :collapsed="collapsed"
        show-trigger
        @collapse="collapsed = true"
        @expand="collapsed = false"
      >
        <n-menu
          :collapsed="collapsed"
          :collapsed-width="64"
          :collapsed-icon-size="22"
          :options="menuOptions"
          :value="currentPath"
          @update:value="handleMenuSelect"
        />
      </n-layout-sider>
      <n-layout>
        <n-layout-header bordered style="padding: 12px 24px; display: flex; align-items: center; justify-content: flex-end;">
          <n-tag :type="serverRunning ? 'success' : 'error'" size="small">
            {{ serverRunning ? `Proxy :${proxyPort}` : 'Proxy stopped' }}
          </n-tag>
        </n-layout-header>
        <n-layout-content content-style="padding: 24px;">
          <router-view />
        </n-layout-content>
      </n-layout>
    </n-layout>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { ref, computed, h, onMounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { NIcon, zhCN, dateZhCN } from 'naive-ui'
import { initApi, getProxyPort, isInitialized } from './api'
import {
  HomeOutline,
  ServerOutline,
  GitBranchOutline,
  DocumentTextOutline,
  BarChartOutline,
  SettingsOutline,
} from '@vicons/ionicons5'

const router = useRouter()
const route = useRoute()
const collapsed = ref(false)
const serverRunning = ref(false)
const proxyPort = ref(7860)

onMounted(async () => {
  try {
    await initApi()
    serverRunning.value = true
    proxyPort.value = getProxyPort()
  } catch {
    serverRunning.value = false
  }
})

function renderIcon(icon: typeof HomeOutline) {
  return () => h(NIcon, null, () => h(icon))
}

const menuOptions = [
  { label: '仪表盘', key: '/', icon: renderIcon(HomeOutline) },
  { label: '供应商', key: '/providers', icon: renderIcon(ServerOutline) },
  { label: '模型总览', key: '/models', icon: renderIcon(GitBranchOutline) },
  { label: '请求日志', key: '/logs', icon: renderIcon(DocumentTextOutline) },
  { label: '用量统计', key: '/statistics', icon: renderIcon(BarChartOutline) },
  { label: '设置', key: '/settings', icon: renderIcon(SettingsOutline) },
]

const currentPath = computed(() => route.path)

watch(currentPath, () => {
  const running = isInitialized()
  serverRunning.value = running
  if (running) {
    proxyPort.value = getProxyPort()
  }
})

function handleMenuSelect(key: string) {
  router.push(key)
}
</script>
