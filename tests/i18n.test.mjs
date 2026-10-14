import assert from 'node:assert/strict'
import { test } from 'node:test'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { build } from 'esbuild'

const root = fileURLToPath(new URL('..', import.meta.url))
const result = await build({
    stdin: {
        contents: [
            "export * from './src/i18n/index.ts'",
            "export { useUserConfigStore } from './src/stores/user_config.ts'",
            "export { useAuthStore } from './src/stores/auth.ts'",
            "export { computed } from 'vue'",
            "export { createPinia, setActivePinia } from 'pinia'"
        ].join('\n'),
        resolveDir: root,
        loader: 'ts'
    },
    bundle: true,
    format: 'esm',
    platform: 'node',
    write: false,
    plugins: [
        {
            name: 'mock-tauri',
            setup(builder) {
                builder.onResolve({ filter: /^@tauri-apps\/api\/core$/ }, () => ({
                    path: 'core',
                    namespace: 'mock-tauri'
                }))
                builder.onLoad({ filter: /.*/, namespace: 'mock-tauri' }, () => ({
                    contents: 'export const invoke = (...args) => globalThis.__i18nInvoke(...args)'
                }))
            }
        }
    ]
})
const {
    computed,
    createPinia,
    setActivePinia,
    useUserConfigStore,
    useAuthStore,
    setLanguage,
    t,
    useI18nState
} = await import(
    'data:text/javascript;base64,' + Buffer.from(result.outputFiles[0].text).toString('base64')
)

const initialConfig = () => ({
    max_curr: 2,
    auto_upload: true,
    auto_start: true,
    log_level: 'info',
    language: 'zh-CN',
    config: {
        1: {
            user: { name: '中文用户名', cookie: {} },
            limit: 3,
            watermark: 0,
            auto_edit: 0,
            templates: { 中文模板: { title: '中文投稿标题', desc: '原始简介', videos: [] } },
            template_order: ['中文模板'],
            template_updated_at: { 中文模板: 0 }
        }
    }
})
const freshStore = async (config = initialConfig()) => {
    setActivePinia(createPinia())
    globalThis.__i18nInvoke = async command => {
        assert.equal(command, 'load_config')
        return structuredClone(config)
    }
    const store = useUserConfigStore()
    await store.loadConfig()
    return store
}

test('界面文本和 Element Plus 语言随切换刷新', () => {
    setLanguage('zh-CN')
    const label = computed(() => t('视频标题'))
    const { elementLocale } = useI18nState()
    assert.equal(label.value, '视频标题')
    assert.equal(elementLocale.value.name, 'zh-cn')
    setLanguage('en-US')
    assert.equal(label.value, 'Video title')
    assert.equal(elementLocale.value.name, 'en')
    setLanguage('ja-JP')
    assert.equal(label.value, '動画タイトル')
    assert.equal(elementLocale.value.name, 'ja')
    setLanguage('zh-CN')
    assert.equal(label.value, '视频标题')
})

test('插值保留原始中文、占位符和替换符号', () => {
    setLanguage('en-US')
    const raw = 'B站返回：失败，{error} $& $1'
    assert.equal(t('globalConfig.save.failed', { error: raw }), 'Failed to save settings: ' + raw)
    assert.equal(t(raw), raw)
    assert.equal(t('不存在的词条'), '不存在的词条')
    setLanguage('ja-JP')
    assert.equal(t('globalConfig.save.failed', { error: raw }), '設定の保存に失敗: ' + raw)
    assert.equal(t(raw), raw)
})

test('旧配置及不支持的语言回退到中文', async () => {
    const old = initialConfig()
    delete old.language
    const store = await freshStore(old)
    assert.equal(store.configRoot.language, 'zh-CN')
    assert.equal(t('视频标题'), '视频标题')
    setLanguage('en-US')
    setLanguage('invalid')
    assert.equal(t('视频标题'), '视频标题')
})

for (const selectedLanguage of ['en-US', 'ja-JP']) {
    test('保存等待期间和失败后保持语言、配置和未保存内容 (' + selectedLanguage + ')', async () => {
        const store = await freshStore()
        store.configRoot.config[1].templates['中文模板'].title = '未保存的修改'
        const before = JSON.parse(JSON.stringify(store.configRoot))
        let rejectSave
        globalThis.__i18nInvoke = async (command, args) => {
            assert.equal(command, 'save_global_config')
            assert.equal(args.language, selectedLanguage)
            return new Promise((_, reject) => {
                rejectSave = reject
            })
        }
        const saving = store.updateGlobalConfig({ language: selectedLanguage, max_curr: 4 })
        assert.equal(t('视频标题'), '视频标题')
        assert.deepEqual(JSON.parse(JSON.stringify(store.configRoot)), before)
        rejectSave('磁盘写入失败')
        await assert.rejects(saving, /磁盘写入失败/)
        assert.equal(t('视频标题'), '视频标题')
        assert.deepEqual(JSON.parse(JSON.stringify(store.configRoot)), before)
    })
}

