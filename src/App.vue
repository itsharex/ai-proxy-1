<template>
  <n-config-provider :theme="darkTheme">
    <n-message-provider>
      <n-layout has-sider style="height: 100vh">
        <n-layout-sider
          bordered
          collapse-mode="width"
          :collapsed-width="64"
          :width="220"
          :collapsed="collapsed"
          show-trigger
          @collapse="collapsed = true"
          @expand="collapsed = false"
        >
          <div class="sider-header">
            <span v-if="!collapsed" class="app-title">{{ t('app.title') }}</span>
            <span v-else class="app-title-short">AP</span>
          </div>
          <n-menu
            :collapsed="collapsed"
            :collapsed-width="64"
            :collapsed-icon-size="22"
            :options="menuOptions"
            :value="currentRoute"
            @update:value="handleMenuSelect"
          />
        </n-layout-sider>
        <n-layout>
          <n-layout-header bordered style="height: 48px; padding: 0 24px; display: flex; align-items: center; justify-content: space-between">
            <span style="font-size: 14px; font-weight: 500">{{ currentRouteName }}</span>
            <n-tag :type="serverRunning ? 'success' : 'error'" size="small" round>
              {{ serverRunning ? t('app.status.running') : t('app.status.stopped') }}
            </n-tag>
          </n-layout-header>
          <n-layout-content style="padding: 24px; overflow: auto" content-style="height: 100%">
            <router-view />
          </n-layout-content>
        </n-layout>
      </n-layout>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { ref, computed, h, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import {
  NConfigProvider,
  NLayout,
  NLayoutSider,
  NLayoutHeader,
  NLayoutContent,
  NMenu,
  NTag,
  NMessageProvider,
  darkTheme,
  type MenuOption,
} from 'naive-ui'
import {
  HomeOutline,
  ServerOutline,
  GitMergeOutline,
  DocumentTextOutline,
  StatsChartOutline,
  ShieldCheckmarkOutline,
  SettingsOutline,
} from '@vicons/ionicons5'
import { invoke } from '@tauri-apps/api/core'

const router = useRouter()
const route = useRoute()
const { t } = useI18n()
const collapsed = ref(false)
const serverRunning = ref(false)

const currentRoute = computed(() => route.path)
const currentRouteName = computed(() => {
  const name = String(route.name || 'Dashboard').toLowerCase()
  return t(`nav.${name}`)
})

function renderIcon(icon: unknown) {
  return () => h(icon as Parameters<typeof h>[0], { size: 20 })
}

const menuOptions = computed<MenuOption[]>(() => [
  { label: () => t('nav.dashboard'), key: '/', icon: renderIcon(HomeOutline) },
  { label: () => t('nav.providers'), key: '/providers', icon: renderIcon(ServerOutline) },
  { label: () => t('nav.models'), key: '/models', icon: renderIcon(GitMergeOutline) },
  { label: () => t('nav.rules'), key: '/rules', icon: renderIcon(ShieldCheckmarkOutline) },
  { label: () => t('nav.logs'), key: '/logs', icon: renderIcon(DocumentTextOutline) },
  { label: () => t('nav.statistics'), key: '/statistics', icon: renderIcon(StatsChartOutline) },
  { label: () => t('nav.settings'), key: '/settings', icon: renderIcon(SettingsOutline) },
])

function handleMenuSelect(key: string) {
  router.push(key)
}

async function checkServerStatus() {
  try {
    await invoke('get_providers')
    serverRunning.value = true
  } catch {
    serverRunning.value = false
  }
}

onMounted(() => {
  checkServerStatus()
  setInterval(checkServerStatus, 10000)
})
</script>

<style scoped>
.sider-header {
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-bottom: 1px solid var(--n-border-color);
}

.app-title {
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 0.5px;
}

.app-title-short {
  font-size: 16px;
  font-weight: 700;
}
</style>
