import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import vm from 'node:vm'
import ts from 'typescript'
import { setupMethod } from './helpers/vueSetup.mjs'

// 执行 store 中真实的业务方法，验证创建和刷新共用的模板转换。
function storeMethod(file, name) {
    const source = readFileSync(new URL(file, import.meta.url), 'utf8')
    const ast = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true)
    let declaration
    const visit = node => {
        if (ts.isVariableDeclaration(node) && node.name.getText(ast) === name) declaration = node
        ts.forEachChild(node, visit)
    }
    visit(ast)
    return ts.transpile(`const ${declaration.getText(ast)}; globalThis.run = ${name}`, {
        target: ts.ScriptTarget.ES2020
    })
}

const build = storeMethod('../src/stores/utils.ts', 'getVideoTemplate')
const create = storeMethod('../src/stores/user_config.ts', 'createTemplateFromVideo')
const original = { aid: 123, title: '稿件', videos: [{ filename: 'remote-file', path: '/local/file' }] }

test('编辑和复制复用相同转换，保留稿件信息与合集，复制只清除 aid', async () => {
    const calls = []
    const context = {
        getVideoDetail: async (...args) => { calls.push(args); return structuredClone(original) },
        getSeasonList: async uid => { assert.equal(uid, 1); return true },
        getVideoSeason: async (uid, aid) => {
            assert.equal(uid, 1)
            assert.equal(aid, 123)
            return { season_id: 10, section_id: 11 }
        }
    }
    vm.runInNewContext(build, context)
    const edit = await context.run(1, 'av123')
    assert.equal(edit.aid, 123)
    assert.equal(edit.title, original.title)
    assert.equal(edit.videos[0].id, 'remote-file')
    assert.equal(edit.videos[0].path, '')
    assert.equal(edit.season_id, 10)
    assert.equal(edit.section_id, 11)
    const copy = await context.run(1, 'av123', false)
    assert.equal(copy.aid, undefined)
    assert.equal(copy.section_id, 11)
    assert.equal(original.videos[0].path, '/local/file')
    assert.deepEqual(calls, [[1, 'av123'], [1, 'av123']])
    context.getSeasonList = async () => false
    context.getVideoSeason = async () => { throw new Error('不应查询不存在的合集') }
    assert.equal((await context.run(1, 'av123')).season_id, undefined)
    context.getVideoDetail = async () => { throw new Error('查询失败') }
    await assert.rejects(context.run(1, 'av123'), /查询失败/)
})

test('创建模板调用共享转换并保存到指定账号，失败不会保存不完整模板', async () => {
    const calls = [], messages = []
    const utils = {
        getVideoTemplate: async (...args) => { calls.push(['build', ...args]); return original },
        showMessage: (...args) => messages.push(args)
    }
    const context = {
        useUtilsStore: () => utils,
        addUserTemplate: async (...args) => calls.push(['save', ...args]),
        t: key => key,
        console: { error: () => {} }
    }
    vm.runInNewContext(create, context)
    await context.run(2, 'av123', '新模板', false)
    assert.deepEqual(calls[0], ['build', 2, 'av123', false])
    assert.deepEqual(calls[1], ['save', 2, '新模板', original])
    utils.getVideoTemplate = async () => { throw new Error('转换失败') }
    await assert.rejects(context.run(2, 'av123', '失败模板'), /转换失败/)
    assert.equal(calls.filter(call => call[0] === 'save').length, 1)
    assert.equal(messages.at(-1)[1], 'error')
})

const syncSeason = storeMethod('../src/stores/user_config.ts', 'syncVideoSeason')
test('同步合集归属保留本地编辑，并隔离账号与稿件', async () => {
    const template = { aid: 123, title: 'local draft', videos: [{ path: '/draft' }], season_id: 0 }
    const other = { aid: 999, season_id: 20 }
    const account = { aid: 123, season_id: 30 }
    const context = { invoke: async () => {}, configRoot: { value: { config: {
        1: { templates: { a: template, b: other } }, 2: { templates: { a: account } }
    } } } }
    vm.runInNewContext(syncSeason, context)
    const videos = template.videos
    for (const season of [{ season_id: 10, section_id: 11 }, { season_id: 10, section_id: 12 }, { season_id: 0, section_id: 0 }]) {
        await context.run(1, [123], season)
        assert.equal(template.season_id, season.season_id)
        assert.equal(template.section_id, season.section_id)
        assert.equal(template.title, 'local draft')
        assert.equal(template.videos, videos)
    }
    assert.equal(other.season_id, 20)
    assert.equal(account.season_id, 30)
})


test('实际投稿入口核对失败时不提交，重试成功后使用核对后的合集归属', async () => {
    const handler = setupMethod(readFileSync(new URL('../src/views/MainView.vue', import.meta.url), 'utf8'), 'performTemplateSubmit')
    const submitted = [], synced = []
    const context = {
        loginUsers: { value: [{ uid: 1, username: '测试账号' }] },
        submitting: { value: false }, selectedUser: { value: null }, currentTemplateName: { value: '' },
        userConfigStore: {
            ensureVideoSeason: async () => { throw new Error('归属核对失败') },
            getSeasonRevision: () => 0
        },
        uploadStore: { submitTemplate: async (uid, form) => { submitted.push(form); return { aid: form.aid } } },
        syncSeasonAfterSubmit: async (...args) => synced.push(args),
        recordSubmitStats: () => {}, collectFailedVideoNames: () => [],
        utilsStore: { showMessage: () => {} }, console: { error: () => {}, log: () => {} },
        handleAutoEditAfterSubmit: async () => {}, setTimeout: callback => { callback() },
        t: key => key
    }
    vm.runInNewContext(handler + '\nglobalThis.submit = performTemplateSubmit', context)
    const template = { aid: 123, season_id: 10, section_id: 11, title: '本地编辑' }
    await context.submit(1, '模板', template)
    assert.equal(submitted.length, 0)
    context.userConfigStore.ensureVideoSeason = async (uid, snapshot) => {
        snapshot.season_id = 0
        snapshot.section_id = 0
    }
    await context.submit(1, '模板', template)
    assert.equal(submitted[0].season_id, 0)
    assert.equal(synced[0][2].season_id, 0)
    assert.equal(submitted[0].title, '本地编辑')
    context.uploadStore.submitTemplate = async () => {
        context.userConfigStore.getSeasonRevision = () => 1
        return { aid: 123 }
    }
    await context.submit(1, '模板', template)
    assert.equal(synced.length, 1)
})
