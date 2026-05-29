import { invoke } from "@tauri-apps/api/core";

export type TaskStatus = "pending" | "processing" | "success" | "failed";

export interface VoiceProfile {
    id: string;
    name: string;
    type: "builtin" | "custom";
    language: string[];
    previewAudio?: string;
    description?: string;
}

export interface GenerateSpeechPayload {
    text: string;
    voiceId: string;
    speed: number;
    language: string;
    outputFormat: "wav";
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
    mode: "coqui" | "qwen3";
    modelLoaded: boolean;
    message: string;
}

export interface LicenseInfo {
    status: "Trial" | "Active" | "Expired";
    trial_days_total: number;
    trial_days_left: number;
    license_key: string | null;
    message: string;
}

// ---------------------------------------------------------------------------
// Mock data for browser preview
// ---------------------------------------------------------------------------

const builtinMockVoices: VoiceProfile[] = [
    {
        id: "female_01",
        name: "温柔女声",
        type: "builtin",
        language: ["zh"],
        description: "适合客服、旁白和引导音。",
    },
    {
        id: "female_02",
        name: "明亮女声",
        type: "builtin",
        language: ["zh"],
        description: "适合短视频和内容播报。",
    },
    {
        id: "male_01",
        name: "沉稳男声",
        type: "builtin",
        language: ["zh"],
        description: "适合解说和资讯播报。",
    },
    {
        id: "male_02",
        name: "清晰男声",
        type: "builtin",
        language: ["zh"],
        description: "适合教程和产品介绍。",
    },
    {
        id: "narrator_01",
        name: "中性旁白",
        type: "builtin",
        language: ["zh", "en"],
        description: "适合故事和说明文案。",
    },
];

// ---------------------------------------------------------------------------
// Runtime detection & helpers
// ---------------------------------------------------------------------------

function isTauriRuntime() {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function safeInvoke<T>(
    command: string,
    args?: Record<string, unknown>,
): Promise<T> {
    if (!isTauriRuntime()) {
        throw new Error("当前未运行在 Tauri 环境中。");
    }

    return invoke<T>(command, args);
}

function tauriOnly<A extends unknown[], R>(
    fallback: () => R,
): (...args: A) => Promise<R> {
    return async (..._args: A) => {
        if (!isTauriRuntime()) {
            return fallback();
        }
        throw new Error(
            "This function should only be called from Tauri runtime",
        );
    };
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

export async function getServiceStatus(): Promise<ServiceStatus> {
    if (!isTauriRuntime()) {
        return {
            running: false,
            mode: "qwen3",
            modelLoaded: false,
            message: "浏览器预览模式：TTS 功能需在 Tauri 桌面应用中运行。",
        };
    }

    return safeInvoke<ServiceStatus>("get_service_status");
}

export async function listVoices(): Promise<VoicesResponse> {
    if (!isTauriRuntime()) {
        return {
            builtinVoices: builtinMockVoices,
            customVoices: [],
        };
    }

    return safeInvoke<VoicesResponse>("list_voices");
}

export async function generateSpeech(
    payload: GenerateSpeechPayload,
): Promise<GenerateSpeechResult> {
    if (!isTauriRuntime()) {
        return {
            taskId: "mock",
            status: "failed",
            error: "浏览器预览模式不支持语音合成。",
        };
    }

    return safeInvoke<GenerateSpeechResult>("generate_speech", { payload });
}

export async function cloneVoice(
    payload: CloneVoicePayload,
): Promise<CloneVoiceResult> {
    if (!isTauriRuntime()) {
        return {
            voiceProfileId: "mock",
            status: "failed",
        };
    }

    return safeInvoke<CloneVoiceResult>("clone_voice", { payload });
}

export async function deleteVoiceProfile(
    voiceProfileId: string,
): Promise<void> {
    if (!isTauriRuntime()) {
        return;
    }

    await safeInvoke("delete_voice_profile", { voiceProfileId });
}

export async function getLicenseStatus(): Promise<LicenseInfo> {
    if (!isTauriRuntime()) {
        return {
            status: "Trial",
            trial_days_total: 7,
            trial_days_left: 7,
            license_key: null,
            message: "浏览器预览模式",
        };
    }

    return safeInvoke<LicenseInfo>("get_license_status");
}

export async function activateLicense(key: string): Promise<LicenseInfo> {
    return safeInvoke<LicenseInfo>("activate_license", { key });
}

export async function pickAudioFile(): Promise<string | null> {
    if (!isTauriRuntime()) {
        return null;
    }

    return safeInvoke<string | null>("pick_audio_file");
}
