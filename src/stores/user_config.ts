import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAuthStore } from './auth'

// 用户信息接口
interface User {
    uid: number
    username: string
    avatar: string
    expired: boolean
}

export interface DescV2Item {
    biz_id: string
    raw_text: string
    type: number
    sub_type: number
    sub_biz_id: string
}

// 模板配置接口
export interface TemplateConfig {
    copyright: number // 1: 自制, 2: 转载
    source: string
    tid: number // 分区ID
    cover: string // 封面URL
    title: string
    desc: string
    desc_v2?: DescV2Item[]
    dynamic: string // 粉丝动态
    subtitle: { open: number; lan: string }
    tag: string // 逗号分隔的标签
    videos: any[]
    dtime?: number // 定时发布时间, 10位时间戳
    open_subtitle: boolean
    interactive: number
    mission_id?: number
    topic_id?: number
    topic_name?: string
    season_id?: number
    section_id?: number
    dolby: number
    lossless_music: number
    no_reprint: number
    open_elec: number
    no_disturbance: number
    aid?: number
    up_selection_reply: number
    up_close_reply: number
    up_close_danmu: number
    atomic_int: number
    is_only_self: number
    watermark: number
    is_360: number
    staff?: Array<{ title: string; mid: number; is_del: number }>
    state?: number
    state_desc?: string
}

interface UserConfig {
    user: { name: string; cookie: any }
    line?: string
    proxy?: string
    limit: number
    watermark: number
    auto_edit: number
    templates: Record<string, TemplateConfig> // 模板名 -> 模板配置
    template_order: string[]
    template_updated_at: Record<string, number>
}

// 配置根接口
interface ConfigRoot {
    max_curr: number
    auto_upload: boolean
    auto_start: boolean
    log_level: string
    config: Record<number, UserConfig> // uid -> 用户配置
}

// 用户模板组合接口
interface UserWithTemplates {
    user: User
    templates: Array<{ name: string; config: TemplateConfig; updatedAt: number }>
    expanded: boolean // 是否展开
}

interface TemplateCommandResponse {
    success: boolean
    message: string
    template?: TemplateConfig
}

interface TemplateOrderCommandResponse {
    success: boolean
    message: string
    template_order: string[]
}

interface RenameTemplateCommandResponse {
    success: boolean
    message: string
    template_order: string[]
}

