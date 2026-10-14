import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import App from './App.vue'
import router from './router'
import type { Component } from 'vue'
import { setLanguage } from './i18n'

const initLanguage = async () => {
    try {
        const config = await invoke<{ language?: string }>('load_config')
        setLanguage(config?.language)
    } catch {
        setLanguage('zh-CN')
    }
}

const mountApp = async () => {
    await initLanguage()

    const app = createApp(App)
    const pinia = createPinia()

    // 注册所有图标
    for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
        app.component(key, component as Component)
    }

    app.use(pinia)
    app.use(router)
    app.use(ElementPlus)

    app.mount('#app')
}

void mountApp()
