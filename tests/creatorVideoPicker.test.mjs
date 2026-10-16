import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import vm from 'node:vm'
import ts from 'typescript'
import { computed, nextTick, reactive, ref, watch } from 'vue'

// 运行组件的真实 setup 逻辑，用可控 IPC 验证异步请求与选择事件。
const component = readFileSync(
    new URL('../src/components/CreatorVideoPicker.vue', import.meta.url),
    'utf8'
)
const script = component.match(/<script setup lang="ts">([\s\S]*?)<\/script>/)[1]
const ast = ts.createSourceFile('Picker.ts', script, ts.ScriptTarget.Latest, true)
const source = ast.statements
    .filter(statement => !ts.isImportDeclaration(statement))
    .map(statement => statement.getText(ast))
    .join('\n')

test('稿件选择界面文案均有翻译', () => {
    const parent = readFileSync(
        new URL('../src/components/NewTemplete.vue', import.meta.url),
        'utf8'
    )
    const keys = [...(component + parent).matchAll(/\bt\('([^']+)'/g)].map(match => match[1])
    for (const language of ['en-US', 'ja-JP', 'zh-TW']) {
        const catalog = JSON.parse(
            readFileSync(new URL(`../src/i18n/${language}.json`, import.meta.url), 'utf8')
        )
        for (const key of keys) assert.ok(catalog[key], `${language}: ${key}`)
    }
})

const video = (aid, bvid = '') => ({ aid, bvid, title: `稿件 ${aid}`, cover: '' })
const settle = async () => {
    await Promise.resolve()
    await nextTick()
    await Promise.resolve()
}
const waitForRequests = async (h, count) => {
    for (let i = 0; i < 10 && h.requests.length < count; i++) await settle()
    assert.equal(h.requests.length, count)
}

function harness(initial = {}, options = {}) {
    const props = reactive({ modelValue: true, uid: 1, username: '测试账号',
        manageVideo: async (uid, video, action) => events.push([action, uid, video.bvid || `av${video.aid}`]), ...initial })
    const requests = []
    const events = []
    const snapshots = []
    const deletions = []
    const messages = []
    const stops = []
    let unmount
    const context = {
        computed,
        ref,
        watch: (...args) => {
            const stop = watch(...args)
            stops.push(stop)
            return stop
        },
        onBeforeUnmount: fn => {
            unmount = fn
        },
        defineProps: () => props,
        ElMessageBox: { confirm: options.confirm || (async () => {}) },
        defineEmits:
            () =>
            (...args) =>
                events.push(args),
        useUtilsStore: () => ({
            deleteCreatorVideo: async (...args) => { deletions.push(args); return options.delete?.(...args) },
            showMessage: (...args) => messages.push(args),
            openVideoListSnapshot: uid => {
                snapshots.push(['open', uid])
                return options.openSnapshot?.(uid) || Promise.resolve(`snapshot-${snapshots.length}`)
            },
            closeVideoListSnapshot: async (uid, id) => { snapshots.push(['close', uid, id]) },
            getVideoList: (...args) =>
                new Promise((resolve, reject) => {
                    requests.push({ args, resolve, reject })
                })
        }),
        t: (key, values) => (values ? key.replace('{0}', values[0]) : key)
    }
    vm.runInNewContext(
        ts.transpile(source, { target: ts.ScriptTarget.ES2020 }) +
            '\nglobalThis.picker = { items, page, total, loading, actionLoading, error, loadPage, selectVideo, deleteVideo, selectedVideos, updateSelection, confirmSelection, canSelectVideo }',
        context
    )
    return {
        props,
        requests,
        events,
        snapshots,
        deletions,
        messages,
        picker: context.picker,
        dispose() {
            unmount()
            stops.forEach(stop => stop())
        }
    }
}

test('仅打开弹窗且选择账号后加载，按页请求并支持编辑与复制', async context => {
    const h = harness({ modelValue: false })
    context.after(() => h.dispose())
    assert.equal(h.requests.length, 0)
    h.props.modelValue = true
    await nextTick()
    assert.deepEqual(h.requests[0].args, [1, 1, 10])
    h.requests[0].resolve({ items: [video(123, 'BVtest')], total: 25 })
    await settle()
    await h.picker.selectVideo(h.picker.items.value[0])
    assert.deepEqual(h.events, [
        ['edit', 1, 'BVtest'],
        ['update:modelValue', false]
    ])
    h.events.length = 0
    const loading = h.picker.loadPage(2)
    assert.deepEqual(h.requests[1].args, [1, 2, 10])
    assert.equal(h.picker.items.value.length, 0)
    h.requests[1].resolve({ items: [video(456)], total: 25 })
    await loading
    assert.equal(h.picker.page.value, 2)
    assert.equal(h.picker.total.value, 25)
    await h.picker.selectVideo(h.picker.items.value[0], 'copy')
    assert.deepEqual(h.events, [
        ['copy', 1, 'av456'],
        ['update:modelValue', false]
    ])
})

test('稿件操作等待完成才关闭，重复点击被拦截，失败后保留列表可重试', async context => {
    let finish, fail
    const calls = []
    const h = harness({ manageVideo: (...args) => {
        calls.push(args)
        return new Promise((resolve, reject) => { finish = resolve; fail = reject })
    } })
    context.after(() => h.dispose())
    h.requests[0].resolve({ items: [video(123)], total: 1 })
    await settle()
    const row = h.picker.items.value[0]
    const pending = h.picker.selectVideo(row, 'copy')
    await h.picker.selectVideo(row, 'edit')
    assert.equal(calls.length, 1)
    assert.equal(calls[0][0], 1)
    assert.equal(calls[0][2], 'copy')
    assert.equal(h.picker.actionLoading.value, true)
    assert.equal(h.events.length, 0)
    fail(new Error('操作失败'))
    await pending
    assert.equal(h.picker.actionLoading.value, false)
    assert.match(h.picker.error.value, /操作失败/)
    assert.equal(h.events.length, 0)
    assert.equal(h.picker.items.value.length, 1)
    const retry = h.picker.selectVideo(row, 'edit')
    finish()
    await retry
    assert.equal(h.picker.error.value, '')
    assert.deepEqual(h.events, [['update:modelValue', false]])
})

test('操作期间账号失效后重新加载新账号，旧操作完成不会关闭新列表', async context => {
    let finish
    const h = harness({ manageVideo: () => new Promise(resolve => { finish = resolve }) })
    context.after(() => h.dispose())
    h.requests[0].resolve({ items: [video(123)], total: 1 })
    await settle()
    const pending = h.picker.selectVideo(h.picker.items.value[0], 'edit')
    h.props.uid = 2
    await settle()
    assert.equal(h.requests.length, 1)
    finish()
    await pending
    assert.equal(h.events.length, 0)
    assert.deepEqual(h.requests[1].args, [2, 1, 10])
    h.requests[1].resolve({ items: [video(456)], total: 1 })
    await settle()
    assert.equal(h.picker.items.value[0].aid, 456)
    assert.equal(h.picker.actionLoading.value, false)
})

test('删除需要确认，取消不请求；成功保留管理窗口并刷新当前页', async context => {
    const cancelled = harness({}, { confirm: async () => { throw 'cancel' } })
    context.after(() => cancelled.dispose())
    cancelled.requests[0].resolve({ items: [video(123)], total: 1 })
    await settle()
    await cancelled.picker.deleteVideo(cancelled.picker.items.value[0])
    assert.equal(cancelled.deletions.length, 0)
    assert.equal(cancelled.picker.actionLoading.value, false)
    const h = harness()
    context.after(() => h.dispose())
    h.requests[0].resolve({ items: [video(123), video(456)], total: 2 })
    await settle()
    const pending = h.picker.deleteVideo(h.picker.items.value[0])
    await h.picker.deleteVideo(h.picker.items.value[0])
    await settle()
    assert.deepEqual(h.deletions, [[1, 123]])
    assert.deepEqual(h.requests[1].args, [1, 1, 10])
    h.requests[1].resolve({ items: [video(456)], total: 1 })
    await pending
    assert.equal(h.picker.items.value[0].aid, 456)
    assert.equal(h.events.length, 0)
    assert.equal(h.messages[0][1], 'success')
})

test('删除最后一页的最后一条后回到有效页', async context => {
    const h = harness()
    context.after(() => h.dispose())
    h.requests[0].resolve({ items: [], total: 11 })
    await settle()
    const second = h.picker.loadPage(2)
    h.requests[1].resolve({ items: [video(123)], total: 11 })
    await second
    const pending = h.picker.deleteVideo(h.picker.items.value[0])
    await waitForRequests(h, 3)
    h.requests[2].resolve({ items: [], total: 10 })
    await waitForRequests(h, 4)
    assert.deepEqual(h.requests[3].args, [1, 1, 10])
    h.requests[3].resolve({ items: [video(456)], total: 10 })
    await pending
    assert.equal(h.picker.page.value, 1)
    assert.equal(h.deletions.length, 1)
})

test('删除结果异常仍刷新核对，保留错误且不自动重发删除', async context => {
    const h = harness({}, { delete: async () => { throw new Error('结果未确认') } })
    context.after(() => h.dispose())
    h.requests[0].resolve({ items: [video(123)], total: 1 })
    await settle()
    const pending = h.picker.deleteVideo(h.picker.items.value[0])
    await waitForRequests(h, 2)
    h.requests[1].resolve({ items: [video(123)], total: 1 })
    await pending
    assert.equal(h.deletions.length, 1)
    assert.equal(h.picker.items.value.length, 1)
    assert.match(h.picker.error.value, /结果未确认/)
    assert.equal(h.messages.length, 0)
})

test('删除确认期间换账号不会删除旧账号稿件', async context => {
    let confirm
    const h = harness({}, { confirm: () => new Promise(resolve => { confirm = resolve }) })
    context.after(() => h.dispose())
    h.requests[0].resolve({ items: [video(123)], total: 1 })
    await settle()
    const pending = h.picker.deleteVideo(h.picker.items.value[0])
    h.props.uid = 2
    await settle()
    confirm()
    await pending
    assert.equal(h.deletions.length, 0)
    assert.deepEqual(h.requests[1].args, [2, 1, 10])
})

test('切换账号后丢弃旧结果，旧请求结束不影响新请求的加载状态', async context => {
    const h = harness()
    context.after(() => h.dispose())
    h.props.uid = 2
    await nextTick()
    assert.deepEqual(h.requests[1].args, [2, 1, 10])
    h.requests[0].resolve({ items: [video(111)], total: 1 })
    await settle()
    assert.equal(h.picker.items.value.length, 0)
    assert.equal(h.picker.loading.value, true)
    h.requests[1].resolve({ items: [video(222)], total: 1 })
    await settle()
    assert.equal(h.picker.items.value[0].aid, 222)
    assert.equal(h.picker.loading.value, false)
})

test('关闭及卸载后的请求不再发布，失败可以刷新重试', async context => {
    const h = harness()
    context.after(() => h.dispose())
    h.props.modelValue = false
    await nextTick()
    h.requests[0].reject(new Error('旧请求失败'))
    await settle()
    assert.equal(h.picker.error.value, '')
    h.props.modelValue = true
    await nextTick()
    h.requests[1].reject(new Error('网络异常'))
    await settle()
    assert.match(h.picker.error.value, /网络异常/)
    assert.equal(h.picker.loading.value, false)
    const retry = h.picker.loadPage(1)
    h.requests[2].resolve({ items: [], total: 0 })
    await retry
    assert.equal(h.picker.error.value, '')
    assert.equal(h.picker.items.value.length, 0)
    const pending = h.picker.loadPage(2)
    h.dispose()
    h.requests[3].resolve({ items: [video(333)], total: 1 })
    await pending
    assert.equal(h.picker.items.value.length, 0)
})

test('连续翻页仅使用最新页，加载期间不能选中旧稿件', async context => {
    const h = harness()
    context.after(() => h.dispose())
    const latest = h.picker.loadPage(2)
    h.picker.selectVideo(video(111))
    assert.equal(h.events.length, 0)
    h.requests[1].resolve({ items: [video(222)], total: 21 })
    await latest
    h.requests[0].resolve({ items: [video(111)], total: 21 })
    await settle()
    assert.equal(h.picker.items.value[0].aid, 222)
    assert.equal(h.picker.page.value, 2)
})

test('添加单集请求筛选后的分页与总数，翻页清空选择，普通编辑列表不筛选', async context => {
    const h = harness({ seasonAddOnly: true })
    context.after(() => h.dispose())
    const videos = [
        { ...video(1), season_add_state: 0 },
        { ...video(2), season_add_state: 1 },
        video(3),
        { ...video(4), season_add_state: null },
        { ...video(5), season_add_state: '0' }
    ]
    await settle()
    assert.deepEqual(h.requests[0].args, [1, 1, 5, true, 'snapshot-1'])
    h.requests[0].resolve({ items: [videos[0]], total: 1 })
    await settle()
    assert.deepEqual(Array.from(h.picker.items.value, row => row.aid), [1])
    h.picker.updateSelection([videos[1]])
    h.picker.confirmSelection()
    assert.equal(h.events.length, 0)
    h.picker.updateSelection([h.picker.items.value[0]])
    h.picker.confirmSelection()
    assert.equal(JSON.stringify(h.events), JSON.stringify([['select-many', ['av1']], ['update:modelValue', false]]))
    h.events.length = 0
    const pending = h.picker.loadPage(2)
    await settle()
    assert.deepEqual(h.requests[1].args, [1, 2, 5, true, 'snapshot-1'])
    assert.equal(h.picker.selectedVideos.value.length, 0)
    h.picker.confirmSelection()
    assert.equal(h.events.length, 0)
    h.requests[1].resolve({ items: [], total: 1 })
    await pending
    assert.equal(h.picker.items.value.length, 0)
    assert.equal(h.picker.total.value, 1)
    h.props.seasonAddOnly = false
    await nextTick()
    h.requests[2].resolve({ items: videos, total: 25 })
    assert.deepEqual(h.requests[2].args, [1, 1, 10])
    await settle()
    assert.equal(h.picker.items.value.length, 5)
})

test('复选框多选按列表顺序统一确认，每页请求五条，刷新清空旧选择', async context => {
    const h = harness({ seasonAddOnly: true })
    context.after(() => h.dispose())
    await settle()
    assert.deepEqual(h.requests[0].args, [1, 1, 5, true, 'snapshot-1'])
    h.requests[0].resolve({ items: [
        { ...video(1, 'BVone'), season_add_state: 0 },
        { ...video(2), season_add_state: 0 }
    ], total: 12 })
    await settle()
    const [first, second] = h.picker.items.value
    h.picker.updateSelection([second, first])
    assert.equal(h.events.length, 0)
    h.picker.confirmSelection()
    assert.equal(JSON.stringify(h.events), JSON.stringify([
        ['select-many', ['BVone', 'av2']], ['update:modelValue', false]
    ]))
    h.events.length = 0
    const refresh = h.picker.loadPage(1)
    assert.equal(h.picker.selectedVideos.value.length, 0)
    assert.equal(h.picker.canSelectVideo(first), false)
    h.picker.updateSelection([first])
    h.picker.confirmSelection()
    assert.equal(h.events.length, 0)
    await settle()
    h.requests[1].resolve({ items: [], total: 0 })
    await refresh
    assert.equal(h.picker.selectedVideos.value.length, 0)
})

test('同次弹窗翻页复用快照，刷新、换账号和关闭都会释放快照', async context => {
    const h = harness({ seasonAddOnly: true })
    context.after(() => h.dispose())
    await settle()
    h.requests[0].resolve({ items: [], total: 12 })
    await settle()
    const second = h.picker.loadPage(2)
    await settle()
    assert.equal(h.snapshots.filter(call => call[0] === 'open').length, 1)
    assert.equal(h.requests[1].args[4], h.requests[0].args[4])
    h.requests[1].resolve({ items: [], total: 12 })
    await second
    const refresh = h.picker.loadPage(1, true)
    await settle()
    assert.ok(h.snapshots.some(call => call[0] === 'close' && call[2] === 'snapshot-1'))
    assert.notEqual(h.requests[2].args[4], 'snapshot-1')
    h.requests[2].resolve({ items: [], total: 3 })
    await refresh
    h.props.uid = 2
    await settle()
    assert.equal(h.requests[3].args[0], 2)
    assert.equal(h.snapshots.filter(call => call[0] === 'close').length, 2)
    const currentId = h.requests[3].args[4]
    h.props.modelValue = false
    await settle()
    assert.ok(h.snapshots.some(call => call[0] === 'close' && call[1] === 2 && call[2] === currentId))
    h.requests[3].resolve({ items: [video(99)], total: 1 })
    await settle()
    assert.equal(h.picker.items.value.length, 0)
})

test('关闭后才创建完成的快照仍被释放，不继续发起稿件查询', async context => {
    let resolve
    const pending = new Promise(yes => { resolve = yes })
    const h = harness({ seasonAddOnly: true }, { openSnapshot: () => pending })
    context.after(() => h.dispose())
    h.props.modelValue = false
    await settle()
    resolve('late-snapshot')
    await settle()
    assert.equal(h.requests.length, 0)
    assert.ok(h.snapshots.some(call => call[0] === 'close' && call[2] === 'late-snapshot'))
})
