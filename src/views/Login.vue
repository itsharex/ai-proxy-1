<template>
  <div class="login-shell">
    <!-- Left hero panel -->
    <aside class="hero-panel">
      <div class="grid-bg" />
      <div class="glow-bg" />

      <div class="hero-content">
        <header class="hero-header">
          <div class="brand">
            <span class="brand-mark" />
            <svg class="brand-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 2L2 7l10 5 10-5-10-5z"/>
              <path d="M2 17l10 5 10-5"/>
              <path d="M2 12l10 5 10-5"/>
            </svg>
            <span class="brand-name">AI<span class="brand-sep">·</span>PROXY</span>
          </div>
          <div class="hero-tagline fade-in" style="animation-delay: 0.1s">
            UNIFIED GATEWAY · OPENAI · ANTHROPIC · GEMINI
          </div>
        </header>

        <div class="topology">
          <svg viewBox="0 0 400 320" class="topology-svg">
            <!-- Connections -->
            <g class="links">
              <path class="link link-1" d="M 60 60 Q 200 100 200 160" />
              <path class="link link-2" d="M 60 160 L 200 160" />
              <path class="link link-3" d="M 60 260 Q 200 220 200 160" />
              <path class="link link-out" d="M 200 160 L 340 160" />
            </g>

            <!-- Upstream nodes -->
            <g class="node node-up fade-in" style="animation-delay: 0.2s">
              <circle cx="60" cy="60" r="6" />
              <text x="78" y="64">openai</text>
            </g>
            <g class="node node-up fade-in" style="animation-delay: 0.3s">
              <circle cx="60" cy="160" r="6" />
              <text x="78" y="164">anthropic</text>
            </g>
            <g class="node node-up fade-in" style="animation-delay: 0.4s">
              <circle cx="60" cy="260" r="6" />
              <text x="78" y="264">gemini</text>
            </g>

            <!-- Center node -->
            <g class="node node-center fade-in" style="animation-delay: 0.5s">
              <circle cx="200" cy="160" r="22" class="center-halo" />
              <circle cx="200" cy="160" r="10" class="center-core" />
              <text x="200" y="200" class="center-label">AI PROXY</text>
            </g>

            <!-- Client node -->
            <g class="node node-client fade-in" style="animation-delay: 0.6s">
              <rect x="330" y="152" width="20" height="16" rx="2" />
              <text x="356" y="164">client</text>
            </g>
          </svg>
        </div>

        <footer class="telemetry fade-in" style="animation-delay: 0.9s">
          <span class="tel-item"><span class="tel-dot" /> SYSTEM ONLINE</span>
          <span class="tel-sep">/</span>
          <span class="tel-item">v{{ version }}</span>
          <span class="tel-sep">/</span>
          <span class="tel-item">127.0.0.1:7860</span>
          <span class="tel-sep">/</span>
          <span class="tel-item tel-bar"><span class="bar-fill" /> 8/10 MODELS</span>
        </footer>
      </div>
    </aside>

    <!-- Right form panel -->
    <main class="form-panel">
      <div class="form-wrap fade-in" style="animation-delay: 0.5s">
        <div class="form-header">
          <div class="form-eyebrow">CONSOLE ACCESS</div>
          <h1 class="form-title">欢迎回来</h1>
          <p class="form-subtitle">请输入管理员凭据继续</p>
        </div>

        <form class="login-form" @submit.prevent="handleSubmit">
          <div class="field">
            <label class="field-label">USERNAME</label>
            <n-input
              v-model:value="form.username"
              placeholder="admin"
              :input-props="{ autocomplete: 'username', spellcheck: 'false' }"
              class="field-input"
              @keyup.enter="handleSubmit"
            />
          </div>

          <div class="field">
            <label class="field-label">PASSWORD</label>
            <n-input
              v-model:value="form.password"
              type="password"
              show-password-on="click"
              placeholder="••••••••"
              :input-props="{ autocomplete: 'current-password' }"
              class="field-input"
              @keyup.enter="handleSubmit"
            />
          </div>

          <button
            type="submit"
            class="submit-btn"
            :class="{ loading: submitting }"
            :disabled="submitting"
          >
            <span v-if="!submitting" class="submit-label">
              登录 <span class="submit-arrow">→</span>
            </span>
            <span v-else class="submit-loading">[ AUTHENTICATING... ]</span>
          </button>
        </form>

        <div class="form-footer">
          <span class="footer-hint">Protected by JWT · 24h session</span>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { useAuthStore } from '../stores/auth'
import pkg from '../../package.json'

