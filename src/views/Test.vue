<template>
  <div>
    <n-h2>{{ t('test.title') }}</n-h2>
    <n-grid :cols="24" :x-gap="16">
      <n-gi :span="10">
        <n-card :title="t('test.title')" size="small">
          <n-form label-placement="top">
            <n-form-item :label="t('test.format')">
              <n-select v-model:value="selectedFormat" :options="formatOptions" />
            </n-form-item>
            <n-form-item :label="t('test.model')">
              <n-input v-model:value="modelName" placeholder="gpt-4o" />
            </n-form-item>
            <n-form-item :label="t('test.systemPrompt')">
              <n-input
                v-model:value="systemPrompt"
                type="textarea"
                :rows="2"
                :placeholder="t('test.systemPromptPlaceholder')"
              />
            </n-form-item>
            <n-form-item :label="t('test.userMessage')">
              <n-input
                v-model:value="userMessage"
                type="textarea"
                :rows="3"
                :placeholder="t('test.userMessagePlaceholder')"
              />
            </n-form-item>
            <n-form-item :label="t('test.temperature')">
              <n-slider v-model:value="temperature" :min="0" :max="2" :step="0.1" />
            </n-form-item>
            <n-form-item :label="t('test.maxTokens')">
              <n-input-number v-model:value="maxTokens" :min="1" :max="128000" style="width: 100%" />
            </n-form-item>
            <n-form-item :label="t('test.stream')">
              <n-switch v-model:value="stream" />
            </n-form-item>
            <n-form-item :label="t('test.proxyUrl')">
              <n-input v-model:value="proxyUrl" placeholder="http://127.0.0.1:7860" />
            </n-form-item>
            <n-space>
              <n-button type="primary" :loading="sending" :disabled="!canSend" @click="sendRequest">
                {{ sending ? t('test.sending') : t('test.sendRequest') }}
              </n-button>
              <n-button @click="clearResponse">{{ t('test.clearResponse') }}</n-button>
            </n-space>
          </n-form>
        </n-card>

        <n-collapse style="margin-top: 12px">
          <n-collapse-item :title="t('test.requestPreview')" name="preview">
            <n-code :code="requestPreview" language="json" />
          </n-collapse-item>
        </n-collapse>
      </n-gi>

      <n-gi :span="14">
        <n-card :title="t('test.response')" size="small">
          <template v-if="hasResponse">
            <n-space align="center" style="margin-bottom: 12px">
              <n-tag :type="statusCode && statusCode < 400 ? 'success' : 'error'" size="small">
                {{ t('test.statusCode') }}: {{ statusCode }}
              </n-tag>
              <n-tag size="small">{{ t('test.responseTime') }}: {{ responseTime }}ms</n-tag>
            </n-space>

            <template v-if="hasTokenUsage">
              <n-descriptions bordered size="small" :column="3" style="margin-bottom: 12px">
                <n-descriptions-item :label="t('test.promptTokens')">{{ promptTokens }}</n-descriptions-item>
                <n-descriptions-item :label="t('test.completionTokens')">{{ completionTokens }}</n-descriptions-item>
                <n-descriptions-item :label="t('test.totalTokens')">{{ totalTokens }}</n-descriptions-item>
              </n-descriptions>
            </template>

            <template v-if="errorMessage">
              <n-alert type="error" style="margin-bottom: 12px">{{ errorMessage }}</n-alert>
            </template>

            <n-scrollbar style="max-height: 500px">
              <n-code :code="responseBody" language="json" />
            </n-scrollbar>
          </template>

          <n-empty v-else :description="t('test.noResponse')" style="padding: 60px 0" />
        </n-card>
      </n-gi>
    </n-grid>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const selectedFormat = ref<string>('completions')
const modelName = ref('gpt-4o')
const systemPrompt = ref('')
const userMessage = ref('')
const temperature = ref(0.7)
const maxTokens = ref(1024)
const stream = ref(false)
const proxyUrl = ref('http://127.0.0.1:7860')

const sending = ref(false)
const statusCode = ref<number | null>(null)
const responseTime = ref<number | null>(null)
const responseBody = ref('')
const errorMessage = ref<string | null>(null)
const promptTokens = ref<number | null>(null)
const completionTokens = ref<number | null>(null)
const totalTokens = ref<number | null>(null)

const formatOptions = computed(() => [
  { label: t('format.completions'), value: 'completions' },
  { label: t('format.responses'), value: 'responses' },
  { label: t('format.anthropic'), value: 'anthropic' },
  { label: t('format.gemini'), value: 'gemini' },
])

