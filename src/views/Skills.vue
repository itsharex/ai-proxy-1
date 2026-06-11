<template>
  <n-space vertical size="large">
    <n-card>
      <template #header>
        <n-space justify="space-between" align="center">
          <n-text strong>技能管理</n-text>
          <n-space>
            <n-button @click="handleScan" :loading="scanLoading">
              刷新扫描
            </n-button>
            <n-button
              v-if="brokenCount > 0"
              type="error"
              @click="handleCleanupBroken"
              :loading="cleanupLoading"
            >
              清理失效链接 ({{ brokenCount }})
            </n-button>
            <n-button @click="handleAutoDiscover" :loading="discoverLoading">
              自动发现
            </n-button>
            <n-button @click="openAddSourceModal">
              添加源
            </n-button>
            <n-dropdown :options="installOptions" @select="handleInstallSelect">
              <n-button>
                安装技能
              </n-button>
            </n-dropdown>
            <n-button type="primary" @click="openCreateSkillModal">
              创建技能
            </n-button>
          </n-space>
        </n-space>
      </template>

      <n-input
        v-model:value="searchQuery"
        placeholder="搜索技能名称或描述..."
        clearable
        style="margin-bottom: 12px"
      />

      <n-spin :show="loading">
        <n-tabs
          v-if="sources.length > 0"
          type="line"
          :value="activeTab"
          @update:value="handleTabChange"
        >
          <n-tab-pane
            v-for="source in visibleSources"
            :key="source.id"
            :name="source.id"
            :tab="`${source.name} (${getSkillsForSource(source.id).length})`"
          >
            <n-data-table
              :columns="columns"
              :data="getSkillsForSource(source.id)"
              :row-key="(row: Skill) => row.id"
              :row-class-name="(row: Skill) => toBool(row.is_broken_symlink) ? 'broken-symlink-row' : ''"
              :bordered="false"
              :scroll-x="900"
            />
          </n-tab-pane>
        </n-tabs>
        <n-empty v-else description="暂无技能源，请添加源或自动发现" />
      </n-spin>
    </n-card>

    <!-- Add Source Modal -->
    <n-modal
      v-model:show="showAddSourceModal"
      preset="dialog"
      title="添加技能源"
      positive-text="确认"
      negative-text="取消"
      :loading="addSourceLoading"
      @positive-click="handleAddSource"
      style="width: 480px"
    >
      <n-form :model="addSourceForm" label-placement="left" label-width="80">
        <n-form-item label="名称" required>
          <n-input v-model:value="addSourceForm.name" placeholder="例如: my-skills" :input-props="{ autocapitalize: 'off' }" />
        </n-form-item>
        <n-form-item label="路径" required>
          <n-input v-model:value="addSourceForm.path" placeholder="例如: ~/.claude/skills" :input-props="{ autocapitalize: 'off' }" />
        </n-form-item>
      </n-form>
    </n-modal>

    <!-- Create Skill Modal -->
    <n-modal
      v-model:show="showCreateSkillModal"
      preset="dialog"
      title="创建技能"
      positive-text="确认"
      negative-text="取消"
      :loading="createSkillLoading"
      @positive-click="handleCreateSkill"
      style="width: 560px"
    >
      <n-form :model="createSkillForm" label-placement="left" label-width="100">
        <n-form-item label="目标源" required>
          <n-select
            v-model:value="createSkillForm.sourceId"
            :options="sourceSelectOptions"
            placeholder="选择目标源"
          />
        </n-form-item>
        <n-form-item label="技能名称" required>
          <n-input v-model:value="createSkillForm.name" placeholder="例如: my-skill" :input-props="{ autocapitalize: 'off' }" />
        </n-form-item>
        <n-form-item label="描述">
          <n-input v-model:value="createSkillForm.description" placeholder="技能描述" />
        </n-form-item>
        <n-form-item label="SKILL.md">
          <n-input
            v-model:value="createSkillForm.skill_md_content"
            type="textarea"
            placeholder="输入 SKILL.md 内容"
            :rows="6"
          />
        </n-form-item>
      </n-form>
    </n-modal>

    <!-- Install from URL Modal -->
    <n-modal
      v-model:show="showUrlInstallModal"
      preset="dialog"
      title="从 URL 安装技能"
      positive-text="安装"
      negative-text="取消"
      :loading="urlInstallLoading"
      @positive-click="handleUrlInstall"
      style="width: 480px"
    >
      <n-form :model="urlInstallForm" label-placement="left" label-width="80">
        <n-form-item label="目标源" required>
          <n-select
            v-model:value="urlInstallForm.sourceId"
            :options="writableSourceSelectOptions"
            placeholder="选择目标源"
          />
        </n-form-item>
        <n-form-item label="URL" required>
          <n-input v-model:value="urlInstallForm.url" placeholder="技能 URL 地址" :input-props="{ autocapitalize: 'off' }" />
        </n-form-item>
      </n-form>
    </n-modal>

    <!-- Install to Target Sources Modal -->
    <n-modal
      v-model:show="showInstallTargetModal"
      preset="dialog"
      :title="`安装技能 - ${installTargetSkill?.name || ''}`"
      positive-text="安装"
      negative-text="取消"
      :loading="installTargetLoading"
      @positive-click="handleInstallToTargets"
      style="width: 480px"
    >
      <n-space vertical>
        <n-text>选择要安装到的目标源：</n-text>
        <n-checkbox-group v-model:value="installTargetSourceIds">
          <n-space vertical>
            <n-checkbox
              v-for="source in installableSources"
              :key="source.id"
              :value="source.id"
              :label="source.name"
            />
          </n-space>
        </n-checkbox-group>
      </n-space>
    </n-modal>

    <!-- Edit SKILL.md Modal -->
    <n-modal
      v-model:show="showEditMdModal"
      preset="card"
      :title="`编辑 SKILL.md - ${editMdSkill?.name || ''}`"
      :style="editMdFullscreen ? 'width: 95vw' : 'width: 680px'"
      :bordered="false"
      :mask-closable="false"
    >
      <template #header-extra>
        <n-button quaternary size="small" @click="editMdFullscreen = !editMdFullscreen">
          <template #icon>
            <n-icon><contract-outline v-if="editMdFullscreen" /><expand-outline v-else /></n-icon>
          </template>
        </n-button>
      </template>
      <n-spin :show="editMdLoading">
        <div :style="{ position: 'relative', height: editMdFullscreen ? 'calc(90vh - 180px)' : '480px' }">
          <Codemirror
            v-model="editMdContent"
            :style="{ height: editMdFullscreen ? 'calc(90vh - 180px)' : '480px' }"
            :extensions="mdEditorExtensions"
          />
        </div>
      </n-spin>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showEditMdModal = false">取消</n-button>
          <n-button type="primary" :loading="editMdLoading" @click="handleSaveMd">保存</n-button>
        </n-space>
      </template>
    </n-modal>

    <!-- Marketplace Search Modal -->
    <n-modal
      v-model:show="showMarketplaceModal"
      preset="card"
      title="从 skills.sh 安装"
      style="width: 680px"
      :bordered="false"
      :segmented="{ content: true }"
    >
      <n-space vertical size="medium">
        <n-space>
          <n-input
            v-model:value="marketplaceQuery"
            placeholder="搜索技能，例如 frontend"
            clearable
            style="width: 440px"
            :input-props="{ autocapitalize: 'off' }"
            @keyup.enter="handleMarketplaceSearch"
          />
          <n-button type="primary" :loading="marketplaceLoading" @click="handleMarketplaceSearch">
            搜索
          </n-button>
        </n-space>
        <n-spin :show="marketplaceLoading">
          <div v-if="marketplaceResults.length > 0" style="max-height: 400px; overflow-y: auto;">
            <n-card
              v-for="skill in marketplaceResults"
              :key="skill.id"
              size="small"
              hoverable
              style="margin-bottom: 8px"
            >
              <n-space justify="space-between" align="center">
                <n-space vertical size="small">
                  <n-text strong>{{ skill.name }}</n-text>
                  <n-space size="small">
                    <n-tag size="small" type="info">{{ skill.source }}</n-tag>
                    <n-text depth="3">{{ formatInstalls(skill.installs) }} 次安装</n-text>
                  </n-space>
                </n-space>
                <n-button
                  size="small"
                  type="primary"
                  :loading="marketplaceInstalling === skill.skillId"
                  @click="handleMarketplaceInstall(skill)"
                >
                  安装
                </n-button>
              </n-space>
            </n-card>
          </div>
          <n-empty v-else-if="marketplaceSearched" description="未找到相关技能" />
        </n-spin>
      </n-space>
    </n-modal>

    <!-- Diff / Conflict Modal -->
    <n-modal
      v-model:show="showDiffModal"
      preset="card"
      title="技能内容对比"
      :style="diffFullscreen ? 'width: 98vw; height: 96vh' : 'width: 90vw; max-width: 1200px'"
      :bordered="false"
      :mask-closable="false"
    >
      <template #header-extra>
        <n-space align="center">
          <n-button quaternary size="small" @click="diffFullscreen = !diffFullscreen">
            <template #icon>
              <n-icon><contract-outline v-if="diffFullscreen" /><expand-outline v-else /></n-icon>
            </template>
          </n-button>
          <n-button @click="destroyDiffEditors(); diffFullscreen = false; showDiffModal = false">取消</n-button>
          <n-button type="error" :loading="copyForceLoading" @click="handleForceCopy">
            覆盖并复制
          </n-button>
        </n-space>
      </template>
      <n-spin :show="diffLoading">
        <div :style="{ display: 'flex', gap: '16px' }">
          <div style="flex: 1; min-width: 0;">
            <n-text strong>源技能 ({{ diffSourceName }})</n-text>
            <div ref="diffSourceCmRef" class="diff-cm-container" :style="diffFullscreen ? { height: 'calc(96vh - 180px)' } : undefined"></div>
          </div>
          <div style="flex: 1; min-width: 0;">
            <n-text strong>全局源同名技能 ({{ diffTargetName }})</n-text>
            <div ref="diffTargetCmRef" class="diff-cm-container" :style="diffFullscreen ? { height: 'calc(96vh - 180px)' } : undefined"></div>
          </div>
        </div>
      </n-spin>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, computed, h, onMounted, nextTick } from 'vue'