export const useUserConfigStore = defineStore('userConfig', () => {
    const templateNameCollator = new Intl.Collator(
        ['zh-Hans-u-co-pinyin', 'zh-CN-u-co-pinyin', 'zh-CN', 'en'],
        {
            numeric: true,
            sensitivity: 'base'
        }
    )

    const authStore = useAuthStore()
    const configRoot = ref<ConfigRoot | null>(null)
    const configBase = ref<ConfigRoot | null>(null)
    const loginUsers = computed(() => authStore.loginUsers)
    const loading = ref(false)
    const error = ref<string | null>(null)

    const getCurrentTimestamp = () => Math.floor(Date.now() / 1000)

    const ensureUserConfigTemplateMetadata = (userConfig: UserConfig) => {
        userConfig.template_order = Array.isArray(userConfig.template_order)
            ? userConfig.template_order.filter(name => typeof name === 'string')
            : []
        userConfig.template_updated_at = userConfig.template_updated_at || {}

        userConfig.template_order = [...new Set(userConfig.template_order)]

        userConfig.template_order = userConfig.template_order.filter(
            name => userConfig.templates[name]
        )

        for (const templateName of Object.keys(userConfig.templates)) {
            if (!userConfig.template_order.includes(templateName)) {
                userConfig.template_order.push(templateName)
            }

            if (typeof userConfig.template_updated_at[templateName] !== 'number') {
                userConfig.template_updated_at[templateName] = 0
            }
        }

        for (const templateName of Object.keys(userConfig.template_updated_at)) {
            if (!userConfig.templates[templateName]) {
                delete userConfig.template_updated_at[templateName]
            }
        }
    }

    const normalizeConfigRoot = (configData: ConfigRoot | null) => {
        if (!configData?.config) {
            return
        }

        for (const userConfig of Object.values(configData.config)) {
            ensureUserConfigTemplateMetadata(userConfig)
        }
    }

    const getOrderedTemplateEntries = (userConfig?: UserConfig | null) => {
        if (!userConfig?.templates) {
            return []
        }

        ensureUserConfigTemplateMetadata(userConfig)

        return userConfig.template_order
            .filter(name => userConfig.templates[name])
            .map(name => ({
                name,
                config: userConfig.templates[name],
                updatedAt: userConfig.template_updated_at[name] || 0
            }))
    }

    const updateTemplateOrderLocally = (userUid: number, templateOrder: string[]) => {
        const userConfig = configRoot.value?.config[userUid]
        if (!userConfig) {
            throw new Error('用户配置不存在')
        }

        ensureUserConfigTemplateMetadata(userConfig)
        userConfig.template_order = templateOrder.filter(name => userConfig.templates[name])
        ensureUserConfigTemplateMetadata(userConfig)
    }

    const persistTemplateOrder = async (userUid: number, templateOrder: string[]) => {
        updateTemplateOrderLocally(userUid, templateOrder)

        const userConfig = configRoot.value?.config[userUid]
        if (!userConfig) {
            throw new Error('用户配置不存在')
        }

        const response: TemplateOrderCommandResponse = await invoke('save_template_order', {
            uid: userUid,
            templateOrder: userConfig.template_order
        })

        if (!response?.success) {
            throw new Error(response?.message || '保存模板顺序失败')
        }

        userConfig.template_order = response.template_order
        ensureUserConfigTemplateMetadata(userConfig)
        return true
    }

    const userTemplates = computed(() => {
        if (!configRoot.value || loginUsers.value.length === 0) {
            return []
        }

        const result: UserWithTemplates[] = []

        for (const user of loginUsers.value) {
            const userConfig = configRoot.value.config[user.uid]
            const templates = getOrderedTemplateEntries(userConfig)

            result.push({
                user,
                templates,
                expanded: false // 这个状态需要单独管理
            })
        }

        return result
    })

    // 用户展开状态管理
    const userExpandedState = ref<Record<number, boolean>>({})

    const allUsers = computed(() => userTemplates.value.map(ut => ut.user))
    const totalTemplateCount = computed(() =>
        userTemplates.value.reduce((sum, ut) => sum + ut.templates.length, 0)
    )

    // 获取带有展开状态的用户模板列表（用于UI显示）
    const userTemplatesWithExpandedState = computed(() => {
        return userTemplates.value.map(ut => ({
            ...ut,
            expanded: userExpandedState.value[ut.user.uid] || false
        }))
    })

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
        topic_name: undefined,
        season_id: undefined,
        section_id: undefined,
        dolby: 0,
        lossless_music: 0,
        no_reprint: 0,
        open_elec: 0,
        no_disturbance: 0,
        aid: undefined,
        up_selection_reply: 0,
        up_close_reply: 0,
        up_close_danmu: 0,
        atomic_int: 0,
        is_only_self: 0,
        watermark: 0,
        is_360: -1,
        staff: undefined,
        state: undefined,
        state_desc: undefined
    })

    // 配置文件操作
    const loadConfig = async () => {
        loading.value = true
        error.value = null
        try {
            const configData: ConfigRoot = await invoke('load_config')
            normalizeConfigRoot(configData)
            configRoot.value = configData
            configBase.value = JSON.parse(JSON.stringify(configData))
            return configData
        } catch (err) {
            error.value = `加载配置失败: ${err}`
            console.error('加载配置失败:', err)
            throw err
        } finally {
            loading.value = false
        }
    }

    const loadBaseConfig = async () => {
        loading.value = true
        error.value = null
        try {
            const configData: ConfigRoot = await invoke('load_config')
            normalizeConfigRoot(configData)
            configBase.value = JSON.parse(JSON.stringify(configData))
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
            await loadBaseConfig()
            return true
        } catch (err) {
            error.value = `保存配置失败: ${err}`
            console.error('保存配置失败:', err)
            throw err
        } finally {
            loading.value = false
        }
    }

    // 确保用户模板数据可用（登录用户来源由 auth store 统一提供）
    const ensureUserTemplatesReady = async () => {
        // 确保配置已加载
        if (!configRoot.value) {
            await loadConfig()
        }

        return userTemplatesWithExpandedState.value
    }

    // 兼容旧调用，后续可移除
    const buildUserTemplates = async (_users: User[]) => ensureUserTemplatesReady()

    // 切换用户展开/收起状态
    const toggleUserExpanded = (userUid: number) => {
        userExpandedState.value[userUid] = !userExpandedState.value[userUid]
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
                max_curr: 1,
                auto_start: true,
                auto_upload: true,
                log_level: 'info',
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
        ensureUserConfigTemplateMetadata(userConfig)

        // 检查模板名是否已存在
        if (userConfig.templates[templateName]) {
            throw new Error('模板名已存在')
        }

        const to_add = templateConfig || createDefaultTemplate()
        to_add.watermark = userConfig.watermark // 使用用户配置中的水印设置
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
        userConfig.template_order.push(templateName)
        userConfig.template_updated_at[templateName] = getCurrentTimestamp()
        ensureUserConfigTemplateMetadata(userConfig)

        // 保存配置
        await saveConfig()

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
        ensureUserConfigTemplateMetadata(userConfig)

        // 删除模板
        delete userConfig.templates[templateName]
        userConfig.template_order = userConfig.template_order.filter(name => name !== templateName)
        delete userConfig.template_updated_at[templateName]
        ensureUserConfigTemplateMetadata(userConfig)

        const server_response: TemplateCommandResponse = await invoke('delete_user_template', {
            uid: userUid,
            templateName
        })

        if (!server_response || !server_response.success) {
            throw new Error('删除模板失败: ' + server_response.message)
        }

        // 保存配置
        await saveConfig()

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
        ensureUserConfigTemplateMetadata(userConfig)

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
        userConfig.template_updated_at[templateName] = getCurrentTimestamp()
        ensureUserConfigTemplateMetadata(userConfig)

        // 保存配置
        await saveConfig()

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

    const renameUserTemplate = async (userUid: number, oldName: string, newName: string) => {
        if (!configRoot.value) {
            throw new Error('配置未加载')
        }

        const userConfig = configRoot.value.config[userUid]
        if (!userConfig || !userConfig.templates[oldName]) {
            throw new Error('原模板不存在')
        }
        if (userConfig.templates[newName]) {
            throw new Error('该名称的模板已存在')
        }

        ensureUserConfigTemplateMetadata(userConfig)

        const originalTemplate = userConfig.templates[oldName]

        const response: RenameTemplateCommandResponse = await invoke('rename_user_template', {
            uid: userUid,
            oldName,
            newName
        })

        if (!response?.success) {
            throw new Error(response?.message || '模板重命名失败')
        }

        delete userConfig.templates[oldName]
        userConfig.templates[newName] = originalTemplate

        const oldUpdatedAt = userConfig.template_updated_at[oldName]
        delete userConfig.template_updated_at[oldName]
        userConfig.template_updated_at[newName] =
            typeof oldUpdatedAt === 'number' ? oldUpdatedAt : getCurrentTimestamp()

        userConfig.template_order = response.template_order
        ensureUserConfigTemplateMetadata(userConfig)

        await saveConfig()

        return true
    }

    const reorderUserTemplates = async (userUid: number, templateOrder: string[]) => {
        return persistTemplateOrder(userUid, templateOrder)
    }

    const sortUserTemplates = async (
        userUid: number,
        mode: 'name-asc' | 'name-desc' | 'recent-saved' | 'recent-saved-desc'
    ) => {
        if (!configRoot.value) {
            throw new Error('配置未加载')
        }

        const userConfig = configRoot.value.config[userUid]
        if (!userConfig) {
            throw new Error('用户配置不存在')
        }

        const templateEntries = getOrderedTemplateEntries(userConfig)
        const sortedNames = [...templateEntries]

        if (mode === 'recent-saved') {
            sortedNames.sort((left, right) => {
                if (right.updatedAt !== left.updatedAt) {
                    return right.updatedAt - left.updatedAt
                }
                return templateNameCollator.compare(left.name, right.name)
            })
        } else if (mode === 'recent-saved-desc') {
            sortedNames.sort((left, right) => {
                if (left.updatedAt !== right.updatedAt) {
                    return left.updatedAt - right.updatedAt
                }
                return templateNameCollator.compare(left.name, right.name)
            })
        } else {
            sortedNames.sort((left, right) => templateNameCollator.compare(left.name, right.name))
            if (mode === 'name-desc') {
                sortedNames.reverse()
            }
        }

        return persistTemplateOrder(
            userUid,
            sortedNames.map(template => template.name)
        )
    }

    // 更新用户基础配置
    const updateUserConfig = async (
        userUid: number,
        updates: Partial<Pick<UserConfig, 'line' | 'proxy' | 'limit' | 'watermark' | 'auto_edit'>>
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

        if ('watermark' in updates) {
            userConfig.watermark = updates.watermark!
        }

        if ('auto_edit' in updates) {
            userConfig.auto_edit = updates.auto_edit!
        }

        try {
            await invoke('save_user_config', {
                uid: userUid,
                line: userConfig.line,
                proxy: userConfig.proxy,
                limit: userConfig.limit,
                watermark: userConfig.watermark,
                autoEdit: userConfig.auto_edit
            })
            // 保存配置
            await saveConfig()
        } catch (err) {
            throw new Error('保存配置失败: ' + err)
        }

        return true
    }

    const updateGlobalConfig = async (
        updates: Partial<Pick<ConfigRoot, 'max_curr' | 'auto_upload' | 'auto_start' | 'log_level'>>
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
                autoUpload: configRoot.value.auto_upload,
                logLevel: configRoot.value.log_level
            })
            // 保存配置
            await saveConfig()
        } catch (err) {
            throw new Error('保存配置失败: ' + err)
        }

        return true
    }

    return {
        // 状态
        configRoot,
        configBase,
        userTemplates: userTemplatesWithExpandedState, // 导出带有展开状态的版本
        loading,
        error,

        // 计算属性
        allUsers,
        totalTemplateCount,

        // 方法
        loadConfig,
        saveConfig,
        ensureUserTemplatesReady,
        buildUserTemplates,
        toggleUserExpanded,
        getUserTemplates,
        getUserTemplate,
        addUserTemplate,
        removeUserTemplate,
        updateUserTemplate,
        duplicateUserTemplate,
        renameUserTemplate,
        reorderUserTemplates,
        sortUserTemplates,
        updateUserConfig,
        updateGlobalConfig,
        createDefaultTemplate
    }
})
