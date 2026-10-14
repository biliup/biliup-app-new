<template>
    <div id="app">
        <div v-if="startupError" class="config-recovery">
            <el-alert :title="t('app.recovery.title')" type="error" :closable="false" show-icon />
            <p>{{ t('app.recovery.p1') }}</p>
            <pre>{{ startupError }}</pre>
            <p>{{ t('app.recovery.p2') }}</p>
            <p>{{ t('app.recovery.p3') }}</p>
        </div>
        <el-config-provider v-else-if="startupChecked" :locale="elementLocale">
            <router-view />
        </el-config-provider>
    </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18nState } from './i18n'

const startupError = ref<string | null>(null)
const startupChecked = ref(false)
const { t, elementLocale } = useI18nState()

onMounted(async () => {
    try {
        startupError.value = await invoke<string | null>('get_startup_error')
    } catch (error) {
        startupError.value = t('app.recovery.checkError', { error: String(error) })
    } finally {
        startupChecked.value = true
    }
})
</script>

<style>
.config-recovery {
    padding: 32px;
    max-width: 900px;
    margin: 0 auto;
}
.config-recovery p,
.config-recovery pre {
    margin-top: 20px;
    line-height: 1.6;
}
.config-recovery pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
}
#app {
    height: 100vh;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', sans-serif;
}

* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    background-color: #f5f5f5;
}

html[lang='ja-JP'] .el-form-item__label {
    height: auto;
    min-height: var(--el-component-size);
    align-self: flex-start;
    padding-top: 6px;
    padding-bottom: 6px;
    line-height: 20px;
}
</style>