import {
  NTag,
  NPopconfirm,
  NButton,
  NSpace,
  NTooltip,
  NIcon,
  useMessage,
  useDialog,
} from 'naive-ui'
import { api } from '../api'
import { ApiError } from '../api'
import { Codemirror } from 'vue-codemirror'
import { basicSetup } from 'codemirror'
import { markdown } from '@codemirror/lang-markdown'
import { oneDark } from '@codemirror/theme-one-dark'
import { EditorView } from '@codemirror/view'
import { EditorState, type Extension } from '@codemirror/state'
import { ExpandOutline, ContractOutline } from '@vicons/ionicons5'
import type {
  SkillSourceWithCount,
  Skill,
  SkillDetail,
  CreateSkillSourceBody,
  CreateSkillBody,
  InstallFromUrlBody,
} from '../types'

const message = useMessage()
const dialog = useDialog()
const mdEditorExtensions: Extension[] = [...(basicSetup as Extension[]), markdown(), oneDark]
const loading = ref(false)
const scanLoading = ref(false)
const discoverLoading = ref(false)
const cleanupLoading = ref(false)
const sources = ref<SkillSourceWithCount[]>([])
const skills = ref<Skill[]>([])
const activeTab = ref<string>('')
const searchQuery = ref('')

// Add Source Modal
const showAddSourceModal = ref(false)
const addSourceLoading = ref(false)
const addSourceForm = ref({ name: '', path: '' })

