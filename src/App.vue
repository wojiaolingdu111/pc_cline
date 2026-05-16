<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import AudioPlayer from './components/AudioPlayer.vue';
import CloneVoicePanel from './components/CloneVoicePanel.vue';
import TaskList from './components/TaskList.vue';
import TextInputPanel from './components/TextInputPanel.vue';
import VoiceSelector from './components/VoiceSelector.vue';
import { useSettingsStore } from './stores/settings';
import { useTtsStore } from './stores/tts';
import { useVoicesStore } from './stores/voices';
import { getLicenseStatus, activateLicense, type LicenseInfo } from './api/tauri';

const settingsStore = useSettingsStore();
const ttsStore = useTtsStore();
const voicesStore = useVoicesStore();
const runtimeError = ref('');
const licenseInfo = ref<LicenseInfo | null>(null);
const showActivation = ref(false);
const licenseKeyInput = ref('');
const activationError = ref('');
const activating = ref(false);

const serviceBadge = computed(() => {
  if (settingsStore.serviceStatus.running) return '服务运行中';
  if (settingsStore.serviceStatus.mode === 'offline') return '浏览器预览模式';
  return '服务未连接';
});

const licenseBadge = computed(() => {
  if (!licenseInfo.value) return '';
  if (licenseInfo.value.status === 'Active') return '已激活';
  if (licenseInfo.value.status === 'Expired') return '试用过期';
  return `试用 ${licenseInfo.value.trial_days_left} 天`;
});

const licenseBadgeClass = computed(() => {
  if (!licenseInfo.value) return '';
  if (licenseInfo.value.status === 'Active') return 'license-active';
  if (licenseInfo.value.status === 'Expired') return 'license-expired';
  return 'license-trial';
});

async function bootstrap() {
  runtimeError.value = '';
  try {
    const [license] = await Promise.all([
      getLicenseStatus(),
      settingsStore.refreshServiceStatus(),
      voicesStore.refreshVoices(),
    ]);
    licenseInfo.value = license;
    if (license.status === 'Expired') showActivation.value = true;
  } catch (error) {
    runtimeError.value = error instanceof Error ? error.message : '初始化失败。';
  }
}

async function handleActivate() {
  if (!licenseKeyInput.value.trim()) return;
  activationError.value = '';
  activating.value = true;
  try {
    const info = await activateLicense(licenseKeyInput.value.trim());
    licenseInfo.value = info;
    showActivation.value = false;
  } catch (error) {
    activationError.value = error instanceof Error ? error.message : '激活失败';
  } finally {
    activating.value = false;
  }
}

onMounted(() => { void bootstrap(); });
</script>

<template>
  <div class="shell">
    <header class="hero">
      <div>
        <p class="eyebrow">PC 端本地语音合成</p>
        <h1>AI ToReder</h1>
      </div>
      <div class="status-panel">
        <div class="badge-row">
          <span class="status-chip">{{ serviceBadge }}</span>
          <span v-if="licenseInfo" class="status-chip" :class="licenseBadgeClass">{{ licenseBadge }}</span>
          <button v-if="licenseInfo?.status === 'Expired'" class="ghost-btn-sm" @click="showActivation = true">激活</button>
        </div>
        <p>{{ settingsStore.serviceStatus.message }}</p>
      </div>
    </header>

    <p v-if="runtimeError" class="runtime-error">{{ runtimeError }}</p>

    <div v-if="showActivation" class="modal-overlay" @click.self="showActivation = false">
      <div class="modal">
        <h2>{{ licenseInfo?.status === 'Expired' ? '试用已过期' : '激活授权' }}</h2>
        <p class="modal-desc">输入您的授权码激活 AI ToReder</p>
        <input v-model="licenseKeyInput" type="text" placeholder="输入授权码" class="modal-input" @keyup.enter="handleActivate" />
        <p v-if="activationError" class="modal-error">{{ activationError }}</p>
        <div class="modal-actions">
          <button v-if="licenseInfo?.status !== 'Expired'" class="btn-outline" @click="showActivation = false">取消</button>
          <button class="btn-primary" :disabled="activating || !licenseKeyInput.trim()" @click="handleActivate">
            {{ activating ? '验证中…' : '激活' }}
          </button>
        </div>
        <p class="modal-footer">还未购买？<a href="https://ai-toreder.vercel.app" target="_blank">前往购买</a></p>
      </div>
    </div>

    <main class="workspace-grid">
      <section class="panel panel-main">
        <TextInputPanel />
        <AudioPlayer :audio-path="ttsStore.currentAudioPath" :result="ttsStore.currentResult" />
      </section>
      <section class="panel panel-side">
        <VoiceSelector />
        <CloneVoicePanel />
      </section>
      <section class="panel panel-wide">
        <TaskList :tasks="ttsStore.tasks" />
      </section>
    </main>
  </div>
