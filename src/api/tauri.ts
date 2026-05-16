import { invoke } from '@tauri-apps/api/core';

export type TaskStatus = 'pending' | 'processing' | 'success' | 'failed';

export interface VoiceProfile {
  id: string;
  name: string;
  type: 'builtin' | 'custom';
  language: string[];
  previewAudio?: string;
  description?: string;
}

export interface GenerateSpeechPayload {
  text: string;
  voiceId: string;
  speed: number;
  language: string;
  outputFormat: 'wav';
}

export interface GenerateSpeechResult {
  taskId: string;
  status: TaskStatus;
  audioPath?: string;
  error?: string;
  durationMs?: number;
}

export interface CloneVoicePayload {
  name: string;
  audioPath: string;
  language: string;
}

export interface CloneVoiceResult {
  voiceProfileId: string;
  status: TaskStatus;
}

export interface VoicesResponse {
  builtinVoices: VoiceProfile[];
  customVoices: VoiceProfile[];
}

export interface ServiceStatus {
  running: boolean;
  mode: 'mock' | 'coqui' | 'offline';
  baseUrl: string;
  message: string;
}

export interface LicenseInfo {
  status: 'Trial' | 'Active' | 'Expired';
  trial_days_total: number;
  trial_days_left: number;
  license_key: string | null;
  message: string;
}

interface PythonVoiceProfile {
  id: string;
  name: string;
  type: 'builtin' | 'custom';
  language: string[];
  preview_audio?: string | null;
  description?: string;
}

interface PythonVoicesResponse {
  builtin_voices: PythonVoiceProfile[];
  custom_voices: PythonVoiceProfile[];
}

interface PythonGenerateSpeechResponse {
  task_id: string;
  status: TaskStatus;
  audio_path?: string;
  error?: string;
  duration_ms?: number;
}

interface PythonCloneVoiceResponse {
  voice_profile_id: string;
  status: TaskStatus;
}

interface PythonHealthResponse {
  status: string;
  mode: 'mock' | 'coqui';
  model_loaded?: boolean;
}

const PYTHON_BASE_URL = 'http://127.0.0.1:8765';

const builtinMockVoices: VoiceProfile[] = [
  {
    id: 'female_01',
    name: '温柔女声',
    type: 'builtin',
    language: ['zh'],
    description: '适合客服、旁白和引导音。',
  },
  {
    id: 'female_02',
    name: '明亮女声',
    type: 'builtin',
    language: ['zh'],
    description: '适合短视频和内容播报。',
  },
  {
    id: 'male_01',
    name: '沉稳男声',
    type: 'builtin',
    language: ['zh'],
    description: '适合解说和资讯播报。',
  },
  {
    id: 'male_02',
    name: '清晰男声',
    type: 'builtin',
    language: ['zh'],
    description: '适合教程和产品介绍。',
  },
  {
    id: 'narrator_01',
    name: '中性旁白',
    type: 'builtin',
    language: ['zh', 'en'],
    description: '适合故事和说明文案。',
  },
];

function isTauriRuntime() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function toAbsoluteUrl(path: string | undefined) {
  if (!path) {
    return undefined;
  }

  if (path.startsWith('http://') || path.startsWith('https://') || path.startsWith('asset:')) {
    return path;
  }

  if (path.startsWith('/')) {
    return `${PYTHON_BASE_URL}${path}`;
  }

  return path;
}

function normalizeVoiceProfile(profile: PythonVoiceProfile): VoiceProfile {
  return {
    id: profile.id,
    name: profile.name,
    type: profile.type,
    language: profile.language,
    previewAudio: toAbsoluteUrl(profile.preview_audio ?? undefined),
    description: profile.description,
  };
}

function normalizeVoicesResponse(response: PythonVoicesResponse): VoicesResponse {
  return {
    builtinVoices: response.builtin_voices.map(normalizeVoiceProfile),
    customVoices: response.custom_voices.map(normalizeVoiceProfile),
  };
}

