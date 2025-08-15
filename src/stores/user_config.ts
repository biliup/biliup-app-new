import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// 用户信息接口
interface User {
    uid: number
    username: string
    avatar: string
}

// 模板配置接口
interface TemplateConfig {
    copyright: number // 1: 自制, 2: 转载
    source: string
    tid: number // 分区ID
    cover: string // 封面URL
    title: string
    desc: string
    desc_v2?: string
    dynamic: string // 粉丝动态
    subtitle: { open: number; lan: string }
    tag: string // 逗号分隔的标签
    videos: Array<{ title: string; name: string; desc: string; path: string }>
    dtime?: number // 定时发布时间, 10位时间戳
    open_subtitle: boolean
    interactive: number
    mission_id?: number
    topic_id?: number
    season_id?: number
    section_id?: number
    dolby: number
    lossless_music: number
    no_reprint: number
    open_elec: number
    aid?: number
    up_selection_reply: number
    up_close_reply: number
    up_close_danmu: number
    atomic_int: number
    is_only_self: number
}

interface UserConfig {
    user: { name: string; cookie: any }
    line?: string
    proxy?: string
    limit: number
    templates: Record<string, TemplateConfig> // 模板名 -> 模板配置
}

// 配置根接口
interface ConfigRoot {
    max_curr: number
    auto_upload: boolean
    auto_start: boolean
    config: Record<number, UserConfig> // uid -> 用户配置
}

// 用户模板组合接口
interface UserWithTemplates {
    user: User
    templates: Array<{ name: string; config: TemplateConfig }>
    expanded: boolean // 是否展开
}

interface TemplateCommandResponse {
    success: boolean
    message: string
    template?: TemplateConfig
}

// 接口定义
export interface uploadForm {
    title: string
    cover: string
    copyright: number
    source: string
    aid?: number
    tid: number
    tag: string
    desc: string
    dynamic: string
    videos: any[]
    dtime?: number
    open_subtitle: boolean
    interactive: number
    mission_id?: number
    topic_id?: number
    season_id?: number
    section_id?: number
    dolby: number
    lossless_music: number
    no_reprint: number
    open_elec: number
    up_selection_reply: number
    up_close_reply: number
    up_close_danmu: number
    is_only_self: number
}

