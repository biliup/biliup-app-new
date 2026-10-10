<template>
    <div id="app">
        <div v-if="startupError" class="config-recovery">
            <el-alert
                title="配置加载失败，已进入受限恢复模式"
                type="error"
                :closable="false"
                show-icon
            />
            <p>原配置及备份已保留。本次启动禁止保存配置，登录和上传功能暂不可用。</p>
            <pre>{{ startupError }}</pre>
            <p>
                请关闭应用，检查配置文件是否被其他程序占用、目录权限或安全软件拦截，然后重新启动。
            </p>
            <p>
                若配置文件损坏，请先复制原文件留存，再用同目录中的 config.json.bak 恢复 config.json
                后重启。
            </p>
        </div>
        <router-view v-else-if="startupChecked" />
    </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const startupError = ref<string | null>(null)
const startupChecked = ref(false)

onMounted(async () => {
    try {
        startupError.value = await invoke<string | null>('get_startup_error')
    } catch (error) {
        startupError.value = `无法确认配置状态：${error}`
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
</style>
