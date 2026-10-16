import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import vm from 'node:vm'
import ts from 'typescript'
import { ref, reactive, watch, nextTick } from 'vue'

const component = readFileSync(new URL('../src/components/VideoStatus.vue', import.meta.url), 'utf8')
const script = component.match(/<script setup lang="ts">([\s\S]*?)<\/script>/)[1]
const ast = ts.createSourceFile('VideoStatus.ts', script, ts.ScriptTarget.Latest, true)
const source = ast.statements.filter(node => !ts.isImportDeclaration(node)).map(node => node.getText(ast)).join('\n')
const settle = async () => { await Promise.resolve(); await nextTick(); await Promise.resolve() }
const list = (bvid = 'BVtest', videos = [{ cid: 101, index: 1, xcode_state: 2 }, { cid: 102, index: 2, xcode_state: 1 }]) => ({
    bvid, page: { total: 21, pn: 1, ps: 20 }, videos,
    audit: { aegis_state: 2, audit_process: null }
})
const detail = (cid, state = 1) => ({ cid, xcode_state: state, xcode_tip: '', fail_tip: '', transcode_list: [] })

function harness(initial = {}) {
    const props = reactive({ modelValue: true, uid: 1, aid: 123, ...initial })
    const lists = [], details = [], stops = []
    let unmount
    const context = {
        ref, defineProps: () => props, defineEmits: () => () => {},
        watch: (...args) => { const stop = watch(...args); stops.push(stop); return stop },
        onBeforeUnmount: fn => { unmount = fn },
        useUtilsStore: () => ({
            getArchiveProcessing: (...args) => new Promise((resolve, reject) => lists.push({ args, resolve, reject })),
            getVideoTranscode: (...args) => new Promise((resolve, reject) => details.push({ args, resolve, reject }))
        }),
        t: (key, params) => key.replace(/\{(\w+)\}/g, (match, name) => params?.[name] ?? match)
    }
    vm.runInNewContext(ts.transpile(source, { target: ts.ScriptTarget.ES2020 }) +
        '\nglobalThis.status = { processing, transcode, selectedCid, page, loading, transcodeLoading, error, transcodeError, loadPage, loadTranscode, resolutionProgress, resolutionText, xcodeText, auditText, stepText }', context)
    return { props, lists, details, status: context.status, dispose: () => { unmount(); stops.forEach(stop => stop()) } }
}

test('只在打开已投稿稿件后查询，优先选择转码中的 P，审核独立显示', async context => {
    const h = harness({ modelValue: false })
    context.after(h.dispose)
    assert.equal(h.lists.length, 0)
    h.props.modelValue = true
    await settle()
    assert.deepEqual(h.lists[0].args, [1, 123, 1])
    h.lists[0].resolve(list())
    await settle()
    assert.deepEqual(h.details[0].args, [1, 'BVtest', 102])
    assert.equal(h.status.processing.value.audit.aegis_state, 2)
    assert.equal(h.status.loading.value, false)
    h.details[0].resolve(detail(102))
    await settle()
    assert.equal(h.status.transcode.value.xcode_state, 1)
    assert.equal(h.status.auditText(2), '已完成')
    assert.equal(h.status.xcodeText(1), '转码中')
    const refresh = h.status.loadPage(1)
    h.lists[1].resolve(list())
    await refresh
    assert.equal(h.status.selectedCid.value, 102)
    const next = h.status.loadPage(2)
    assert.deepEqual(h.lists[2].args, [1, 123, 2])
    h.lists[2].resolve(list('BVtest', [{ cid: 121, index: 21, xcode_state: 2 }]))
    await next
    assert.equal(h.status.selectedCid.value, 121)
})