// Create Skill Modal
const showCreateSkillModal = ref(false)
const createSkillLoading = ref(false)
const createSkillForm = ref({
  sourceId: '',
  name: '',
  description: '',
  skill_md_content: '',
})

// URL Install Modal
const showUrlInstallModal = ref(false)
const urlInstallLoading = ref(false)
const urlInstallForm = ref({ sourceId: '', url: '' })

// Install Target Modal
const showInstallTargetModal = ref(false)
const installTargetLoading = ref(false)
const installTargetSkill = ref<Skill | null>(null)
const installTargetSourceIds = ref<string[]>([])

// Edit MD Modal
const showEditMdModal = ref(false)
const editMdLoading = ref(false)
const editMdSkill = ref<Skill | null>(null)
const editMdContent = ref('')
const editMdFullscreen = ref(false)

// Dropdown options for install button
const installOptions = [
  { label: '从全局库安装', key: 'global' },
  { label: '从 URL 安装', key: 'url' },
  { label: '从 skills.sh 安装', key: 'marketplace' },
]

// Marketplace Search Modal
const showMarketplaceModal = ref(false)
const marketplaceQuery = ref('')
const marketplaceLoading = ref(false)
const marketplaceSearched = ref(false)
const marketplaceResults = ref<MarketplaceSkill[]>([])
const marketplaceInstalling = ref<string | null>(null)

// Diff / Conflict Modal
const showDiffModal = ref(false)
const diffLoading = ref(false)
const copyForceLoading = ref(false)
const diffFullscreen = ref(false)
const diffSourceContent = ref('')
const diffTargetContent = ref('')
const diffSourceName = ref('')
const diffTargetName = ref('')
const conflictSourceSkillId = ref<string | null>(null)
const conflictExistingSkillId = ref<string | null>(null)

// Diff CodeMirror refs
const diffSourceCmRef = ref<HTMLElement | null>(null)
const diffTargetCmRef = ref<HTMLElement | null>(null)
let diffSourceView: EditorView | null = null
let diffTargetView: EditorView | null = null
let syncScrollEnabled = true

