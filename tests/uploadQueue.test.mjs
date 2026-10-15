import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import vm from 'node:vm'
import { fileURLToPath } from 'node:url'
import { build } from 'esbuild'
import ts from 'typescript'

const root = fileURLToPath(new URL('..', import.meta.url))
const bundle = await build({
    stdin: {
        contents: [
            "export { useUploadStore } from './src/stores/upload.ts'",
            "export { createPinia, setActivePinia } from 'pinia'",
            "export { watch } from 'vue'"
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
            name: 'mock-upload-ipc',
            setup(builder) {
                builder.onResolve({ filter: /^@tauri-apps\/api\/core$/ }, () => ({
                    path: 'core',
                    namespace: 'mock-upload'
                }))
                builder.onLoad({ filter: /.*/, namespace: 'mock-upload' }, () => ({
                    contents:
                        'export const invoke = (...args) => globalThis.__uploadInvoke(...args)'
                }))
            }
        }
    ]
})
const { useUploadStore, createPinia, setActivePinia, watch } = await import(
    'data:text/javascript;base64,' + Buffer.from(bundle.outputFiles[0].text).toString('base64')
)
const deferred = () => {
    let resolve, reject
    const promise = new Promise((yes, no) => {
        resolve = yes
        reject = no
    })
    return { promise, resolve, reject }
}
const tick = () => new Promise(resolve => setImmediate(resolve))
const batchResult = { successCount: 1, noopTaskIds: [], failedTaskIds: [] }
const task = (id = 'one', overrides = {}) => ({
    id,
    template: '模板',
    user: { uid: 1, username: '用户' },
    video: { id, title: '视频', filename: '', path: '/video.mp4' },
    status: 'Running',
    progress: 0,
    total_transmit_bytes: 0,
    total_size: 1000,
    created_at: 0,
    started_at: 100,
    retry_count: 0,
    ...overrides
})
const freshStore = invoke => {
    setActivePinia(createPinia())
    globalThis.__uploadInvoke = invoke
    return useUploadStore()
}

test('并发普通查询共享一次 IPC', async () => {
    const response = deferred()
    let calls = 0
    const store = freshStore(command => {
        assert.equal(command, 'get_upload_queue')
        calls++
        return response.promise
    })
    const waiting = [store.getUploadQueue(), store.getUploadQueue(), store.getUploadQueue()]
    assert.equal(calls, 1)
    response.resolve([task()])
    const results = await Promise.all(waiting)
    assert.equal(calls, 1)
    for (const result of results) assert.equal(result, store.uploadQueue)
})

test('多个操作期间的旧查询被丢弃，操作和轮询共同等待一次补查', async () => {
    const responses = []
    const store = freshStore(command => {
        if (command !== 'get_upload_queue') return Promise.resolve(batchResult)
        const response = deferred()
        responses.push(response)
        return response.promise
    })
    const polling = store.getUploadQueue()
    const starting = store.startUpload('one')
    const pausing = store.pauseUpload('one')
    await tick()
    responses[0].resolve([task('one', { status: 'Completed' })])
    await tick()
    assert.equal(responses.length, 2)
    assert.equal(store.uploadQueue.length, 0)
    responses[1].resolve([task('one', { status: 'Paused' })])
    await Promise.all([polling, starting, pausing])
    assert.equal(store.uploadQueue[0].status, 'Paused')
    assert.equal(responses.length, 2)
})

test('查询刚返回而操作同时完成时，也必须补查操作后的状态', async () => {
    const old = deferred(),
        action = deferred(),
        latest = deferred()
    let queries = 0
    const store = freshStore(command =>
        command === 'get_upload_queue'
            ? ++queries === 1
                ? old.promise
                : latest.promise
            : action.promise
    )
    const polling = store.getUploadQueue()
    const cancelling = store.cancelUpload('one')
    old.resolve([task()])
    action.resolve(batchResult)
    await tick()
    assert.equal(queries, 2)
    latest.resolve([])
    await Promise.all([polling, cancelling])
    assert.equal(store.uploadQueue.length, 0)
})

test('共享请求失败后释放查询状态，后续查询仍能成功', async context => {
    const errors = context.mock.method(console, 'error', () => {})
    const failed = deferred()
    let queries = 0
    const store = freshStore(() => (++queries === 1 ? failed.promise : Promise.resolve([task()])))
    const first = assert.rejects(store.getUploadQueue(), /查询失败/)
    const second = assert.rejects(store.getUploadQueue(), /查询失败/)
    failed.reject(new Error('查询失败'))
    await Promise.all([first, second])
    assert.equal(errors.mock.callCount(), 1)
    await store.getUploadQueue()
    assert.equal(queries, 2)
    assert.equal(store.uploadQueue.length, 1)
})

