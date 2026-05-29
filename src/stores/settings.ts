import { ref } from "vue";
import { defineStore } from "pinia";
import { getServiceStatus, type ServiceStatus } from "../api/tauri";

export const useSettingsStore = defineStore("settings", () => {
    const outputDirectory = ref("app-data/outputs");
    const defaultFormat = ref<"wav">("wav");
    const defaultSpeed = ref(1);
    const currentLanguage = ref("zh");
    const serviceStatus = ref<ServiceStatus>({
        running: false,
        mode: "qwen3",
        modelLoaded: false,
        message: "服务状态未加载。",
    });

    async function refreshServiceStatus() {
        serviceStatus.value = await getServiceStatus();
    }

    return {
        outputDirectory,
        defaultFormat,
        defaultSpeed,
        currentLanguage,
        serviceStatus,
        refreshServiceStatus,
    };
});