function createDiffEditor(container: HTMLElement | null, content: string): EditorView | null {
  if (!container) return null
  const state = EditorState.create({
    doc: content,
    extensions: [
      ...(basicSetup as Extension[]),
      markdown(),
      oneDark,
      EditorView.lineWrapping,
      EditorState.readOnly.of(true),
    ],
  })
  const view = new EditorView({
    state,
    parent: container,
  })
  return view
}

function destroyDiffEditors() {
  diffSourceView?.destroy()
  diffTargetView?.destroy()
  diffSourceView = null
  diffTargetView = null
}

function setupSyncScroll() {
  if (!diffSourceView || !diffTargetView) return

  const sourceScroller = diffSourceView.scrollDOM
  const targetScroller = diffTargetView.scrollDOM

  const syncFromSource = () => {
    if (!syncScrollEnabled) return
    syncScrollEnabled = false
    targetScroller.scrollTop = sourceScroller.scrollTop
    targetScroller.scrollLeft = sourceScroller.scrollLeft
    syncScrollEnabled = true
  }

  const syncFromTarget = () => {
    if (!syncScrollEnabled) return
    syncScrollEnabled = false
    sourceScroller.scrollTop = targetScroller.scrollTop
    sourceScroller.scrollLeft = targetScroller.scrollLeft
    syncScrollEnabled = true
  }

  sourceScroller.addEventListener('scroll', syncFromSource)
  targetScroller.addEventListener('scroll', syncFromTarget)
}

interface MarketplaceSkill {
  id: string
  skillId: string
  name: string
  installs: number
  source: string
}

// Computed source select options
const sourceSelectOptions = computed(() =>
  sources.value.map((s) => ({ label: s.name, value: s.id }))
)

const writableSourceSelectOptions = computed(() =>
  sources.value.filter((s) => !s.is_global).map((s) => ({ label: s.name, value: s.id }))
)

// Sources available for install targets (exclude global library)
const installableSources = computed(() =>
  sources.value.filter((s) => !s.is_global)
)

// Get global source
const globalSource = computed(() =>
  sources.value.find((s) => s.is_global)
)

// Count broken symlinks across all sources
const brokenCount = computed(() =>
  skills.value.filter((s) => toBool(s.is_broken_symlink)).length
)

// Filter sources: hide non-global sources with 0 skills
const visibleSources = computed(() =>
  sources.value.filter((s) => toBool(s.is_global) || getSkillsForSource(s.id).length > 0)
)

// Get skills for a given source
function getSkillsForSource(sourceId: string): Skill[] {
  let list = skills.value.filter((s) => s.source_id === sourceId)
  const q = searchQuery.value.trim().toLowerCase()
  if (q) {
    list = list.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        (s.description || '').toLowerCase().includes(q)
    )
  }
  // Sort broken symlinks to top
  return [...list].sort((a, b) => {
    const aBroken = toBool(a.is_broken_symlink) ? 1 : 0
    const bBroken = toBool(b.is_broken_symlink) ? 1 : 0
    return bBroken - aBroken
  })
}

// Helper to normalize boolean from SQLite (0/1 -> boolean)
function toBool(val: unknown): boolean {
  if (typeof val === 'boolean') return val
  if (typeof val === 'number') return val !== 0
  if (typeof val === 'string') return val === '1' || val.toLowerCase() === 'true'
  return false
}