export const useUserConfigStore = defineStore('userConfig', () => {
    const configRoot = ref<ConfigRoot | null>(null)
    const userTemplates = ref<UserWithTemplates[]>([])
    const loading = ref(false)
    const error = ref<string | null>(null)

    // 计算属性
    const allUsers = computed(() => userTemplates.value.map(ut => ut.user))
    const totalTemplateCount = computed(() =>
        userTemplates.value.reduce((sum, ut) => sum + ut.templates.length, 0)
    )

    // 默认模板配置
    const createDefaultTemplate = (): TemplateConfig => ({
        copyright: 1,
        source: '',
        tid: 0,
        cover: '',
        title: '',
        desc: '',
        desc_v2: undefined,
        dynamic: '',
        subtitle: { open: 0, lan: '' },
        tag: '',
        videos: [],
        dtime: undefined,
        open_subtitle: false,
        interactive: 0,
        mission_id: undefined,
        topic_id: undefined,
        season_id: undefined,
        section_id: undefined,
        dolby: 0,
        lossless_music: 0,
        no_reprint: 0,
        open_elec: 0,
        aid: undefined,
        up_selection_reply: 0,
        up_close_reply: 0,
        up_close_danmu: 0,
        atomic_int: 0,
        is_only_self: 0
    })

    // 配置文件操作
    const loadConfig = async () => {
        loading.value = true
        error.value = null
        try {
            const configData: ConfigRoot = await invoke('load_config')
            configRoot.value = configData
            return configData
        } catch (err) {
            error.value = `加载配置失败: ${err}`
            console.error('加载配置失败:', err)
            throw err
        } finally {
            loading.value = false
        }
    }

    const saveConfig = async () => {
        loading.value = true
        error.value = null
        try {
            await invoke('save_config', {})
            return true
        } catch (err) {
            error.value = `保存配置失败: ${err}`
            console.error('保存配置失败:', err)
            throw err
        } finally {
            loading.value = false
        }
    }

    // 构建用户模板列表
    const buildUserTemplates = async (loginUsers: User[]) => {
        // 确保配置已加载
        if (!configRoot.value) {
            await loadConfig()
        }

        const newUserTemplates: UserWithTemplates[] = []

        // 为每个登录用户构建模板列表
        for (const user of loginUsers) {
            // 查找对应的用户配置
            const userConfig = configRoot.value?.config[user.uid]

            const templates =
                userConfig && userConfig.templates
                    ? Object.entries(userConfig.templates).map(([name, config]) => ({
                          name,
                          config
                      }))
                    : []

            newUserTemplates.push({
                user,
                templates,
                expanded: false
            })
        }

        userTemplates.value = newUserTemplates
        return newUserTemplates
    }

    // 切换用户展开/收起状态
    const toggleUserExpanded = (userUid: number) => {
        const userTemplate = userTemplates.value.find(ut => ut.user.uid === userUid)
        if (userTemplate) {
            userTemplate.expanded = !userTemplate.expanded
        }
    }

    // 获取指定用户的模板
    const getUserTemplates = (userUid: number) => {
        const userTemplate = userTemplates.value.find(ut => ut.user.uid === userUid)
        return userTemplate?.templates || []
    }

    // 获取指定用户的指定模板
    const getUserTemplate = (userUid: number, templateName: string) => {
        const userTemplate = userTemplates.value.find(ut => ut.user.uid === userUid)
        return userTemplate?.templates.find(t => t.name === templateName)?.config
    }

    // 为指定用户添加模板
    const addUserTemplate = async (
        userUid: number,
        templateName: string,
        templateConfig?: TemplateConfig
    ) => {
        // 确保配置已加载
        if (!configRoot.value) {
            await loadConfig()
        }

        if (!configRoot.value) {
            configRoot.value = {
                max_curr: 2,
                auto_start: true,
                auto_upload: true,
                config: {}
            }
        }

        // 找到对应的用户
        const user = userTemplates.value.find(ut => ut.user.uid === userUid)?.user
        if (!user) {
            throw new Error('用户不存在')
        }

        // 查找或创建用户配置
        let userConfig = configRoot.value.config[userUid]
        if (!userConfig) {
            throw new Error('用户配置不存在')
        }

        // 检查模板名是否已存在
        if (userConfig.templates[templateName]) {
            throw new Error('模板名已存在')
        }

        const to_add = templateConfig || createDefaultTemplate()
        const server_response: TemplateCommandResponse = await invoke('add_user_template', {
            uid: userUid,
            templateName,
            template: to_add
        })

        if (!server_response || !server_response.success || !server_response.template) {
            throw new Error('添加模板失败: ' + server_response.message)
        }

        // 添加模板
        userConfig.templates[templateName] = server_response.template

        // 保存配置
        await saveConfig()

        // 更新本地状态
        const userTemplate = userTemplates.value.find(ut => ut.user.uid === userUid)
        if (userTemplate) {
            userTemplate.templates.push({
                name: templateName,
                config: userConfig.templates[templateName]
            })
        }

        return true
    }

    // 删除指定用户的模板
    const removeUserTemplate = async (userUid: number, templateName: string) => {
        if (!configRoot.value) {
            throw new Error('配置未加载')
        }

        const user = userTemplates.value.find(ut => ut.user.uid === userUid)?.user
        if (!user) {
            throw new Error('用户不存在')
        }

        const userConfig = configRoot.value.config[userUid]
        if (!userConfig || !userConfig.templates[templateName]) {
            throw new Error('模板不存在')
        }

        // 删除模板
        delete userConfig.templates[templateName]

        const server_response: TemplateCommandResponse = await invoke('delete_user_template', {
            uid: userUid,
            templateName
        })

        if (!server_response || !server_response.success) {
            throw new Error('删除模板失败: ' + server_response.message)
        }

        // 保存配置
        await saveConfig()

        // 更新本地状态
        const userTemplate = userTemplates.value.find(ut => ut.user.uid === userUid)
        if (userTemplate) {
            const templateIndex = userTemplate.templates.findIndex(t => t.name === templateName)
            if (templateIndex !== -1) {
                userTemplate.templates.splice(templateIndex, 1)
            }
        }

        return true
    }

    // 更新指定用户的模板
    const updateUserTemplate = async (
        userUid: number,
        templateName: string,
        templateConfig: TemplateConfig
    ) => {
        if (!configRoot.value) {
            throw new Error('配置未加载')
        }

        const user = userTemplates.value.find(ut => ut.user.uid === userUid)?.user
        if (!user) {
            throw new Error('用户不存在')
        }

        const userConfig = configRoot.value.config[userUid]
        if (!userConfig || !userConfig.templates[templateName]) {
            throw new Error('模板不存在')
        }

        const server_response: TemplateCommandResponse = await invoke('update_user_template', {
            uid: userUid,
            templateName,
            template: templateConfig
        })

        if (!server_response || !server_response.success || !server_response.template) {
            throw new Error('更新模板失败: ' + server_response.message)
        }

        // 更新模板
        userConfig.templates[templateName] = server_response.template

        // 保存配置
        await saveConfig()

        // 更新本地状态
        const userTemplate = userTemplates.value.find(ut => ut.user.uid === userUid)
        if (userTemplate) {
            const template = userTemplate.templates.find(t => t.name === templateName)
            if (template) {
                template.config = userConfig.templates[templateName]
            }
        }

        return true
    }

    // 复制模板
    const duplicateUserTemplate = async (
        userUid: number,
        templateName: string,
        newTemplateName: string
    ) => {
        const templateConfig = getUserTemplate(userUid, templateName)
        if (!templateConfig) {
            throw new Error('源模板不存在')
        }

        return await addUserTemplate(userUid, newTemplateName, {
            ...templateConfig,
            aid: undefined // 复制时清除稿件ID
        })
    }

    // 更新用户基础配置
    const updateUserConfig = async (
        userUid: number,
        updates: Partial<Pick<UserConfig, 'line' | 'proxy' | 'limit'>>
    ) => {
        if (!configRoot.value) {
            throw new Error('配置未加载')
        }

        const userConfig = configRoot.value.config[userUid]
        if (!userConfig) {
            throw new Error('用户配置不存在')
        }

        // 更新配置
        if ('line' in updates) {
            userConfig.line = updates.line
        }
        if ('proxy' in updates) {
            userConfig.proxy = updates.proxy
        }
        if ('limit' in updates) {
            userConfig.limit = updates.limit!
        }

        try {
            await invoke('save_user_config', {
                uid: userUid,
                line: userConfig.line,
                proxy: userConfig.proxy,
                limit: userConfig.limit
            })
            // 保存配置
            await saveConfig()
        } catch (err) {
            throw new Error('保存配置失败: ' + err)
        }

        return true
    }

    const updateGlobalConfig = async (
        updates: Partial<Pick<ConfigRoot, 'max_curr' | 'auto_upload' | 'auto_start'>>
    ) => {
        if (!configRoot.value) {
            throw new Error('配置未加载')
        }

        // 更新全局配置
        Object.assign(configRoot.value, updates)

        try {
            await invoke('save_global_config', {
                maxCurr: configRoot.value.max_curr,
                autoStart: configRoot.value.auto_start,
                autoUpload: configRoot.value.auto_upload
            })
            // 保存配置
            await saveConfig()
        } catch (err) {
            throw new Error('保存配置失败: ' + err)
        }

        return true
    }

    const buildTemplateFromUploadForm = (form: uploadForm) => {
        return {
            ...createDefaultTemplate(),
            title: form.title,
            cover: form.cover,
            copyright: form.copyright,
            source: form.source,
            aid: form.aid,
            tid: form.tid,
            desc: form.desc,
            dynamic: form.dynamic,
            tag: form.tag,
            videos: form.videos,
            dtime: form.dtime || undefined,
            open_subtitle: Boolean(form.open_subtitle),
            interactive: form.interactive,
            mission_id: form.mission_id,
            topic_id: form.topic_id,
            season_id: form.season_id,
            section_id: form.section_id,
            dolby: form.dolby,
            lossless_music: form.lossless_music,
            no_reprint: form.no_reprint,
            open_elec: form.open_elec,
            up_selection_reply: form.up_selection_reply,
            up_close_reply: form.up_close_reply,
            up_close_danmu: form.up_close_danmu,
            is_only_self: form.is_only_self
        }
    }

    return {
        // 状态
        configRoot,
        userTemplates,
        loading,
        error,

        // 计算属性
        allUsers,
        totalTemplateCount,

        // 方法
        loadConfig,
        saveConfig,
        buildUserTemplates,
        toggleUserExpanded,
        getUserTemplates,
        getUserTemplate,
        addUserTemplate,
        removeUserTemplate,
        updateUserTemplate,
        duplicateUserTemplate,
        updateUserConfig,
        updateGlobalConfig,
        createDefaultTemplate,
        buildTemplateFromUploadForm
    }
})