</template>

<style scoped>
.shell { max-width:1240px; margin:0 auto; padding:40px 20px 56px; }
.hero { display:grid; gap:24px; grid-template-columns:minmax(0,1.6fr) minmax(280px,0.9fr); align-items:end; margin-bottom:28px; }
.eyebrow { margin:0 0 8px; font-size:13px; font-weight:700; letter-spacing:.18em; text-transform:uppercase; color:#117864; }
h1 { margin:0; font-size:clamp(2.8rem,5vw,4.8rem); line-height:.95; }
.status-panel { padding:22px; border:1px solid rgba(20,33,61,.08); border-radius:24px; background:rgba(255,255,255,.84); box-shadow:0 18px 45px rgba(20,33,61,.09); }
.badge-row { display:flex; gap:8px; align-items:center; flex-wrap:wrap; }
.status-chip { display:inline-flex; align-items:center; padding:6px 12px; border-radius:999px; background:#14213d; color:#fff; font-size:12px; font-weight:700; }
.license-active { background:#166534; }
.license-expired { background:#8a1c28; }
.license-trial { background:#92400e; }
.status-panel p { margin:12px 0 0; color:rgba(20,33,61,.74); }
.runtime-error { margin:0 0 18px; padding:14px 16px; border-radius:16px; background:rgba(176,42,55,.12); color:#8a1c28; }
.workspace-grid { display:grid; gap:20px; grid-template-columns:minmax(0,1.6fr) minmax(320px,0.95fr); }
.panel { border:1px solid rgba(20,33,61,.08); border-radius:28px; background:rgba(255,255,255,.86); box-shadow:0 20px 50px rgba(20,33,61,.08); backdrop-filter:blur(12px); }
.panel-main,.panel-side,.panel-wide { padding:24px; }
.panel-side { display:grid; gap:20px; align-content:start; }
.panel-wide { grid-column:1/-1; }
.modal-overlay { position:fixed; inset:0; background:rgba(20,33,61,.5); display:flex; align-items:center; justify-content:center; z-index:100; backdrop-filter:blur(4px); }
.modal { background:#fff; border-radius:28px; padding:36px; max-width:420px; width:90%; box-shadow:0 40px 80px rgba(20,33,61,.2); }
.modal h2 { margin:0 0 8px; font-size:1.4rem; }
.modal-desc { margin:0 0 20px; color:rgba(20,33,61,.7); }
.modal-input { width:100%; padding:14px 16px; border:1px solid rgba(20,33,61,.15); border-radius:14px; font-size:1rem; outline:none; transition:border-color .2s; }
.modal-input:focus { border-color:#ff9f1c; }
.modal-error { margin:8px 0 0; color:#8a1c28; font-size:.9rem; }
.modal-actions { display:flex; gap:12px; justify-content:flex-end; margin-top:20px; }
.modal-actions button { padding:10px 24px; border-radius:12px; font-weight:700; cursor:pointer; border:none; font-size:.95rem; }
.modal-actions .btn-primary { background:linear-gradient(135deg,#ff9f1c,#ffbf69); color:#14213d; }
.modal-actions .btn-primary:disabled { opacity:.5; cursor:not-allowed; }
.modal-actions .btn-outline { background:transparent; border:1px solid rgba(20,33,61,.15); color:#14213d; }
.modal-footer { margin:20px 0 0; text-align:center; font-size:.85rem; color:rgba(20,33,61,.6); }
.modal-footer a { color:#0d9488; text-decoration:none; font-weight:700; }
.ghost-btn-sm { padding:6px 12px; border-radius:999px; border:1px solid rgba(20,33,61,.15); background:transparent; color:#8a1c28; font-size:12px; font-weight:700; cursor:pointer; }
@media (max-width:960px) { .hero,.workspace-grid { grid-template-columns:1fr; } .shell { padding:24px 16px 40px; } }
</style>
