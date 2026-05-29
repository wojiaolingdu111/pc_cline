import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { convertFileSrc } from "@tauri-apps/api/core";
import type {
    GenerateSpeechPayload,
    GenerateSpeechResult,
    TaskStatus,
} from "../api/tauri";
import { generateSpeech } from "../api/tauri";

export interface SpeechTask {
    id: string;
    text: string;
    voiceId: string;
    status: TaskStatus;
    audioPath?: string;
    error?: string;
    createdAt: string;
}

export const useTtsStore = defineStore("tts", () => {
    const currentText = ref("你好，欢迎使用本地语音合成应用。");
    const currentSpeed = ref(1);
    const currentLanguage = ref("zh");
    const currentAudioPath = ref("");
    const currentResult = ref<GenerateSpeechResult | null>(null);
    const isGenerating = ref(false);
    const tasks = ref<SpeechTask[]>([]);

    const latestTask = computed(() => tasks.value[0] ?? null);

    async function submitGeneration(voiceId: string) {
        const payload: GenerateSpeechPayload = {
            text: currentText.value.trim(),
            voiceId,
            speed: currentSpeed.value,
            language: currentLanguage.value,
            outputFormat: "wav",
        };

        if (!payload.text) {
            throw new Error("请输入要合成的文本。");
        }

        isGenerating.value = true;
        const draftTask: SpeechTask = {
            id: `draft-${Date.now()}`,
            text: payload.text,
            voiceId,
            status: "processing",
            createdAt: new Date().toISOString(),
        };
        tasks.value.unshift(draftTask);

        try {
            const result = await generateSpeech(payload);
            // Convert local file path to webview-accessible URL
            const webPath = result.audioPath
                ? convertFileSrc(result.audioPath)
                : "";
            currentResult.value = { ...result, audioPath: webPath };
            currentAudioPath.value = webPath;
            tasks.value[0] = {
                ...draftTask,
                id: result.taskId,
                status: result.status,
                audioPath: webPath,
                error: result.error,
            };
            return result;
        } catch (error) {
            const message =
                error instanceof Error ? error.message : "生成失败。";
            tasks.value[0] = {
                ...draftTask,
                status: "failed",
                error: message,
            };
            throw error;
        } finally {
            isGenerating.value = false;
        }
    }

    return {
        currentText,
        currentSpeed,
        currentLanguage,
        currentAudioPath,
        currentResult,
        isGenerating,
        tasks,
        latestTask,
        submitGeneration,
    };
});