const router = useRouter()
const route = useRoute()
const message = useMessage()
const authStore = useAuthStore()

const version = pkg.version

const submitting = ref(false)

const form = reactive({
  username: '',
  password: '',
})

async function handleSubmit() {
  if (!form.username || !form.password) {
    message.error('请输入用户名和密码')
    return
  }
  submitting.value = true
  try {
    await authStore.login(form.username, form.password)
    const redirect = (route.query.redirect as string) || '/'
    router.replace(redirect)
  } catch (err) {
    const msg = err instanceof Error ? err.message : '登录失败'
    message.error(msg)
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.login-shell {
  display: grid;
  grid-template-columns: 1.4fr 1fr;
  min-height: 100vh;
  background: #0A0E14;
  color: #E4E8F0;
  font-family: var(--font-sans);
  overflow: hidden;
}

/* ===== HERO PANEL ===== */
.hero-panel {
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 56px 64px;
  border-right: 1px solid rgba(34, 211, 238, 0.12);
  overflow: hidden;
  min-height: 100vh;
}

.grid-bg {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(to right, rgba(34, 211, 238, 0.05) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(34, 211, 238, 0.05) 1px, transparent 1px);
  background-size: 48px 48px;
  mask-image: radial-gradient(ellipse at 30% 50%, #000 30%, transparent 80%);
  -webkit-mask-image: radial-gradient(ellipse at 30% 50%, #000 30%, transparent 80%);
  pointer-events: none;
}

.glow-bg {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(circle at 25% 40%, rgba(34, 211, 238, 0.15), transparent 55%),
    radial-gradient(circle at 80% 70%, rgba(99, 102, 241, 0.08), transparent 50%);
  pointer-events: none;
}

.hero-content {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  height: 100%;
  justify-content: space-between;
}

.hero-header {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.brand {
  display: flex;
  align-items: center;
  gap: 12px;
}

.brand-mark {
  width: 14px;
  height: 14px;
  background: #22D3EE;
  box-shadow: 0 0 16px rgba(34, 211, 238, 0.6);
}

.brand-icon {
  color: #22D3EE;
  opacity: 0.7;
}

.brand-name {
  font-family: var(--font-sans);
  font-size: 28px;
  font-weight: 700;
  letter-spacing: 0.04em;
  color: #E4E8F0;
}

.brand-sep {
  color: #22D3EE;
  margin: 0 2px;
  font-weight: 400;
}

.hero-tagline {
  font-family: var(--font-mono);
  font-size: 11px;
  color: rgba(34, 211, 238, 0.7);
  letter-spacing: 0.18em;
}

/* ===== TOPOLOGY ===== */
.topology {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 24px 0;
}

.topology-svg {
  width: 100%;
  max-width: 460px;
  height: auto;
}

.topology-svg .link {
  fill: none;
  stroke: rgba(34, 211, 238, 0.35);
  stroke-width: 1;
  stroke-dasharray: 4 4;
  animation: dash 1.5s linear infinite;
}

.topology-svg .link-out {
  stroke: rgba(34, 211, 238, 0.6);
  stroke-width: 1.5;
}

@keyframes dash {
  to { stroke-dashoffset: -16; }
}

.topology-svg .node text {
  font-family: var(--font-mono);
  font-size: 11px;
  fill: #8892A2;
  letter-spacing: 0.05em;
}

.topology-svg .node-up circle {
  fill: rgba(34, 211, 238, 0.3);
  stroke: #22D3EE;
  stroke-width: 1.5;
}

.topology-svg .node-center .center-halo {
  fill: rgba(34, 211, 238, 0.08);
  stroke: rgba(34, 211, 238, 0.3);
  stroke-width: 1;
  animation: pulse 2.4s ease-in-out infinite;
}

.topology-svg .node-center .center-core {
  fill: #22D3EE;
  filter: drop-shadow(0 0 8px rgba(34, 211, 238, 0.8));
}

.topology-svg .node-center .center-label {
  fill: #E4E8F0;
  font-weight: 600;
  letter-spacing: 0.15em;
  text-anchor: middle;
}

.topology-svg .node-client rect {
  fill: rgba(99, 102, 241, 0.2);
  stroke: #818CF8;
  stroke-width: 1.5;
}

.topology-svg .node-client text {
  fill: #818CF8;
}

@keyframes pulse {
  0%, 100% { opacity: 0.6; transform: scale(1); transform-origin: 200px 160px; }
  50% { opacity: 1; transform: scale(1.15); transform-origin: 200px 160px; }
}

/* ===== TELEMETRY ===== */
.telemetry {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  font-family: var(--font-mono);
  font-size: 11px;
  color: #555E6E;
  letter-spacing: 0.06em;
  padding-top: 16px;
  border-top: 1px solid rgba(255, 255, 255, 0.04);
}

.tel-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: #8892A2;
}

.tel-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #22C55E;
  box-shadow: 0 0 8px rgba(34, 197, 94, 0.6);
  animation: blink 2s ease-in-out infinite;
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.tel-sep {
  color: #2A303C;
}

.tel-bar .bar-fill {
  display: inline-block;
  width: 48px;
  height: 6px;
  background: linear-gradient(to right, #22D3EE 80%, rgba(255,255,255,0.08) 80%);
  margin-right: 2px;
}

/* ===== FORM PANEL ===== */
.form-panel {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 48px 56px;
  position: relative;
}

.form-wrap {
  width: 100%;
  max-width: 360px;
}

.form-header {
  margin-bottom: 36px;
}

.form-eyebrow {
  font-family: var(--font-mono);
  font-size: 11px;
  color: #22D3EE;
  letter-spacing: 0.18em;
  margin-bottom: 14px;
}

.form-title {
  font-family: var(--font-sans);
  font-size: 32px;
  font-weight: 600;
  color: #E4E8F0;
  margin: 0 0 8px;
  letter-spacing: -0.01em;
}

.form-subtitle {
  font-size: 13px;
  color: #8892A2;
  margin: 0;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 22px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.field-label {
  font-family: var(--font-mono);
  font-size: 10px;
  color: #555E6E;
  letter-spacing: 0.2em;
}

.field-input :deep(.n-input) {
  background: transparent !important;
}

.field-input :deep(.n-input .n-input__border),
.field-input :deep(.n-input .n-input__state-border) {
  border: none !important;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1) !important;
  border-radius: 0 !important;
  box-shadow: none !important;
  transition: border-color 0.2s, box-shadow 0.2s !important;
}

.field-input :deep(.n-input:focus-within .n-input__state-border),
.field-input :deep(.n-input--focus .n-input__state-border) {
  border-bottom-color: #22D3EE !important;
  box-shadow: 0 1px 0 0 #22D3EE !important;
}

.field-input :deep(.n-input__input-el),
.field-input :deep(.n-input__textarea-el) {
  color: #E4E8F0 !important;
  font-family: var(--font-mono) !important;
  font-size: 14px !important;
  caret-color: #22D3EE;
}

.field-input :deep(.n-input__placeholder) {
  color: #2A303C !important;
  font-family: var(--font-mono) !important;
}

.field-input :deep(.n-input__suffix) {
  color: #555E6E !important;
}

/* ===== SUBMIT BUTTON ===== */
.submit-btn {
  margin-top: 8px;
  height: 44px;
  background: transparent;
  border: 1px solid #22D3EE;
  color: #22D3EE;
  font-family: var(--font-mono);
  font-size: 13px;
  letter-spacing: 0.1em;
  cursor: pointer;
  position: relative;
  overflow: hidden;
  transition: color 0.2s, border-color 0.2s;
}

.submit-btn::before {
  content: '';
  position: absolute;
  inset: 0;
  background: #22D3EE;
  transform: translateX(-100%);
  transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  z-index: 0;
}

.submit-btn:not(:disabled):hover {
  color: #0A0E14;
}

.submit-btn:not(:disabled):hover::before {
  transform: translateX(0);
}

.submit-btn:disabled {
  cursor: not-allowed;
  opacity: 0.7;
}

.submit-label,
.submit-loading {
  position: relative;
  z-index: 1;
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.submit-arrow {
  transition: transform 0.2s;
}

.submit-btn:not(:disabled):hover .submit-arrow {
  transform: translateX(4px);
}

.submit-loading {
  animation: flicker 1.2s ease-in-out infinite;
}

@keyframes flicker {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

/* ===== FOOTER ===== */
.form-footer {
  margin-top: 32px;
  text-align: center;
}

.footer-hint {
  font-family: var(--font-mono);
  font-size: 10px;
  color: #2A303C;
  letter-spacing: 0.15em;
}

/* ===== ENTRANCE ANIMATION ===== */
.fade-in {
  opacity: 0;
  animation: fadeInUp 0.6s cubic-bezier(0.16, 1, 0.3, 1) forwards;
}

@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* ===== RESPONSIVE ===== */
@media (max-width: 880px) {
  .login-shell {
    grid-template-columns: 1fr;
  }
  .hero-panel {
    display: none;
  }
  .form-panel {
    min-height: 100vh;
  }
}
</style>
