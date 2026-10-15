import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import vm from 'node:vm'
import ts from 'typescript'
import { nextTick, reactive, ref, watch } from 'vue'

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
}

function harness(initial = {}) {
    const props = reactive({ modelValue: true, uid: 1, username: '测试账号', ...initial })
    const requests = []
    const events = []
    const stops = []
    let unmount
    const context = {
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
        defineEmits:
            () =>
            (...args) =>
                events.push(args),
        useUtilsStore: () => ({
            getVideoList: (...args) =>
                new Promise((resolve, reject) => {
                    requests.push({ args, resolve, reject })
                })
        }),
        t: (key, values) => (values ? key.replace('{0}', values[0]) : key)
    }
    vm.runInNewContext(
        ts.transpile(source) +
            '\nglobalThis.picker = { items, page, total, loading, error, loadPage, selectVideo }',
        context
    )
    return {
        props,
        requests,
        events,
        picker: context.picker,
        dispose() {
            unmount()
            stops.forEach(stop => stop())
        }
    }
}

test('仅打开弹窗且选择账号后加载，按页请求并回填 BV/AV 号', async context => {
    const h = harness({ modelValue: false })
    context.after(() => h.dispose())
    assert.equal(h.requests.length, 0)
    h.props.modelValue = true
    await nextTick()
    assert.deepEqual(h.requests[0].args, [1, 1, 10])
    h.requests[0].resolve({ items: [video(123, 'BVtest')], total: 25 })
    await settle()
    h.picker.selectVideo(h.picker.items.value[0])
    assert.deepEqual(h.events, [
        ['select', 'BVtest'],
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
    h.picker.selectVideo(h.picker.items.value[0])
    assert.deepEqual(h.events, [
        ['select', 'av456'],
        ['update:modelValue', false]
    ])
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