test('快速切换 P、账号、稿件和关闭后均丢弃过期结果', async context => {
    const h = harness()
    context.after(h.dispose)
    h.lists[0].resolve(list())
    await settle()
    const oldPart = h.details[0]
    const select = h.status.loadTranscode(h.status.processing.value.videos[0])
    h.details[1].resolve(detail(101, 2))
    await select
    oldPart.resolve(detail(102))
    await settle()
    assert.equal(h.status.transcode.value.cid, 101)
    h.props.uid = 2
    h.props.aid = 456
    await settle()
    assert.equal(h.status.transcode.value, null)
    assert.deepEqual(h.lists[1].args, [2, 456, 1])
    h.props.modelValue = false
    await settle()
    h.lists[1].resolve(list('BVother'))
    await settle()
    assert.equal(h.status.processing.value, null)
    assert.equal(h.status.loading.value, false)
})

test('列表与单 P 错误可重试，单 P 失败保留分 P 与审核信息', async context => {
    const h = harness()
    context.after(h.dispose)
    h.lists[0].reject(new Error('列表失败'))
    await settle()
    assert.match(h.status.error.value, /列表失败/)
    const retry = h.status.loadPage(1)
    h.lists[1].resolve(list())
    await retry
    h.details[0].reject(new Error('转码失败'))
    await settle()
    assert.match(h.status.transcodeError.value, /转码失败/)
    assert.equal(h.status.processing.value.videos.length, 2)
    const retryPart = h.status.loadTranscode(h.status.processing.value.videos[1])
    h.details[1].resolve(detail(102, 2))
    await retryPart
    assert.equal(h.status.transcodeError.value, '')
    assert.equal(h.status.transcode.value.xcode_state, 2)
})

test('清晰度进度沿用官网估算，处理中不显示完成，异常时间不产生无效百分比', context => {
    const h = harness({ modelValue: false })
    context.after(h.dispose)
    const resolution = { status: 'processing', start_time: 100, time_now: 125, estimated_time: 200 }
    assert.equal(h.status.resolutionProgress(resolution), 25)
    assert.equal(h.status.resolutionText(resolution), '预计 25%')
    assert.equal(h.status.resolutionProgress({ ...resolution, time_now: 300 }), 99)
    assert.equal(h.status.resolutionProgress({ ...resolution, progress: 50 }), 50)
    assert.equal(h.status.resolutionProgress({ ...resolution, estimated_time: 100 }), 0)
    assert.equal(h.status.resolutionProgress({ ...resolution, time_now: 50 }), 0)
    assert.equal(h.status.resolutionProgress({ ...resolution, status: 'success' }), 100)
    assert.equal(h.status.resolutionText({ ...resolution, estimated_time: 0 }), '未开始')
    assert.equal(h.status.resolutionText({ status: 'failed', failure_reason: '编码错误' }), '编码错误')
    assert.equal(h.status.xcodeText(99), '状态未知')
})

test('进度界面及状态文案有翻译，打开弹窗不刷新或覆盖模板', () => {
    const keys = [...component.matchAll(/\bt\('([^']+)'/g)].map(match => match[1])
    keys.push('未开始', '转码中', '转码完成', '转码失败', '状态未知', '审核中', '已完成', '已退回', '已锁定', '受限', '进行中')
    for (const lang of ['en-US', 'ja-JP', 'zh-TW']) {
        const catalog = JSON.parse(readFileSync(new URL(`../src/i18n/${lang}.json`, import.meta.url), 'utf8'))
        for (const key of keys) assert.ok(catalog[key], `${lang}: ${key}`)
    }
    const main = readFileSync(new URL('../src/views/MainView.vue', import.meta.url), 'utf8').match(/<script setup lang="ts">([\s\S]*?)<\/script>/)[1]
    const ast = ts.createSourceFile('Main.ts', main, ts.ScriptTarget.Latest, true)
    const handler = ast.statements.find(node => ts.isVariableStatement(node) && node.declarationList.declarations.some(d => d.name.getText(ast) === 'checkVideoStatus'))
    const draft = { aid: 123, title: '本地修改' }
    const context = { selectedUser: ref({ uid: 1 }), currentTemplate: ref(draft), showVideoStatusDialog: ref(false) }
    vm.runInNewContext(ts.transpile(handler.getText(ast)) + '\nglobalThis.open = checkVideoStatus', context)
    context.open()
    assert.equal(context.showVideoStatusDialog.value, true)
    assert.equal(draft.title, '本地修改')
})