test('操作使失败的旧查询失效时，等待者使用新查询结果', async () => {
    const old = deferred()
    let queries = 0
    const store = freshStore(command =>
        command === 'get_upload_queue'
            ? ++queries === 1
                ? old.promise
                : Promise.resolve([])
            : Promise.resolve(batchResult)
    )
    const polling = store.getUploadQueue()
    const cancelling = store.cancelUpload('one')
    await tick()
    old.reject(new Error('旧查询失败'))
    await Promise.all([polling, cancelling])
    assert.equal(queries, 2)
    assert.equal(store.uploadQueue.length, 0)
})

test('无变化时保留数组和嵌套对象，仅状态变化也能触发监听', async () => {
    let snapshot = [task()]
    const store = freshStore(() => Promise.resolve(structuredClone(snapshot)))
    await store.getUploadQueue()
    const queue = store.uploadQueue,
        saved = queue[0],
        video = saved.video,
        user = saved.user
    let changes = 0
    let queueChanges = 0
    const stop = watch(
        () => saved.status,
        () => changes++,
        { flush: 'sync' }
    )
    const stopQueue = watch(
        () => store.uploadQueue,
        () => queueChanges++,
        { deep: true, flush: 'sync' }
    )
    await store.getUploadQueue()
    assert.equal(store.uploadQueue, queue)
    assert.equal(store.uploadQueue[0], saved)
    assert.equal(saved.video, video)
    assert.equal(saved.user, user)
    assert.equal(changes, 0)
    assert.equal(queueChanges, 0)
    snapshot[0].status = 'Completed'
    snapshot[0].video.filename = 'uploaded'
    snapshot[0].video.path = ''
    await store.getUploadQueue()
    assert.equal(store.uploadQueue, queue)
    assert.equal(saved.video, video)
    assert.equal(video.filename, 'uploaded')
    assert.equal(video.path, '')
    assert.equal(changes, 1)
    stop()
    stopQueue()
    snapshot = [task('two'), snapshot[0]]
    await store.getUploadQueue()
    assert.deepEqual(
        store.uploadQueue.map(item => item.id),
        ['two', 'one']
    )
    assert.equal(store.uploadQueue[1], saved)
    snapshot = [snapshot[0]]
    await store.getUploadQueue()
    assert.deepEqual(
        store.uploadQueue.map(item => item.id),
        ['two']
    )
})

test('速度采样在暂停、计数归零、重试及重新开始后重置', async () => {
    const originalNow = Date.now
    let now = 1000
    Date.now = () => now
    try {
        const snapshot = task('one', { total_transmit_bytes: 100 })
        const store = freshStore(() => Promise.resolve([structuredClone(snapshot)]))
        const sample = async (values, speed) => {
            Object.assign(snapshot, values)
            await store.getUploadQueue()
            assert.equal(store.uploadQueue[0].speed, speed)
            now += 1000
        }
        await sample({}, 0)
        await sample({ total_transmit_bytes: 200 }, 100)
        await sample({ status: 'Paused', total_transmit_bytes: 210 }, 0)
        await sample({ status: 'Running', total_transmit_bytes: 250 }, 0)
        await sample({ total_transmit_bytes: 350 }, 100)
        await sample({ total_transmit_bytes: 20 }, 0)
        await sample({ retry_count: 1, total_transmit_bytes: 120 }, 0)
        await sample({ started_at: 200, total_transmit_bytes: 220 }, 0)
    } finally {
        Date.now = originalNow
    }
})

test('不立即刷新的重试仍使进行中的旧快照失效', async () => {
    const old = deferred()
    let queries = 0
    const store = freshStore(command =>
        command === 'get_upload_queue'
            ? ++queries === 1
                ? old.promise
                : Promise.resolve([task('one', { retry_count: 1 })])
            : Promise.resolve(batchResult)
    )
    const polling = store.getUploadQueue()
    await store.retryUpload('one', false)
    old.resolve([task()])
    await polling
    assert.equal(queries, 2)
    assert.equal(store.uploadQueue[0].retry_count, 1)
    await store.retryUpload('one', false)
    assert.equal(queries, 2)
})