// Table columns
const columns = [
  {
    title: '名称',
    key: 'name',
    width: 160,
    render: (row: Skill) =>
      h(NTooltip, {}, {
        trigger: () => h('span', {}, row.name),
        default: () => row.description || row.name,
      }),
  },
  {
    title: '描述',
    key: 'description',
    ellipsis: { tooltip: true },
    render: (row: Skill) => {
      const desc = row.description || ''
      if (desc.length <= 60) return desc || '-'
      return desc.substring(0, 60) + '...'
    },
  },
  {
    title: '类型',
    key: 'type',
    width: 100,
    render: (row: Skill) => {
      const source = sources.value.find((s) => s.id === row.source_id)
      if (toBool(row.is_broken_symlink)) {
        return h(NTag, { size: 'small', type: 'error' as never }, () => '失效')
      }
      if (source && toBool(source.is_global)) {
        return h(NTag, { size: 'small', type: 'info' as never }, () => '全局')
      }
      if (toBool(row.is_symlink)) {
        return h(NTag, { size: 'small', type: 'warning' as never }, () => '链接')
      }
      return h(NTag, { size: 'small', type: 'success' as never }, () => '本地')
    },
  },
  {
    title: '链接目标',
    key: 'link_target',
    width: 200,
    ellipsis: { tooltip: true },
    render: (row: Skill) => {
      if (!toBool(row.is_symlink) || !row.link_target) return '-'
      return row.link_target
    },
  },
  {
    title: '操作',
    key: 'actions',
    width: 280,
    align: 'center',
    fixed: 'right' as const,
    render: (row: Skill) => {
      const source = sources.value.find((s) => s.id === row.source_id)
      const isGlobal = source ? toBool(source.is_global) : false
      const isSymlink = toBool(row.is_symlink)
      const isBroken = toBool(row.is_broken_symlink)

      const buttons: ReturnType<typeof h>[] = []

      // Install to... (only for global skills)
      if (isGlobal) {
        buttons.push(
          h(
            NButton,
            { size: 'small', quaternary: true, type: 'primary', onClick: () => openInstallTargetModal(row) },
            () => '安装到...'
          )
        )
      }

      // Copy to global (non-global, non-symlink, non-broken)
      if (!isGlobal && !isSymlink && !isBroken) {
        buttons.push(
          h(
            NButton,
            { size: 'small', quaternary: true, type: 'success', onClick: () => handleCopyToGlobal(row) },
            () => '复制到全局'
          )
        )
      }

      // Edit SKILL.md
      if (toBool(row.has_skill_md)) {
        buttons.push(
          h(
            NButton,
            { size: 'small', quaternary: true, type: 'info', onClick: () => openEditMdModal(row) },
            () => '编辑 MD'
          )
        )
      }

      // Cleanup broken symlink
      if (toBool(row.is_broken_symlink)) {
        buttons.push(
          h(
            NPopconfirm,
            { onPositiveClick: () => handleCleanupSingle(row.id) },
            {
              trigger: () =>
                h(NButton, { size: 'small', quaternary: true, type: 'error' }, () => '清理'),
              default: () => '确认清理此失效链接？',
            }
          )
        )
      }

      // Uninstall (symlink only) or Delete
      if (isSymlink && !isBroken) {
        buttons.push(
          h(
            NPopconfirm,
            { onPositiveClick: () => handleUninstall(row.id) },
            {
              trigger: () =>
                h(NButton, { size: 'small', quaternary: true, type: 'warning' }, () => '卸载'),
              default: () => '确认卸载此技能？仅删除符号链接。',
            }
          )
        )
      } else {
        buttons.push(
          h(
            NButton,
            { size: 'small', quaternary: true, type: 'error', onClick: () => handleDeleteWithCheck(row) },
            () => '删除'
          )
        )
      }

      return h(NSpace, { size: 4, justify: 'center' }, () => buttons)
    },
  },
]

// Tab change handler
function handleTabChange(value: string) {
  activeTab.value = value
}

// ---- Data Fetching ----

async function handleScan() {
  scanLoading.value = true
  try {
    await api('/api/skills/scan', { method: 'POST' })
    await fetchData()
    message.success('扫描完成')
  } catch (err) {
    message.error(`扫描失败: ${err}`)
  } finally {
    scanLoading.value = false
  }
}

async function handleAutoDiscover() {
  discoverLoading.value = true
  try {
    await api('/api/skills/discover', { method: 'POST' })
    await fetchData()
    message.success('自动发现完成')
  } catch (err) {
    message.error(`自动发现失败: ${err}`)
  } finally {
    discoverLoading.value = false
  }
}

async function fetchData() {
  loading.value = true
  try {
    const [sourceList, skillList] = await Promise.all([
      api<SkillSourceWithCount[]>('/api/skills/sources'),
      api<Skill[]>('/api/skills'),
    ])
    sources.value = sourceList
    skills.value = skillList

    // Set active tab to first source if not set or current tab no longer valid
    if (sources.value.length > 0) {
      const validIds = sources.value.map((s) => s.id)
      if (!activeTab.value || !validIds.includes(activeTab.value)) {
        activeTab.value = sources.value[0].id
      }
    } else {
      activeTab.value = ''
    }
  } catch (err) {
    message.error(`加载数据失败: ${err}`)
  } finally {
    loading.value = false
  }
}

// ---- Add Source ----

function openAddSourceModal() {
  addSourceForm.value = { name: '', path: '' }
  showAddSourceModal.value = true
}