for (const selectedLanguage of ['en-US', 'ja-JP']) {
    test(
        '成功保存并重新加载语言，保留投稿内容和未保存模板状态 (' + selectedLanguage + ')',
        async () => {
            const persisted = initialConfig()
            const store = await freshStore(persisted)
            store.configRoot.config[1].templates['中文模板'].title = '未保存的修改'
            const calls = []
            globalThis.__i18nInvoke = async (command, args) => {
                calls.push(command)
                if (command === 'save_global_config') {
                    persisted.language = args.language
                    persisted.max_curr = args.maxCurr
                    return true
                }
                assert.equal(command, 'load_config')
                return structuredClone(persisted)
            }
            await store.updateGlobalConfig({ language: selectedLanguage, max_curr: 4 })
            assert.equal(
                t('视频标题'),
                selectedLanguage === 'ja-JP' ? '動画タイトル' : 'Video title'
            )
            assert.equal(store.configRoot.language, selectedLanguage)
            assert.equal(store.configRoot.config[1].templates['中文模板'].title, '未保存的修改')
            assert.equal(store.configBase.config[1].templates['中文模板'].title, '中文投稿标题')
            assert.deepEqual(calls, ['save_global_config', 'load_config'])
            const restarted = await freshStore(persisted)
            assert.equal(
                t('视频标题'),
                selectedLanguage === 'ja-JP' ? '動画タイトル' : 'Video title'
            )
            assert.equal(restarted.configRoot.language, selectedLanguage)
            assert.deepEqual(restarted.configRoot.config, persisted.config)
        }
    )
}

for (const selectedLanguage of ['en-US', 'ja-JP']) {
    test('多语言模式下 API 返回仍保持原样 (' + selectedLanguage + ')', async () => {
        setActivePinia(createPinia())
        setLanguage(selectedLanguage)
        const response = { success: false, message: 'B站原始返回：账号错误', status: 'error' }
        globalThis.__i18nInvoke = async command => {
            assert.equal(command, 'check_qr_login')
            return response
        }
        assert.deepEqual(await useAuthStore().checkQRLogin(), response)
    })
}

for (const selectedLanguage of ['en-US', 'ja-JP']) {
    test('词典保留插值参数，国家选项都有翻译 (' + selectedLanguage + ')', () => {
        const catalog = JSON.parse(
            readFileSync(resolve(root, 'src/i18n/' + selectedLanguage + '.json'), 'utf8')
        )
        const placeholders = value =>
            [...value.matchAll(/\{(\w+)\}/g)].map(match => match[1]).sort()
        const english = JSON.parse(readFileSync(resolve(root, 'src/i18n/en-US.json'), 'utf8'))
        for (const source of Object.keys(english)) {
            const target = catalog[source]
            assert.ok(target, source)
            assert.deepEqual(placeholders(target), placeholders(source), source)
        }
        const countries = readFileSync(resolve(root, 'src/utils/country_codes.ts'), 'utf8')
        for (const match of countries.matchAll(/name: '([^']+)'/g)) {
            assert.ok(catalog[match[1]], match[1])
        }
    })
}

test('日语词典覆盖全部界面文本及命名消息', () => {
    const japanese = JSON.parse(readFileSync(resolve(root, 'src/i18n/ja-JP.json'), 'utf8'))
    const source = readFileSync(resolve(root, 'src/i18n/index.ts'), 'utf8')
    const placeholders = value => [...value.matchAll(/\{(\w+)\}/g)].map(match => match[1]).sort()
    for (const match of source.matchAll(/'([a-zA-Z]\w*(?:\.\w+)+)':\s*'([^']*)'/g)) {
        assert.ok(japanese[match[1]], match[1])
        assert.deepEqual(placeholders(japanese[match[1]]), placeholders(match[2]), match[1])
    }
})

test('文档语言随日语切换更新', () => {
    const originalDocument = globalThis.document
    globalThis.document = { documentElement: { lang: '' } }
    try {
        setLanguage('ja-JP')
        assert.equal(globalThis.document.documentElement.lang, 'ja-JP')
        setLanguage('zh-CN')
        assert.equal(globalThis.document.documentElement.lang, 'zh-CN')
    } finally {
        if (originalDocument === undefined) delete globalThis.document
        else globalThis.document = originalDocument
    }
})
