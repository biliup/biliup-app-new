import assert from 'node:assert/strict'
import { test } from 'node:test'
import { build } from 'esbuild'
import { fileURLToPath } from 'node:url'

const bundle = await build({
    stdin: {
        contents: [
            "export { useUserConfigStore } from './src/stores/user_config.ts'",
            "export { useUploadStore } from './src/stores/upload.ts'",
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
const { useUserConfigStore, useUploadStore, useAuthStore, createPinia, setActivePinia } = await import(
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
            const template = JSON.parse(JSON.stringify(args.template))
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

test('新稿模板的提交后自动编辑选项继承账号设置', async () => {
    for (const autoEdit of [0, 1]) {
        const { store, submitted, persisted } = await freshStore({ auto_edit: autoEdit })
        await store.addUserTemplate(1, '空白模板')
        await store.addUserTemplate(1, '复制稿件模板', {
            ...store.createDefaultTemplate(),
            auto_edit_after_submit: 1 - autoEdit
        })
        store.configRoot.config[1].templates['空白模板'].auto_edit_after_submit = 1 - autoEdit
        await store.duplicateUserTemplate(1, '空白模板', '复制模板')
        assert.deepEqual(
            submitted.map(template => template.auto_edit_after_submit),
            [autoEdit, autoEdit, autoEdit]
        )
        for (const name of ['复制稿件模板', '复制模板']) {
            assert.equal(
                store.configRoot.config[1].templates[name].auto_edit_after_submit,
                autoEdit
            )
            assert.equal(persisted.config[1].templates[name].auto_edit_after_submit, autoEdit)
        }
        // 账号设置仅作为创建时的初始值，后续更改不覆盖已有模板的选择。
        store.configRoot.config[1].auto_edit = 1 - autoEdit
        assert.equal(
            store.configRoot.config[1].templates['复制模板'].auto_edit_after_submit,
            autoEdit
        )
    }
})

test('导入编辑模板保留自身的提交后自动编辑设置', async () => {
    for (const autoEdit of [0, 1]) {
        const { store, submitted } = await freshStore({ auto_edit: 1 - autoEdit })
        await store.addUserTemplate(1, '编辑模板', {
            ...store.createDefaultTemplate(),
            aid: 123,
            auto_edit_after_submit: autoEdit
        })
        assert.equal(submitted[0].auto_edit_after_submit, autoEdit)
    }
})

const pendingStorageKey = 'pending-video-season-sync'
async function seasonHarness(templates = {}) {
    const values = new Map()
    globalThis.localStorage = {
        getItem: key => values.get(key) || null,
        setItem: (key, value) => values.set(key, value)
    }
    const { store, persisted } = await freshStore({ templates })
    const calls = []
    let query = async () => ({ season_id: 0, section_id: 0 })
    globalThis.__configInvoke = async (command, args) => {
        calls.push([command, args])
        if (command === 'get_video_season') return query(args)
        if (command === 'sync_template_season') {
            for (const template of Object.values(persisted.config[args.uid]?.templates || {})) {
                if (args.aids.includes(template.aid)) {
                    template.season_id = args.seasonId
                    template.section_id = args.sectionId
                }
            }
            return
        }
        if (command === 'load_config') return structuredClone(persisted)
        if (command === 'submit') return { aid: args.form.aid }
        throw new Error(command)
    }
    return { store, persisted, calls, values, setQuery: fn => { query = fn } }
}

test('移出响应丢失后核对真实归属，同时同步所有对应模板并保留本地编辑', async () => {
    const h = await seasonHarness({
        first: { aid: 123, title: '未保存标题', season_id: 10, section_id: 11, videos: [{ path: '/draft' }] },
        second: { aid: 123, season_id: 10 },
        other: { aid: 999, season_id: 20 }
    })
    h.store.configRoot.config[2] = { templates: { same: { aid: 123, season_id: 30 } } }
    let writes = 0
    await assert.rejects(h.store.changeVideoSeason(1, [123], { season_id: 0, section_id: 0 }, async () => {
        writes++
        throw new Error('响应丢失')
    }), /响应丢失/)
    const templates = h.store.configRoot.config[1].templates
    assert.equal(templates.first.season_id, 0)
    assert.equal(templates.second.season_id, 0)
    assert.equal(templates.first.title, '未保存标题')
    assert.equal(templates.first.videos[0].path, '/draft')
    assert.equal(templates.other.season_id, 20)
    assert.equal(h.store.configRoot.config[2].templates.same.season_id, 30)
    assert.equal(h.persisted.config[1].templates.first.season_id, 0)
    assert.equal(writes, 1)
    assert.deepEqual(JSON.parse(h.values.get(pendingStorageKey)), [])
    h.store.assertVideoSeasonReady(1, 123)
})

test('添加响应丢失按实际归属同步，操作未执行时保留原归属，批量部分失败独立处理', async () => {
    const h = await seasonHarness({ first: { aid: 123 }, second: { aid: 124, season_id: 30, section_id: 31 } })
    h.setQuery(async ({ aid }) => aid === 123 ? { season_id: 10, section_id: 11 } : { season_id: 30, section_id: 31 })
    await assert.rejects(h.store.changeVideoSeason(1, [123, 124], { season_id: 10, section_id: 11 }, async () => {
        throw new Error('添加响应丢失')
    }), /添加响应丢失/)
    assert.equal(h.store.configRoot.config[1].templates.first.season_id, 10)
    assert.equal(h.store.configRoot.config[1].templates.second.season_id, 30)
    assert.equal(h.calls.filter(([command]) => command === 'get_video_season').length, 2)
})

test('核对失败持久化待确认状态，重建 store 后仍拦截投稿，重试成功恢复并更新快照', async () => {
    const h = await seasonHarness({ first: { aid: 123, season_id: 10, section_id: 11 } })
    h.setQuery(async () => { throw new Error('查询失败') })
    await assert.rejects(h.store.changeVideoSeason(1, [123], { season_id: 0, section_id: 0 }, async () => {
        throw new Error('响应丢失')
    }), /归属核对失败/)
    assert.throws(() => h.store.assertVideoSeasonReady(1, 123), /待确认/)
    h.store.assertVideoSeasonReady(2, 123)
    assert.deepEqual(JSON.parse(h.values.get(pendingStorageKey)), ['1:123'])
    setActivePinia(createPinia())
    const restored = useUserConfigStore()
    await restored.loadConfig()
    assert.throws(() => restored.assertVideoSeasonReady(1, 123), /待确认/)
    const upload = useUploadStore()
    await assert.rejects(upload.submitTemplate(1, { aid: 123, season_id: 10 }), /待确认/)
    assert.equal(h.calls.some(([command]) => command === 'submit'), false)
    h.setQuery(async () => ({ season_id: 0, section_id: 0 }))
    const snapshot = { aid: 123, season_id: 10, section_id: 11 }
    await restored.ensureVideoSeason(1, snapshot)
    assert.deepEqual(snapshot, { aid: 123, season_id: 0, section_id: 0 })
    restored.assertVideoSeasonReady(1, 123)
    assert.deepEqual(JSON.parse(h.values.get(pendingStorageKey)), [])
})

test('合集操作执行中阻止投稿，完成后拒绝操作前的排队快照，正常成功无需额外查询', async () => {
    const h = await seasonHarness({ first: { aid: 123, season_id: 10 } })
    let finish
    const revision = h.store.getSeasonRevision(1, 123)
    const pending = h.store.changeVideoSeason(1, [123], { season_id: 0, section_id: 0 }, () => new Promise(resolve => { finish = resolve }))
    await assert.rejects(h.store.ensureVideoSeason(1, { aid: 123 }), /操作进行中/)
    assert.throws(() => h.store.assertVideoSeasonReady(1, 123), /待确认/)
    finish()
    await pending
    h.store.assertVideoSeasonReady(1, 123)
    assert.throws(() => h.store.assertVideoSeasonReady(1, 123, revision), /已变化/)
    assert.equal(h.calls.some(([command]) => command === 'get_video_season'), false)
})

test('刷新重试只解除核对成功稿件的保护，失败稿件和其他账号仍受保护', async () => {
    const h = await seasonHarness({ first: { aid: 123 }, second: { aid: 124 } })
    h.setQuery(async () => { throw new Error('离线') })
    for (const [uid, aids] of [[1, [123, 124]], [2, [123]]]) {
        await assert.rejects(h.store.changeVideoSeason(uid, aids, { season_id: 10, section_id: 11 }, async () => { throw new Error('离线') }))
    }
    h.setQuery(async ({ aid }) => {
        if (aid === 124) throw new Error('仍离线')
        return { season_id: 10, section_id: 11 }
    })
    await assert.rejects(h.store.retryPendingVideoSeasons(1), /归属核对失败/)
    h.store.assertVideoSeasonReady(1, 123)
    assert.throws(() => h.store.assertVideoSeasonReady(1, 124), /待确认/)
    assert.throws(() => h.store.assertVideoSeasonReady(2, 123), /待确认/)
})


test('投稿队列拒绝合集变更前的旧快照，后续重新提交可使用新归属', async () => {
    const h = await seasonHarness({ first: { aid: 123, season_id: 10 } })
    const invoke = globalThis.__configInvoke
    let finish
    globalThis.__configInvoke = async (command, args) => {
        if (command === 'submit' && !args.form.aid)
            return new Promise(resolve => { finish = resolve })
        return invoke(command, args)
    }
    const upload = useUploadStore()
    const first = upload.submitTemplate(1, {})
    const stale = assert.rejects(upload.submitTemplate(1, { aid: 123, season_id: 10 }), /已变化/)
    await h.store.changeVideoSeason(1, [123], { season_id: 0, section_id: 0 }, async () => {})
    finish({ aid: 456 })
    await first
    await stale
    assert.equal(h.calls.some(([command]) => command === 'submit'), false)
})


test('旧核对请求晚于新合集操作完成时，不覆盖投稿快照或解除新操作的保护', async () => {
    const h = await seasonHarness({ first: { aid: 123, season_id: 10 } })
    h.setQuery(async () => { throw new Error('离线') })
    await assert.rejects(h.store.changeVideoSeason(1, [123], { season_id: 0, section_id: 0 }, async () => { throw new Error('响应丢失') }))
    h.setQuery(async () => ({ season_id: 0, section_id: 0 }))
    const invoke = globalThis.__configInvoke
    let finishOldSync
    let hold = true
    globalThis.__configInvoke = async (command, args) => {
        if (command === 'sync_template_season' && hold) {
            hold = false
            return new Promise(resolve => { finishOldSync = resolve })
        }
        return invoke(command, args)
    }
    const snapshot = { aid: 123, season_id: 10 }
    const checking = h.store.ensureVideoSeason(1, snapshot)
    while (!finishOldSync) await Promise.resolve()
    h.setQuery(async () => { throw new Error('新操作仍待核对') })
    await assert.rejects(h.store.changeVideoSeason(1, [123], { season_id: 20, section_id: 21 }, async () => { throw new Error('新操作响应丢失') }))
    finishOldSync()
    await assert.rejects(checking, /操作进行中/)
    assert.equal(snapshot.season_id, 10)
    assert.throws(() => h.store.assertVideoSeasonReady(1, 123), /待确认/)
})