// 从真实组件中提取轮询和卸载函数，在可控计时器中运行，不复制调度实现。
const mainView = readFileSync(new URL('../src/views/MainView.vue', import.meta.url), 'utf8')
const script = mainView.match(/<script setup lang="ts">([\s\S]*?)<\/script>/)[1]
const ast = ts.createSourceFile('MainView.ts', script, ts.ScriptTarget.Latest, true)
const pollingSource = ast.statements
    .filter(statement => {
        if (ts.isVariableStatement(statement)) {
            return statement.declarationList.declarations.some(item =>
                [
                    'generalUpdateTimer',
                    'queuePollingActive',
                    'queuePollingDisposed',
                    'startQueueUpdates'
                ].includes(item.name.getText(ast))
            )
        }
        return (
            ts.isExpressionStatement(statement) &&
            ts.isCallExpression(statement.expression) &&
            statement.expression.expression.getText(ast) === 'onUnmounted'
        )
    })
    .map(statement => statement.getText(ast))
    .join('\n')
function pollingHarness({ status = 'Running', response = Promise.resolve(), users = [{}] } = {}) {
    const timers = new Map()
    let timerId = 0,
        cleanup,
        queries = 0
    const video = { id: 'one', filename: '', path: '/video.mp4' }
    const context = {
        authStore: { loginUsers: users },
        uploadStore: {
            uploadQueue: [task('one', { status })],
            getUploadQueue: () => {
                queries++
                return response
            }
        },
        userConfigStore: {
            configRoot: { config: { 1: { templates: { 模板: { videos: [video] } } } } }
        },
        window: {
            setTimeout: (fn, delay) => {
                timers.set(++timerId, { fn, delay })
                return timerId
            }
        },
        clearTimeout: id => timers.delete(id),
        onUnmounted: fn => {
            cleanup = fn
        },
        consoleCleanups: [],
        keyboardCleanup: null,
        clearAllSeparateSubmitStates() {},
        autoSubmitProcessingKeys: { value: new Set() },
        autoSubmittingRecord: { value: {} },
        console: { error() {} },
        t: value => value
    }
    vm.runInNewContext(
        ts.transpile(pollingSource) + '\nglobalThis.start = startQueueUpdates',
        context
    )
    const fire = () => {
        const [id, timer] = timers.entries().next().value
        timers.delete(id)
        return timer.fn()
    }
    return {
        start: context.start,
        timers,
        fire,
        cleanup: () => cleanup(),
        queries: () => queries,
        video,
        context
    }
}

test('轮询等请求结束才安排下一轮，并按活动状态调整间隔', async () => {
    const response = deferred()
    const poll = pollingHarness({ response: response.promise })
    poll.start()
    poll.start()
    assert.equal(poll.timers.size, 1)
    const refreshing = poll.fire()
    assert.equal(poll.timers.size, 0)
    assert.equal(poll.queries(), 1)
    response.resolve()
    await refreshing
    assert.equal([...poll.timers.values()][0].delay, 666)
    for (const status of ['Pending', 'AwaitingVerification']) {
        poll.context.uploadStore.uploadQueue[0].status = status
        await poll.fire()
        assert.equal([...poll.timers.values()][0].delay, 666)
    }
    poll.context.uploadStore.uploadQueue[0].status = 'Paused'
    await poll.fire()
    assert.equal([...poll.timers.values()][0].delay, 3000)
    poll.cleanup()
    assert.equal(poll.timers.size, 0)
})

test('完成状态继续同步到模板，供自动提交监听使用', async () => {
    const poll = pollingHarness({ status: 'Completed' })
    Object.assign(poll.context.uploadStore.uploadQueue[0], {
        video: { id: 'one', filename: 'uploaded', path: '' },
        finished_at: 123
    })
    poll.start()
    await poll.fire()
    assert.equal(poll.video.filename, 'uploaded')
    assert.equal(poll.video.path, '')
    assert.equal(poll.video.complete, true)
    assert.equal(poll.video.finished_at, 123)
    poll.cleanup()
})

test('卸载期间返回的请求不再同步模板或启动计时器', async () => {
    const response = deferred()
    const poll = pollingHarness({ status: 'Completed', response: response.promise })
    poll.context.uploadStore.uploadQueue[0].video.filename = 'uploaded'
    poll.start()
    const refreshing = poll.fire()
    poll.cleanup()
    response.resolve()
    await refreshing
    assert.equal(poll.video.filename, '')
    assert.equal(poll.timers.size, 0)
    poll.start()
    assert.equal(poll.timers.size, 0)
})

test('查询失败后继续轮询；没有登录账号时不发送查询', async () => {
    const response = deferred()
    const poll = pollingHarness({ response: response.promise })
    poll.start()
    const refreshing = poll.fire()
    response.reject(new Error('轮询失败'))
    await refreshing
    assert.equal(poll.timers.size, 1)
    poll.cleanup()
    const loggedOut = pollingHarness({ users: [] })
    loggedOut.start()
    await loggedOut.fire()
    assert.equal(loggedOut.queries(), 0)
    assert.equal([...loggedOut.timers.values()][0].delay, 3000)
    loggedOut.cleanup()
})