const canSend = computed(() => modelName.value.trim() !== '' && userMessage.value.trim() !== '')
const hasResponse = computed(() => statusCode.value !== null || responseBody.value !== '' || errorMessage.value !== null)
const hasTokenUsage = computed(() => promptTokens.value !== null)

const requestPreview = computed(() => {
  return JSON.stringify({ url: buildUrl(), method: 'POST', body: buildRequestBody() }, null, 2)
})

function buildUrl(): string {
  const base = proxyUrl.value.replace(/\/$/, '')
  switch (selectedFormat.value) {
    case 'completions':
      return `${base}/v1/chat/completions`
    case 'responses':
      return `${base}/v1/responses`
    case 'anthropic':
      return `${base}/v1/messages`
    case 'gemini':
      return `${base}/v1beta/models/${modelName.value}:generateContent`
    default:
      return `${base}/v1/chat/completions`
  }
}

function buildRequestBody(): Record<string, unknown> {
  const messages: Array<{ role: string; content: string }> = []
  if (systemPrompt.value) {
    messages.push({ role: 'system', content: systemPrompt.value })
  }
  messages.push({ role: 'user', content: userMessage.value })

  switch (selectedFormat.value) {
    case 'completions':
      return {
        model: modelName.value,
        messages,
        temperature: temperature.value,
        max_tokens: maxTokens.value,
        stream: stream.value,
      }
    case 'responses':
      return {
        model: modelName.value,
        input: userMessage.value,
        ...(systemPrompt.value ? { instructions: systemPrompt.value } : {}),
        temperature: temperature.value,
        max_output_tokens: maxTokens.value,
        stream: stream.value,
      }
    case 'anthropic':
      return {
        model: modelName.value,
        messages: messages.filter(m => m.role !== 'system'),
        ...(systemPrompt.value ? { system: systemPrompt.value } : {}),
        max_tokens: maxTokens.value,
        temperature: temperature.value,
        stream: stream.value,
      }
    case 'gemini':
      return {
        contents: messages
          .filter(m => m.role !== 'system')
          .map(m => ({
            role: m.role === 'assistant' ? 'model' : 'user',
            parts: [{ text: m.content }],
          })),
        ...(systemPrompt.value ? { systemInstruction: { parts: [{ text: systemPrompt.value }] } } : {}),
        generationConfig: {
          temperature: temperature.value,
          maxOutputTokens: maxTokens.value,
        },
      }
    default:
      return { model: modelName.value, messages, stream: stream.value }
  }
}

function extractTokenUsage(json: Record<string, unknown>) {
  const usage = json.usage as Record<string, unknown> | undefined
  if (!usage) return

  if ('prompt_tokens' in usage) {
    promptTokens.value = (usage.prompt_tokens as number) ?? null
    completionTokens.value = (usage.completion_tokens as number) ?? null
    totalTokens.value = (usage.total_tokens as number) ?? null
  } else if ('input_tokens' in usage) {
    promptTokens.value = (usage.input_tokens as number) ?? null
    completionTokens.value = (usage.output_tokens as number) ?? null
    totalTokens.value = (promptTokens.value ?? 0) + (completionTokens.value ?? 0)
  }
}

async function sendRequest() {
  sending.value = true
  statusCode.value = null
  responseTime.value = null
  responseBody.value = ''
  errorMessage.value = null
  promptTokens.value = null
  completionTokens.value = null
  totalTokens.value = null

  const url = buildUrl()
  const body = buildRequestBody()
  const start = performance.now()

  try {
    const resp = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })

    const elapsed = Math.round(performance.now() - start)
    statusCode.value = resp.status
    responseTime.value = elapsed

    if (stream.value && resp.body) {
      const reader = resp.body.getReader()
      const decoder = new TextDecoder()
      let chunks = ''
      let done = false
      while (!done) {
        const result = await reader.read()
        done = result.done
        if (result.value) {
          chunks += decoder.decode(result.value, { stream: true })
        }
      }
      responseBody.value = chunks
    } else {
      const text = await resp.text()
      try {
        const json = JSON.parse(text)
        responseBody.value = JSON.stringify(json, null, 2)
        extractTokenUsage(json)
      } catch {
        responseBody.value = text
      }
    }

    if (!resp.ok) {
      errorMessage.value = `HTTP ${resp.status}`
    }
  } catch (e) {
    errorMessage.value = String(e)
  } finally {
    sending.value = false
  }
}

function clearResponse() {
  statusCode.value = null
  responseTime.value = null
  responseBody.value = ''
  errorMessage.value = null
  promptTokens.value = null
  completionTokens.value = null
  totalTokens.value = null
}
</script>