async function handleAddSource() {
  if (!addSourceForm.value.name.trim()) {
    message.warning('请填写名称')
    return false
  }
  if (!addSourceForm.value.path.trim()) {
    message.warning('请填写路径')
    return false
  }

  addSourceLoading.value = true
  try {
    const body: CreateSkillSourceBody = {
      name: addSourceForm.value.name.trim(),
      path: addSourceForm.value.path.trim(),
    }
    await api('/api/skills/sources', {
      method: 'POST',
      body: JSON.stringify(body),
    })
    message.success('技能源添加成功')
    showAddSourceModal.value = false
    await fetchData()
  } catch (err) {
    message.error(`添加失败: ${err}`)
  } finally {
    addSourceLoading.value = false
  }
  return false
}

// ---- Create Skill ----

function openCreateSkillModal() {
  const writableSources = sources.value.filter((s) => !toBool(s.is_global))
  createSkillForm.value = {
    sourceId: writableSources.length > 0 ? writableSources[0].id : '',
    name: '',
    description: '',
    skill_md_content: '',
  }
  showCreateSkillModal.value = true
}

async function handleCreateSkill() {
  if (!createSkillForm.value.sourceId) {
    message.warning('请选择目标源')
    return false
  }
  if (!createSkillForm.value.name.trim()) {
    message.warning('请填写技能名称')
    return false
  }

  createSkillLoading.value = true
  try {
    const body: CreateSkillBody & { source_id: string } = {
      source_id: createSkillForm.value.sourceId,
      name: createSkillForm.value.name.trim(),
      description: createSkillForm.value.description.trim() || undefined,
      skill_md_content: createSkillForm.value.skill_md_content.trim() || undefined,
    }
    await api('/api/skills', {
      method: 'POST',
      body: JSON.stringify(body),
    })
    message.success('技能创建成功')
    showCreateSkillModal.value = false
    await fetchData()
  } catch (err) {
    message.error(`创建失败: ${err}`)
  } finally {
    createSkillLoading.value = false
  }
  return false
}

// ---- Install from URL ----

function openUrlInstallModal() {
  const writable = sources.value.filter((s) => !toBool(s.is_global))
  urlInstallForm.value = {
    sourceId: writable.length > 0 ? writable[0].id : '',
    url: '',
  }
  showUrlInstallModal.value = true
}

async function handleUrlInstall() {
  if (!urlInstallForm.value.sourceId) {
    message.warning('请选择目标源')
    return false
  }
  if (!urlInstallForm.value.url.trim()) {
    message.warning('请填写 URL')
    return false
  }

  urlInstallLoading.value = true
  try {
    const body: InstallFromUrlBody & { source_id: string } = {
      source_id: urlInstallForm.value.sourceId,
      url: urlInstallForm.value.url.trim(),
    }
    await api('/api/skills/install-from-url', {
      method: 'POST',
      body: JSON.stringify(body),
    })
    message.success('技能安装成功')
    showUrlInstallModal.value = false
    await fetchData()
  } catch (err) {
    message.error(`安装失败: ${err}`)
  } finally {
    urlInstallLoading.value = false
  }
  return false
}

// ---- Install to Target Sources ----

function openInstallTargetModal(skill: Skill) {
  installTargetSkill.value = skill
  // Pre-select non-global sources
  installTargetSourceIds.value = installableSources.value.map((s) => s.id)
  showInstallTargetModal.value = true
}

async function handleInstallToTargets() {
  if (!installTargetSkill.value) return false
  if (installTargetSourceIds.value.length === 0) {
    message.warning('请至少选择一个目标源')
    return false
  }

  installTargetLoading.value = true
  try {
    await api(`/api/skills/${installTargetSkill.value.id}/install`, {
      method: 'POST',
      body: JSON.stringify({
        skill_id: installTargetSkill.value.id,
        target_source_ids: installTargetSourceIds.value,
      }),
    })
    message.success('技能安装成功')
    showInstallTargetModal.value = false
    await fetchData()
  } catch (err) {
    message.error(`安装失败: ${err}`)
  } finally {
    installTargetLoading.value = false
  }
  return false
}

// ---- Uninstall Skill (remove symlink) ----

async function handleUninstall(skillId: string) {
  try {
    await api(`/api/skills/${skillId}/uninstall`, {
      method: 'POST',
      body: JSON.stringify({ skill_id: skillId }),
    })
    message.success('技能已卸载')
    await fetchData()
  } catch (err) {
    message.error(`卸载失败: ${err}`)
  }
}

// ---- Delete Skill ----

