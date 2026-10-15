import assert from 'node:assert/strict'
import { test } from 'node:test'
import { build } from 'esbuild'
import { fileURLToPath } from 'node:url'

const bundle = await build({
    stdin: {
        contents: [
            "export { useUserConfigStore } from './src/stores/user_config.ts'",
            "export { useAuthStore } from './src/stores/auth.ts'",
            "export { createPinia, setActivePinia } from 'pinia'"
        ].join('\n'),
        resolveDir: fileURLToPath(new URL('..', import.meta.url)),
        loader: 'ts'
    },
    bundle: true,
    format: 'esm',
    platform: 'node',
    write: false,
    plugins: [
        {
            name: 'mock-config-ipc',
            setup(builder) {
                builder.onResolve({ filter: /^@tauri-apps\/api\/core$/ }, () => ({
                    path: 'core',
                    namespace: 'mock-config'
                }))
                builder.onLoad({ filter: /.*/, namespace: 'mock-config' }, () => ({
                    contents:
                        'export const invoke = (...args) => globalThis.__configInvoke(...args)'
                }))
            }
        }
    ]
})
const { useUserConfigStore, useAuthStore, createPinia, setActivePinia } = await import(
    'data:text/javascript;base64,' + Buffer.from(bundle.outputFiles[0].text).toString('base64')
)

async function freshStore(settings = {}) {
    setActivePinia(createPinia())
    useAuthStore().loginUsers = [{ uid: 1, username: '测试用户' }]
    const persisted = {
        max_curr: 2,
        auto_upload: true,
        auto_start: true,
        log_level: 'info',
        language: 'zh-CN',
        config: {
            1: {
                user: { name: '测试用户', cookie: {} },
                limit: 3,
                watermark: 0,
                auto_edit: 0,
                templates: {},
                template_order: [],
                template_updated_at: {},
                ...settings
            }
        }
    }
    const submitted = []
    globalThis.__configInvoke = async (command, args) => {
        if (command === 'load_config') return structuredClone(persisted)
        if (command === 'add_user_template') {
            const template = structuredClone(args.template)
            submitted.push(template)
            persisted.config[args.uid].templates[args.templateName] = template
            return structuredClone(template)
        }
        assert.equal(command, 'save_config')
    }
    const store = useUserConfigStore()
    await store.loadConfig()
    return { store, submitted, persisted }
}

test('导入编辑模板保留远端水印，包括关闭值', async () => {
    for (const watermark of [0, 1]) {
        const { store, submitted, persisted } = await freshStore({ watermark: 1 - watermark })
        await store.addUserTemplate(1, '编辑模板', {
            ...store.createDefaultTemplate(),
            aid: 123,
            watermark
        })
        assert.equal(submitted[0].watermark, watermark)
        assert.equal(store.configRoot.config[1].templates['编辑模板'].watermark, watermark)
        assert.equal(persisted.config[1].templates['编辑模板'].watermark, watermark)
    }
})

test('空白和复制稿件的新稿模板使用账号默认水印', async () => {
    for (const watermark of [0, 1]) {
        const { store, submitted } = await freshStore({ watermark })
        await store.addUserTemplate(1, '空白模板')
        await store.addUserTemplate(1, '新稿模板', {
            ...store.createDefaultTemplate(),
            watermark: 1 - watermark
        })
        assert.deepEqual(
            submitted.map(template => template.watermark),
            [watermark, watermark]
        )
    }
})
