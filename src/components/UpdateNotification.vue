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

      <!-- 下载/安装进度 -->
      <template v-if="downloadState.downloading || downloadState.done">
        <n-progress
          :percentage="downloadState.percent"
          :status="downloadState.done ? 'success' : 'default'"
          :show-indicator="true"
          :height="18"
          :border-radius="4"
        />
        <n-text v-if="downloadState.downloading && !downloadState.done" depth="3" style="font-size: 12px">
          {{ formatBytes(downloadState.downloaded) }} / {{ formatBytes(downloadState.total) }}
        </n-text>
        <n-text v-if="downloadState.done" type="success" style="font-size: 12px">
          更新安装完成，即将重启...
        </n-text>
      </template>

      <!-- 错误提示 -->
      <n-text v-if="downloadState.error" type="error" style="font-size: 12px">
        {{ downloadState.error }}
      </n-text>
    </n-space>
    <template #footer>
      <n-space justify="end">
        <n-button v-if="!downloadState.downloading && !downloadState.done" @click="handleDismiss">
          稍后提醒
        </n-button>
        <n-button
          v-if="!downloadState.downloading && !downloadState.done"
          type="primary"
          @click="handleUpdate"
        >
          立即更新
        </n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted } from 'vue'
import { useMessage } from 'naive-ui'
import { listen } from '@tauri-apps/api/event'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { isTauri } from '../utils/env'

interface UpdateInfo {
  version: string
  release_notes: string
  published_at: string
}

const message = useMessage()
const visible = ref(false)
const updateInfo = ref<UpdateInfo>({
  version: '',
  release_notes: '',
  published_at: '',
})

const downloadState = reactive({
  downloading: false,
  done: false,
  percent: 0,
  downloaded: 0,
  total: 0,
  error: '',
})

// Store the update object from the plugin for later download+install
let pendingUpdate: Update | null = null

let unlistenUpToDate: (() => void) | null = null

onMounted(async () => {
  if (!isTauri) return

  unlistenUpToDate = await listen('up-to-date', () => {
    message.success('已是最新版本')
  })
})

onUnmounted(() => {
  unlistenUpToDate?.()
})

function show(info: UpdateInfo, update?: Update) {
  updateInfo.value = { ...info }
  pendingUpdate = update ?? null
  downloadState.downloading = false
  downloadState.done = false
  downloadState.percent = 0
  downloadState.downloaded = 0
  downloadState.total = 0
  downloadState.error = ''
  visible.value = true
}

function handleDismiss() {
  visible.value = false
}

async function handleUpdate() {
  downloadState.downloading = true
  downloadState.error = ''

  try {
    let update = pendingUpdate
    if (!update) {
      update = await check()
    }
    if (!update) {
      downloadState.downloading = false
      downloadState.error = '未发现可用更新'
      return
    }

    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          downloadState.total = event.data.contentLength ?? 0
          break
        case 'Progress':
          downloadState.downloaded += event.data.chunkLength
          if (downloadState.total > 0) {
            downloadState.percent = Math.min(
              Math.round((downloadState.downloaded / downloadState.total) * 100),
              100
            )
          }
          break
        case 'Finished':
          downloadState.percent = 100
          break
      }
    })

    downloadState.done = true
    downloadState.downloading = false

    await relaunch()
  } catch (e) {
    downloadState.downloading = false
    downloadState.error = String(e)
  }
}

function formatDate(dateStr: string): string {
  try {
    return new Date(dateStr).toLocaleDateString('zh-CN')
  } catch {
    return dateStr
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

defineExpose({ show })
</script>