async function handleDeleteWithCheck(skill: Skill) {
  try {
    // Query linked skills in other sources
    const linked = await api<Skill[]>(`/api/skills/${skill.id}/linked`)

    if (linked && linked.length > 0) {
      // Build warning message with linked sources
      const linkedNames = linked.map(l => {
        const src = sources.value.find(s => s.id === l.source_id)
        return src ? `${src.name}` : l.skill_path
      }).join('、')

      dialog.warning({
        title: '删除技能',
        content: `该技能已被链接到以下目录：${linkedNames}。删除后将同步移除这些符号链接。确认删除？`,
        positiveText: '确认删除',
        negativeText: '取消',
        onPositiveClick: async () => {
          await doDelete(skill.id)
        },
      })
    } else {
      dialog.warning({
        title: '删除技能',
        content: `确认删除技能「${skill.name}」？此操作不可恢复。`,
        positiveText: '确认删除',
        negativeText: '取消',
        onPositiveClick: async () => {
          await doDelete(skill.id)
        },
      })
    }
  } catch {
    // If linked query fails, fallback to simple confirm
    dialog.warning({
      title: '删除技能',
      content: `确认删除技能「${skill.name}」？此操作不可恢复。`,
      positiveText: '确认删除',
      negativeText: '取消',
      onPositiveClick: async () => {
        await doDelete(skill.id)
      },
    })
  }
}

async function doDelete(skillId: string) {
  try {
    const result = await api<string[]>(`/api/skills/${skillId}`, { method: 'DELETE' })
    if (result && result.length > 0) {
      message.success(`技能已删除，同步清理了 ${result.length} 个关联链接`)
    } else {
      message.success('技能已删除')
    }
    await fetchData()
  } catch (err) {
    message.error(`删除失败: ${err}`)
  }
}

// ---- Edit SKILL.md ----

async function openEditMdModal(skill: Skill) {
  editMdSkill.value = skill
  editMdContent.value = ''
  showEditMdModal.value = true

  try {
    const detail = await api<SkillDetail>(`/api/skills/${skill.id}`)
    editMdContent.value = detail.skill_md_content || ''
  } catch (err) {
    message.error(`加载 SKILL.md 失败: ${err}`)
  }
}

async function handleSaveMd() {
  if (!editMdSkill.value) return false

  editMdLoading.value = true
  try {
    await api(`/api/skills/${editMdSkill.value.id}/skill-md`, {
      method: 'PUT',
      body: JSON.stringify({ content: editMdContent.value }),
    })
    message.success('SKILL.md 已保存')
    showEditMdModal.value = false
    await fetchData()
  } catch (err) {
    message.error(`保存失败: ${err}`)
  } finally {
    editMdLoading.value = false
  }
  return false
}

// ---- Install dropdown handler ----

function handleInstallSelect(key: string) {
  if (key === 'global') {
    // Open install target modal with global skills list
    if (!globalSource.value) {
      message.warning('未找到全局技能库')
      return
    }
    // If there are global skills, let user pick one
    const globalSkills = getSkillsForSource(globalSource.value.id)
    if (globalSkills.length === 0) {
      message.warning('全局技能库中没有技能')
      return
    }
    // Open install target modal for first global skill as example
    // User can select which skills to install from the global source tab
    openInstallTargetModal(globalSkills[0])
  } else if (key === 'url') {
    openUrlInstallModal()
  } else if (key === 'marketplace') {
    marketplaceQuery.value = ''
    marketplaceResults.value = []
    marketplaceSearched.value = false
    showMarketplaceModal.value = true
  }
}

// ---- Marketplace Search & Install ----

async function handleMarketplaceSearch() {
  const q = marketplaceQuery.value.trim()
  if (!q) {
    message.warning('请输入搜索关键词')
    return
  }

  marketplaceLoading.value = true
  marketplaceSearched.value = false
  try {
    const resp = await api<{ skills: MarketplaceSkill[] }>(
      `/api/skills-marketplace/search?q=${encodeURIComponent(q)}&limit=20`
    )
    marketplaceResults.value = resp.skills || []
    marketplaceSearched.value = true
  } catch (err) {
    message.error(`搜索失败: ${err}`)
  } finally {
    marketplaceLoading.value = false
  }
}

async function handleMarketplaceInstall(skill: MarketplaceSkill) {
  marketplaceInstalling.value = skill.skillId
  try {
    await api('/api/skills/install-from-marketplace', {
      method: 'POST',
      body: JSON.stringify({
        source: skill.source,
        skill_name: skill.skillId,
      }),
    })
    message.success(`技能「${skill.name}」安装成功`)
    showMarketplaceModal.value = false
    await fetchData()
  } catch (err) {
    message.error(`安装失败: ${err}`)
  } finally {
    marketplaceInstalling.value = null
  }
}

function formatInstalls(n: number): string {
  if (n >= 10000) return (n / 10000).toFixed(1) + '万'
  if (n >= 1000) return (n / 1000).toFixed(1) + 'k'
  return String(n)
}

