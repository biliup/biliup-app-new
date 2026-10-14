import english from './en-US.json'
import { computed, ref } from 'vue'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import en from 'element-plus/es/locale/lang/en'

export type AppLanguage = 'zh-CN' | 'en-US'

type TranslationMap = Record<string, string>

const SUPPORTED_LANGUAGES: AppLanguage[] = ['zh-CN', 'en-US']

const messages: Record<AppLanguage, TranslationMap> = {
    'zh-CN': {
        'common.cancel': '取消',
        'common.save': '保存',
        'common.confirm': '确认',
        'common.enabled': '开启',
        'common.disabled': '关闭',
        'app.recovery.title': '配置加载失败，已进入受限恢复模式',
        'app.recovery.p1': '原配置及备份已保留。本次启动禁止保存配置，登录和上传功能暂不可用。',
        'app.recovery.p2':
            '请关闭应用，检查配置文件是否被其他程序占用、目录权限或安全软件拦截，然后重新启动。',
        'app.recovery.p3':
            '若配置文件损坏，请先复制原文件留存，再用同目录中的 config.json.bak 恢复 config.json 后重启。',
        'app.recovery.checkError': '无法确认配置状态：{error}',
        'globalConfig.title': '全局设置',
        'globalConfig.section.global': '全局设置',
        'globalConfig.section.user': '用户配置',
        'globalConfig.language.label': '界面语言',
        'globalConfig.language.zh': '中文',
        'globalConfig.language.en': 'English',
        'globalConfig.language.tip': '点击保存后生效，下次启动会恢复所选语言',
        'globalConfig.maxCurr.label': '最大并发任务数',
        'globalConfig.maxCurr.tip1': '控制同时进行的上传任务数量，建议根据网络状况调整',
        'globalConfig.maxCurr.tip2': '已经开始的任务，即使暂停或失败同样占用一条并发数',
        'globalConfig.autoUpload.label': '自动添加到上传队列',
        'globalConfig.autoUpload.tip': '开启后，创建上传任务时会自动添加到上传队列',
        'globalConfig.autoStart.label': '自动开始任务',
        'globalConfig.autoStart.tip': '开启后，任务添加到队列后会自动开始上传',
        'globalConfig.logLevel.label': '日志级别',
        'globalConfig.logLevel.placeholder': '请选择日志级别',
        'globalConfig.logLevel.tip': '该配置程序重启后生效',
        'globalConfig.userSelect.label': '选择用户',
        'globalConfig.userSelect.placeholder': '请选择要配置的用户',
        'globalConfig.userSelect.tip': '选择要修改配置的用户',
        'globalConfig.line.label': '上传线路',
        'globalConfig.line.placeholder': '请选择上传线路',
        'globalConfig.line.auto': '自动选择',
        'globalConfig.line.tip': '自动选择将根据网络环境自动选择最优线路',
        'globalConfig.proxy.label': '代理设置',
        'globalConfig.proxy.enabled': '启用代理',
        'globalConfig.proxy.type': '代理类型',
        'globalConfig.proxy.type.placeholder': '选择代理类型',
        'globalConfig.proxy.host': '服务器地址',
        'globalConfig.proxy.host.placeholder': '127.0.0.1',
        'globalConfig.proxy.port': '端口',
        'globalConfig.proxy.username': '用户名',
        'globalConfig.proxy.username.placeholder': '用户名（可选）',
        'globalConfig.proxy.password': '密码',
        'globalConfig.proxy.password.placeholder': '密码（可选）',
        'globalConfig.proxy.tip': '配置代理服务器用于网络访问',
        'globalConfig.limit.label': '单视频并发数',
        'globalConfig.watermark.label': '默认开启水印',
        'globalConfig.autoEdit.label': '上传完成自动编辑',
        'globalConfig.switch.on': '开启',
        'globalConfig.switch.off': '关闭',
        'globalConfig.save.success': '配置保存成功',
        'globalConfig.save.failed': '保存配置失败: {error}',
        'globalConfig.load.failed': '加载全局配置失败: {error}',
        'globalConfig.userLoad.failed': '加载用户配置失败: {error}',
        'globalConfig.unsaved.message': '有未保存的更改，确定要关闭吗？',
        'main.exportLogs': '导出日志',
        'main.checkUpdate': '检查更新',
        'main.submitStats': '提交统计',
        'main.globalConfig': '全局设置',
        'main.drag.title': '拖拽视频文件到此处',
        'main.drag.formats': '支持 MP4、AVI、MOV、MKV、WMV、FLV、M4V、WEBM 格式',
        'main.drag.needSelection': '请先选择用户和模板'
    },
    'en-US': {
        'common.cancel': 'Cancel',
        'common.save': 'Save',
        'common.confirm': 'Confirm',
        'common.enabled': 'On',
        'common.disabled': 'Off',
        'app.recovery.title': 'Configuration failed to load, running in recovery mode',
        'app.recovery.p1':
            'Original config and backup are preserved. Saving, login, and upload are disabled in this session.',
        'app.recovery.p2':
            'Close the app, then check file locks, directory permissions, or security software and restart.',
        'app.recovery.p3':
            'If config is corrupted, back up the original file first, then restore config.json from config.json.bak and restart.',
        'app.recovery.checkError': 'Unable to check config state: {error}',
        'globalConfig.title': 'Global Settings',
        'globalConfig.section.global': 'Global Settings',
        'globalConfig.section.user': 'User Settings',
        'globalConfig.language.label': 'Language',
        'globalConfig.language.zh': '中文',
        'globalConfig.language.en': 'English',
        'globalConfig.language.tip': 'Applies after saving; your choice is restored on next launch',
        'globalConfig.maxCurr.label': 'Max Concurrent Tasks',
        'globalConfig.maxCurr.tip1': 'Controls how many upload tasks run at the same time',
        'globalConfig.maxCurr.tip2': 'Started tasks still occupy a slot even when paused or failed',
        'globalConfig.autoUpload.label': 'Auto Add to Queue',
        'globalConfig.autoUpload.tip': 'Automatically add created tasks to the upload queue',
        'globalConfig.autoStart.label': 'Auto Start Tasks',
        'globalConfig.autoStart.tip': 'Automatically start tasks after they are added to the queue',
        'globalConfig.logLevel.label': 'Log Level',
        'globalConfig.logLevel.placeholder': 'Select a log level',
        'globalConfig.logLevel.tip': 'Takes effect after app restart',
        'globalConfig.userSelect.label': 'Select User',
        'globalConfig.userSelect.placeholder': 'Select a user to configure',
        'globalConfig.userSelect.tip': 'Choose which user to update',
        'globalConfig.line.label': 'Upload Line',
        'globalConfig.line.placeholder': 'Select upload line',
        'globalConfig.line.auto': 'Auto',
        'globalConfig.line.tip': 'Auto mode chooses the best line based on your network',
        'globalConfig.proxy.label': 'Proxy',
        'globalConfig.proxy.enabled': 'Enable Proxy',
        'globalConfig.proxy.type': 'Proxy Type',
        'globalConfig.proxy.type.placeholder': 'Select proxy type',
        'globalConfig.proxy.host': 'Server Address',
        'globalConfig.proxy.host.placeholder': '127.0.0.1',
        'globalConfig.proxy.port': 'Port',
        'globalConfig.proxy.username': 'Username',
        'globalConfig.proxy.username.placeholder': 'Username (optional)',
        'globalConfig.proxy.password': 'Password',
        'globalConfig.proxy.password.placeholder': 'Password (optional)',
        'globalConfig.proxy.tip': 'Configure proxy server for network access',
        'globalConfig.limit.label': 'Per-Video Concurrency',
        'globalConfig.watermark.label': 'Enable Watermark by Default',
        'globalConfig.autoEdit.label': 'Auto Edit After Upload',
        'globalConfig.switch.on': 'On',
        'globalConfig.switch.off': 'Off',
        'globalConfig.save.success': 'Settings saved successfully',
        'globalConfig.save.failed': 'Failed to save settings: {error}',
        'globalConfig.load.failed': 'Failed to load global settings: {error}',
        'globalConfig.userLoad.failed': 'Failed to load user settings: {error}',
        'globalConfig.unsaved.message': 'You have unsaved changes. Close anyway?',
        'main.exportLogs': 'Export Logs',
        'main.checkUpdate': 'Check Updates',
        'main.submitStats': 'Submit Stats',
        'main.globalConfig': 'Global Settings',
        'main.drag.title': 'Drop video files here',
        'main.drag.formats': 'Supports MP4, AVI, MOV, MKV, WMV, FLV, M4V, WEBM',
        'main.drag.needSelection': 'Please select user and template first'
    }
}

const language = ref<AppLanguage>('zh-CN')

export const normalizeLanguage = (value?: string | null): AppLanguage => {
    if (value && SUPPORTED_LANGUAGES.includes(value as AppLanguage)) {
        return value as AppLanguage
    }
    return 'zh-CN'
}

export const setLanguage = (value?: string | null) => {
    language.value = normalizeLanguage(value)
}

// 只翻译明确标记的应用文本，插值参数保持原样。
export const t = (key: string, params?: Record<string, string | number>) => {
    const source =
        language.value === 'en-US'
            ? ((english as TranslationMap)[key] ??
              messages['en-US'][key] ??
              messages['zh-CN'][key] ??
              key)
            : (messages['zh-CN'][key] ?? key)
    return source.replace(/\{(\w+)\}/g, (placeholder, name) =>
        params && Object.prototype.hasOwnProperty.call(params, name)
            ? String(params[name])
            : placeholder
    )
}

export const useI18nState = () => ({
    language,
    elementLocale: computed(() => (language.value === 'en-US' ? en : zhCn)),
    t
})