function normalizeGenerateSpeechResponse(response: PythonGenerateSpeechResponse): GenerateSpeechResult {
  return {
    taskId: response.task_id,
    status: response.status,
    audioPath: toAbsoluteUrl(response.audio_path),
    error: response.error,
    durationMs: response.duration_ms,
  };
}

function normalizeCloneVoiceResponse(response: PythonCloneVoiceResponse): CloneVoiceResult {
  return {
    voiceProfileId: response.voice_profile_id,
    status: response.status,
  };
}

async function readErrorMessage(response: Response) {
  try {
    const data = (await response.json()) as { detail?: string };
    return data.detail || `请求失败: ${response.status}`;
  } catch {
    return `请求失败: ${response.status}`;
  }
}

async function fetchPython<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${PYTHON_BASE_URL}${path}`, {
    headers: {
      'Content-Type': 'application/json',
      ...(options?.headers ?? {}),
    },
    ...options,
  });

  if (!response.ok) {
    throw new Error(await readErrorMessage(response));
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json() as Promise<T>;
}

async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) {
    throw new Error('当前未运行在 Tauri 环境中。');
  }

  return invoke<T>(command, args);
}

export async function getServiceStatus(): Promise<ServiceStatus> {
  if (!isTauriRuntime()) {
    try {
      const health = await fetchPython<PythonHealthResponse>('/health');
      return {
        running: health.status === 'ok',
        mode: health.mode,
        baseUrl: PYTHON_BASE_URL,
        message: '当前为浏览器预览模式，前端已直接连接 Python 服务。',
      };
    } catch {
      return {
        running: false,
        mode: 'offline',
        baseUrl: PYTHON_BASE_URL,
        message: '当前为浏览器预览模式，请先启动 Python 服务。',
      };
    }
  }

  return safeInvoke<ServiceStatus>('get_service_status');
}

export async function listVoices(): Promise<VoicesResponse> {
  if (!isTauriRuntime()) {
    try {
      const response = await fetchPython<PythonVoicesResponse>('/voices');
      return normalizeVoicesResponse(response);
    } catch {
      return {
        builtinVoices: builtinMockVoices,
        customVoices: [],
      };
    }
  }

  return safeInvoke<VoicesResponse>('list_voices');
}

export async function generateSpeech(payload: GenerateSpeechPayload): Promise<GenerateSpeechResult> {
  if (!isTauriRuntime()) {
    const response = await fetchPython<PythonGenerateSpeechResponse>('/generate-speech', {
      method: 'POST',
      body: JSON.stringify(payload),
    });
    return normalizeGenerateSpeechResponse(response);
  }

  return safeInvoke<GenerateSpeechResult>('generate_speech', { payload });
}

export async function cloneVoice(payload: CloneVoicePayload): Promise<CloneVoiceResult> {
  if (!isTauriRuntime()) {
    const response = await fetchPython<PythonCloneVoiceResponse>('/clone-voice', {
      method: 'POST',
      body: JSON.stringify(payload),
    });
    return normalizeCloneVoiceResponse(response);
  }

  return safeInvoke<CloneVoiceResult>('clone_voice', { payload });
}

export async function deleteVoiceProfile(voiceProfileId: string): Promise<void> {
  if (!isTauriRuntime()) {
    await fetchPython(`/voices/${voiceProfileId}`, {
      method: 'DELETE',
    });
    return;
  }

  await safeInvoke('delete_voice_profile', { voiceProfileId });
}

export async function getLicenseStatus(): Promise<LicenseInfo> {
  if (!isTauriRuntime()) {
    return {
      status: 'Trial',
      trial_days_total: 7,
      trial_days_left: 7,
      license_key: null,
      message: '浏览器预览模式',
    };
  }
  return safeInvoke<LicenseInfo>('get_license_status');
}

export async function activateLicense(key: string): Promise<LicenseInfo> {
  return safeInvoke<LicenseInfo>('activate_license', { key });
}

export async function pickAudioFile(): Promise<string | null> {
  return safeInvoke<string | null>('pick_audio_file');
}