// ---- Cleanup Broken Symlinks ----

async function handleCleanupBroken() {
  cleanupLoading.value = true
  try {
    const result = await api<string[]>('/api/skills/cleanup-broken', { method: 'POST' })
    if (result && result.length > 0) {
      message.success(`已清理 ${result.length} 个失效链接`)
    } else {
      message.info('没有失效链接需要清理')
    }
    await fetchData()
  } catch (err) {
    message.error(`清理失败: ${err}`)
  } finally {
    cleanupLoading.value = false
  }
}

async function handleCleanupSingle(skillId: string) {
  try {
    await api(`/api/skills/${skillId}/cleanup-broken`, { method: 'POST' })
    message.success('已清理')
    await fetchData()
  } catch (err) {
    message.error(`清理失败: ${err}`)
  }
}

// ---- Copy to Global Source ----

async function handleCopyToGlobal(skill: Skill) {
  try {
    await api(`/api/skills/${skill.id}/copy-to-global`, { method: 'POST' })
    message.success(`技能「${skill.name}」已复制到全局源`)
    await fetchData()
  } catch (err: unknown) {
    if (err instanceof ApiError && err.status === 409) {
      const data = err.data as { existing_skill_id?: string; existing_skill_name?: string } | undefined
      const existingId = data?.existing_skill_id || ''
      const existingName = data?.existing_skill_name || skill.name
      openDiffModal(skill, existingId, existingName)
    } else {
      message.error(`复制失败: ${err instanceof Error ? err.message : err}`)
    }
  }
}

async function openDiffModal(skill: Skill, existingId: string, existingName: string) {
  conflictSourceSkillId.value = skill.id
  conflictExistingSkillId.value = existingId
  diffSourceName.value = skill.name
  diffTargetName.value = existingName || skill.name
  diffSourceContent.value = ''
  diffTargetContent.value = ''
  diffLoading.value = true
  showDiffModal.value = true

  try {
    // Load source skill SKILL.md
    const sourceDetail = await api<SkillDetail>(`/api/skills/${skill.id}`)
    diffSourceContent.value = sourceDetail.skill_md_content || '(无 SKILL.md)'

    // Load existing global skill SKILL.md
    if (existingId) {
      const targetDetail = await api<SkillDetail>(`/api/skills/${existingId}`)
      diffTargetContent.value = targetDetail.skill_md_content || '(无 SKILL.md)'
    } else {
      diffTargetContent.value = '(无法加载)'
    }
  } catch {
    diffTargetContent.value = '(加载失败)'
  } finally {
    diffLoading.value = false
    // Wait for DOM to render, then create editors
    await nextTick()
    destroyDiffEditors()
    diffSourceView = createDiffEditor(diffSourceCmRef.value, diffSourceContent.value)
    diffTargetView = createDiffEditor(diffTargetCmRef.value, diffTargetContent.value)
    setupSyncScroll()
  }
}

async function handleForceCopy() {
  if (!conflictSourceSkillId.value) return
  copyForceLoading.value = true
  try {
    await api(`/api/skills/${conflictSourceSkillId.value}/copy-to-global?force=true`, { method: 'POST' })
    message.success('已覆盖并复制到全局源')
    destroyDiffEditors()
    showDiffModal.value = false
    await fetchData()
  } catch (err) {
    message.error(`覆盖复制失败: ${err}`)
  } finally {
    copyForceLoading.value = false
  }
}

// ---- Lifecycle ----

onMounted(async () => {
  // Auto scan on page load, then fetch data
  scanLoading.value = true
  try {
    await api('/api/skills/scan', { method: 'POST' })
  } catch {
    // Ignore scan errors on initial load
  } finally {
    scanLoading.value = false
  }
  await fetchData()
})
</script>

<style scoped>
.broken-symlink-row td {
  color: var(--error) !important;
}
.broken-symlink-row td .n-tag {
  color: var(--error) !important;
}
</style>

<style>
.diff-cm-container {
  height: 60vh;
  overflow: hidden;
  margin-top: 4px;
  border: 1px solid var(--border, #3a3a3a);
  border-radius: 4px;
}
.diff-cm-container .cm-editor {
  height: 100%;
}
.diff-cm-container .cm-scroller {
  overflow: auto !important;
}
.n-spin-container .n-spin-content {
  height: 100%;
}
.n-spin-container .n-spin-content > div {
  height: 100%;
}
.n-spin-container codemirror {
  display: block;
  height: 100%;
}
.n-spin-container .cm-editor {
  height: 100%;
}
.n-spin-container .cm-scroller {
  overflow: auto !important;
}
</style>
