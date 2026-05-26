<template>
  <n-modal
    v-model:show="visible"
    preset="card"
    title="发现新版本"
    style="max-width: 500px"
    :mask-closable="false"
  >
    <n-space vertical size="medium">
      <n-tag type="success" size="medium">
        v{{ updateInfo.version }}
      </n-tag>
      <n-text depth="3" style="font-size: 13px">
        发布于 {{ formatDate(updateInfo.published_at) }}
      </n-text>
      <n-divider style="margin: 8px 0" />
      <n-scrollbar style="max-height: 250px">
        <pre style="white-space: pre-wrap; word-break: break-word; font-size: 13px; margin: 0">{{ updateInfo.release_notes || '暂无更新说明' }}</pre>
      </n-scrollbar>
    </n-space>
    <template #footer>
      <n-space justify="end">
        <n-button @click="handleDismiss">稍后提醒</n-button>
        <n-button type="primary" @click="handleDownload">立即下载</n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'

interface UpdateInfo {
  version: string
  release_notes: string
  download_url: string
  published_at: string
}

const visible = ref(false)
const updateInfo = ref<UpdateInfo>({
  version: '',
  release_notes: '',
  download_url: '',
  published_at: '',
})

function show(info: UpdateInfo) {
  updateInfo.value = { ...info }
  visible.value = true
}

function handleDismiss() {
  visible.value = false
}

async function handleDownload() {
  try {
    await openUrl(updateInfo.value.download_url)
  } catch (e) {
    console.error('Failed to open download URL:', e)
  }
  visible.value = false
}

function formatDate(dateStr: string): string {
  try {
    return new Date(dateStr).toLocaleDateString('zh-CN')
  } catch {
    return dateStr
  }
}

defineExpose({ show })
</script>
