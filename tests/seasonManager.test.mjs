import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import vm from 'node:vm'
import { compileSetup, setupMethod } from './helpers/vueSetup.mjs'
import { computed, nextTick, reactive, ref, watch } from 'vue'

const component = readFileSync(
    new URL('../src/components/SeasonManager.vue', import.meta.url),
    'utf8'
)
const setup = compileSetup(component)
const entry = (id = 10, noSection = 1) => ({
    season: { id, title: `合集 ${id}`, no_section: noSection, ep_num: 2 },
    sections: { sections: [{ id: 11, title: '正片' }] }
})
const initialDetail = () => ({
    section: { id: 11, title: '正片' },
    episodes: [
        { id: 21, aid: 121, title: '稿件一' },
        { id: 22, aid: 122, title: '稿件二' }
    ]
})
const deferred = () => {
    let resolve, reject
    const promise = new Promise((yes, no) => {
        resolve = yes
        reject = no
    })
    return { promise, resolve, reject }
}
const settle = async () => {
    await Promise.resolve()
    await nextTick()
    await Promise.resolve()
}

function harness(options = {}) {
    const props = reactive({ modelValue: true, initialUid: 1, ...options.props })
    const auth = reactive({
        loginUsers: [
            { uid: 1, username: '账号一' },
            { uid: 2, username: '账号二' }
        ]
    })
    const calls = [],
        events = [],
        messages = [],
        stops = []
    let remote = initialDetail(),
        unmount
    const utils = {
        getSeasonArchiveTitle: options.archive || (async (_uid, aid) => ({ archiveTitle: `稿件 ${aid}` })),
        listManagedSeasons:
            options.list ||
            (async uid => {
                calls.push(['list', uid])
                return [entry()]
            }),
        getManagedSection:
            options.section ||
            (async (...args) => {
                calls.push(['section', ...args])
                return structuredClone(remote)
            }),
        addSeasonEpisodes: async (...args) => {
            calls.push(['add', ...args])
            if (options.add) await options.add(...args)
            for (const videoId of args[3]) {
                const id = Math.max(...remote.episodes.map(episode => episode.id)) + 1
                remote.episodes.push({ id, aid: id + 100, title: videoId })
            }
            return remote.episodes.slice(-args[3].length).map(episode => episode.aid)
        },
        editSeasonEpisodeTitle: async (...args) => {
            calls.push(['title', ...args])
            if (options.title) await options.title(...args)
            remote.episodes.find(episode => episode.id === args[3]).title = args[4]
        },
        removeSeasonEpisode: async (...args) => {
            calls.push(['remove', ...args])
            remote.episodes = remote.episodes.filter(episode => episode.id !== args[3])
            if (options.remove) await options.remove(...args)
        },
        reorderSeasonEpisodes:
            options.reorder ||
            (async (...args) => {
                calls.push(['reorder', ...args])
                remote.episodes = Array.from(args[3], id =>
                    remote.episodes.find(episode => episode.id === id)
                )
            }),
        showMessage: (...args) => messages.push(args)
    }
    const box = {
        prompt: options.prompt || (async () => ({ value: 'BVtest' })),
        confirm: options.confirm || (async () => {})
    }
    const context = {
        computed,
        nextTick: async () => {
            await nextTick()
            options.afterLayout?.()
        },
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
        useAuthStore: () => auth,
        useUtilsStore: () => utils,
        useUserConfigStore: () => ({
            retryPendingVideoSeasons: async () => {},
            changeVideoSeason: async (uid, aids, season, action) => {
                calls.push(['change', uid, aids, season])
                await action()
                calls.push(['sync', uid, aids, season])
            }
        }),
        ElMessageBox: box,
        t: (key, params) => key.replace(/\{(\w+)\}/g, (match, name) => params?.[name] ?? match)
    }
    vm.runInNewContext(
        setup +
            '\nglobalThis.manager = { uid, seasons, loading, listError, expandedSeasonId, seasonScrollbar, sectionVisible, sectionLoading, sectionError, busy, detail, episodes, target, orderDirty, hasSections, loadSeasons, openSection, loadSection, addEpisode, selectEpisodes, episodePickerVisible, episodePickerContext, removeEpisode, setEpisodePosition, editEpisodeTitle, editArchive, resetOrder, saveOrder, closeSection, closeManager, toggleSeason, toggleSection, refreshSeasons }',
        context
    )
    return {
        props,
        auth,
        calls,
        events,
        messages,
        box,
        manager: context.manager,
        dispose() {
            unmount()
            stops.forEach(stop => stop())
        }
    }
}

test('标题后台补全最多四并发，列表立即可操作，失败不阻塞其余稿件', async context => {
    const requests = []
    const h = harness({
        section: async () => ({ section: { id: 11 }, episodes: Array.from({ length: 9 }, (_, i) => ({ id: i + 1, aid: i + 101, title: '单集' })) }),
        archive: (uid, aid) => {
            const response = deferred()
            requests.push({ uid, aid, ...response })
            return response.promise
        }
    })
    context.after(() => h.dispose())
    await h.manager.openSection(entry(), entry().sections.sections[0])
    assert.equal(h.manager.sectionLoading.value, false)
    assert.equal(h.manager.episodes.value.length, 9)
    assert.equal(requests.length, 4)
    assert.equal(h.manager.episodes.value[0].archiveTitleState, 'loading')
    h.manager.setEpisodePosition(0, 2)
    assert.equal(h.manager.episodes.value[1].aid, 101)
    requests[0].resolve({ archiveTitle: '稿件一', bvid: 'BVone' })
    requests[1].reject(new Error('单项失败'))
    await settle()
    await settle()
    assert.equal(requests.length, 6)
    assert.equal(h.manager.episodes.value[1].archiveTitle, '稿件一')
    assert.equal(h.manager.episodes.value[0].archiveTitleState, 'error')
    assert.equal(h.manager.sectionError.value, '')
    await h.manager.closeSection(() => {})
    const dispatched = requests.length
    requests.slice(2).forEach(request => request.resolve({ archiveTitle: '旧标题' }))
    await settle()
    await settle()
    assert.equal(requests.length, dispatched)
    assert.equal(h.manager.episodes.value.length, 0)
})

test('标题缓存复用并按账号隔离，手动刷新重新查询，旧结果不覆盖新小节', async context => {
    const requests = []
    const h = harness({ archive: (uid, aid) => {
        const response = deferred()
        requests.push({ uid, aid, ...response })
        return response.promise
    } })
    context.after(() => h.dispose())
    await h.manager.openSection(entry(), entry().sections.sections[0])
    requests.forEach(request => request.resolve({ archiveTitle: '缓存标题' }))
    await settle()
    await settle()
    await h.manager.loadSection()
    assert.equal(requests.length, 2)
    assert.equal(h.manager.episodes.value[0].archiveTitle, '缓存标题')
    await h.manager.loadSection(true)
    assert.equal(requests.length, 4)
    await h.manager.closeSection(() => {})
    await h.manager.openSection(entry(20), { id: 21, title: '新小节' })
    assert.equal(requests.length, 6)
    requests.slice(2, 4).forEach(request => request.resolve({ archiveTitle: '过期标题' }))
    requests.slice(4).forEach(request => request.resolve({ archiveTitle: '新标题' }))
    await settle()
    await settle()
    assert.equal(h.manager.episodes.value[0].archiveTitle, '新标题')
    h.manager.uid.value = 2
    await settle()
    await h.manager.openSection(entry(), entry().sections.sections[0])
    assert.equal(requests.length, 8)
    assert.equal(requests.at(-1).uid, 2)
    assert.equal(h.manager.episodes.value[0].archiveTitleState, 'loading')
})

test('合集分组按 no_section 区分，默认小节不误显示为分组', context => {
    const h = harness({ props: { modelValue: false } })
    context.after(() => h.dispose())
    assert.equal(h.manager.hasSections(entry(10, 1)), false)
    assert.equal(h.manager.hasSections(entry(10, 0)), true)
    const unknown = entry()
    delete unknown.season.no_section
    assert.equal(h.manager.hasSections(unknown), false)
    unknown.sections.sections.push({ id: 12, title: '花絮' })
    assert.equal(h.manager.hasSections(unknown), true)
    assert.equal(h.calls.length, 0)
})

test('切换账号和关闭管理弹窗后，旧列表请求不覆盖当前状态', async context => {
    const responses = []
    const h = harness({
        list: account => {
            const response = deferred()
            responses.push({ account, ...response })
            return response.promise
        }
    })
    context.after(() => h.dispose())
    h.manager.uid.value = 2
    await nextTick()
    assert.deepEqual(
        responses.map(response => response.account),
        [1, 2]
    )
    responses[0].resolve([entry(1)])
    await settle()
    assert.equal(h.manager.seasons.value.length, 0)
    assert.equal(h.manager.loading.value, true)
    responses[1].resolve([entry(2)])
    await settle()
    assert.equal(h.manager.seasons.value[0].season.id, 2)
    const pending = h.manager.loadSeasons()
    h.props.modelValue = false
    await nextTick()
    responses[2].reject(new Error('旧请求失败'))
    await pending
    assert.equal(h.manager.listError.value, '')
    assert.equal(h.manager.seasons.value.length, 0)
})

test('添加、移出与排序使用正确账号、合集、小节及 episode ID', async context => {
    const h = harness()
    context.after(() => h.dispose())
    await settle()
    await h.manager.openSection(entry(), entry().sections.sections[0])
    await h.manager.addEpisode()
    assert.equal(h.manager.episodePickerVisible.value, true)
    assert.equal(h.calls.some(call => call[0] === 'add'), false)
    await h.manager.selectEpisodes(['BVtest'], [123])
    assert.equal(JSON.stringify(h.calls.find(call => call[0] === 'add')),
        JSON.stringify(['add', 1, 10, 11, ['BVtest']]))
    assert.equal(h.manager.episodes.value.length, 3)
    assert.equal(JSON.stringify(h.calls.find(call => call[0] === 'sync')),
        JSON.stringify(['sync', 1, [123], { season_id: 10, section_id: 11 }]))
    await h.manager.removeEpisode(h.manager.episodes.value[0])
    assert.equal(JSON.stringify(h.calls.filter(call => call[0] === 'sync').at(-1)),
        JSON.stringify(['sync', 1, [121], { season_id: 0, section_id: 0 }]))
    assert.deepEqual(
        h.calls.find(call => call[0] === 'remove'),
        ['remove', 1, 10, 11, 21]
    )
    h.manager.setEpisodePosition(1, 1)
    assert.equal(h.manager.orderDirty.value, true)
    await h.manager.saveOrder()
    const reordered = h.calls.find(call => call[0] === 'reorder')
    assert.equal(JSON.stringify(reordered), JSON.stringify(['reorder', 1, 10, 11, [23, 22]]))
    assert.equal(h.manager.orderDirty.value, false)
    assert.equal(h.messages.filter(([, type]) => type === 'success').length, 3)
})

test('未保存排序可取消或放弃，提交期间不能关闭弹窗', async context => {
    const response = deferred()
    const h = harness({
        reorder: () => response.promise,
        confirm: async () => {
            throw 'cancel'
        }
    })
    context.after(() => h.dispose())
    await h.manager.openSection(entry(), entry().sections.sections[0])
    h.manager.setEpisodePosition(1, 1)
    let closed = false
    await h.manager.closeSection(() => {
        closed = true
    })
    assert.equal(closed, false)
    assert.equal(h.manager.orderDirty.value, true)
    const saving = h.manager.saveOrder()
    assert.equal(h.manager.busy.value, true)
    h.manager.closeManager(() => {
        closed = true
    })
    await h.manager.closeSection(() => {
        closed = true
    })
    assert.equal(closed, false)
    response.resolve()
    await saving
    assert.equal(h.manager.busy.value, false)
    h.manager.setEpisodePosition(1, 1)
    h.box.confirm = async () => {}
    await h.manager.closeSection(() => {
        closed = true
    })
    assert.equal(closed, true)
    assert.equal(h.manager.orderDirty.value, false)
})

test('关闭小节期间返回的结果不会更新稿件列表，错误后可刷新', async context => {
    const responses = []
    const h = harness({
        section: () => {
            const response = deferred()
            responses.push(response)
            return response.promise
        }
    })
    context.after(() => h.dispose())
    const first = h.manager.openSection(entry(), entry().sections.sections[0])
    h.manager.sectionVisible.value = false
    responses[0].resolve(initialDetail())
    await first
    assert.equal(h.manager.episodes.value.length, 0)
    const second = h.manager.openSection(entry(), entry().sections.sections[0])
    responses[1].reject(new Error('网络异常'))
    await second
    assert.match(h.manager.sectionError.value, /网络异常/)
    const retry = h.manager.loadSection()
    responses[2].resolve(initialDetail())
    await retry
    assert.equal(h.manager.sectionError.value, '')
    assert.equal(h.manager.episodes.value.length, 2)
})

test('合集向下展开，折叠期间的旧稿件请求不覆盖新小节', async context => {
    const responses = []
    const h = harness({
        section: () => {
            const response = deferred()
            responses.push(response)
            return response.promise
        }
    })
    context.after(() => h.dispose())
    await h.manager.toggleSeason(entry())
    assert.equal(h.manager.expandedSeasonId.value, 10)
    assert.equal(responses.length, 1)
    await h.manager.toggleSeason(entry())
    assert.equal(h.manager.expandedSeasonId.value, null)
    assert.equal(h.manager.sectionVisible.value, false)
    await h.manager.toggleSeason(entry(20, 0))
    assert.equal(h.manager.expandedSeasonId.value, 20)
    assert.equal(responses.length, 1)
    const second = h.manager.toggleSection(entry(20, 0), { id: 12, title: '花絮' })
    responses[1].resolve(initialDetail())
    await second
    await settle()
    responses[0].resolve(initialDetail())
    await settle()
    assert.equal(h.manager.target.value.sectionId, 12)
    assert.equal(h.manager.episodes.value.length, 2)
})

test('展开其他合集前确认未保存排序，取消时保留当前列表', async context => {
    const h = harness({
        confirm: async () => {
            throw 'cancel'
        }
    })
    context.after(() => h.dispose())
    await h.manager.toggleSeason(entry())
    await settle()
    h.manager.setEpisodePosition(1, 1)
    await h.manager.toggleSeason(entry(20, 0))
    assert.equal(h.manager.expandedSeasonId.value, 10)
    assert.equal(h.manager.target.value.sectionId, 11)
    assert.equal(h.manager.orderDirty.value, true)
    h.box.confirm = async () => {}
    await h.manager.toggleSeason(entry(20, 0))
    assert.equal(h.manager.expandedSeasonId.value, 20)
    assert.equal(h.manager.sectionVisible.value, false)
    assert.equal(h.manager.episodes.value.length, 0)
})

test('编辑序号移动稿件并保存完整顺序，非法序号不影响列表', async context => {
    const h = harness()
    context.after(() => h.dispose())
    await h.manager.openSection(entry(), entry().sections.sections[0])
    await h.manager.addEpisode()
    await h.manager.selectEpisodes(['BVtest'], [123])
    for (const invalid of [undefined, NaN, 0, 4, 1.5]) h.manager.setEpisodePosition(0, invalid)
    assert.deepEqual(Array.from(h.manager.episodes.value, row => row.id), [21, 22, 23])
    h.manager.setEpisodePosition(0, 3)
    assert.deepEqual(Array.from(h.manager.episodes.value, row => row.id), [22, 23, 21])
    await h.manager.saveOrder()
    assert.deepEqual(Array.from(h.calls.find(call => call[0] === 'reorder')[4]), [22, 23, 21])
    assert.equal(h.manager.orderDirty.value, false)
})

test('长合集切换后按更新后的布局定位新标题，取消切换和折叠不跳动', async context => {
    const scrolls = []
    const wrap = {
        scrollTop: 3000,
        clientTop: 2,
        getBoundingClientRect: () => ({ top: 100 }),
        querySelector: selector => {
            const id = Number(selector.match(/data-season-id="(\d+)"/)[1])
            const offset = id === 10 ? 120 : 220
            return { getBoundingClientRect: () => ({ top: 102 + offset - wrap.scrollTop }) }
        }
    }
    const h = harness({ afterLayout: () => { wrap.scrollTop = 500 } })
    context.after(() => h.dispose())
    h.manager.seasonScrollbar.value = { wrapRef: wrap, setScrollTop: top => scrolls.push(top) }
    await h.manager.toggleSeason(entry(10, 0))
    await settle()
    await h.manager.toggleSeason(entry(20, 0))
    await settle()
    assert.deepEqual(scrolls, [120, 220])
    await h.manager.toggleSeason(entry(20, 0))
    await settle()
    assert.equal(scrolls.length, 2)
    await h.manager.toggleSeason(entry())
    await settle()
    h.manager.setEpisodePosition(0, 2)
    h.box.confirm = async () => { throw 'cancel' }
    const previous = scrolls.length
    await h.manager.toggleSeason(entry(20, 0))
    await settle()
    assert.equal(scrolls.length, previous)
    assert.equal(h.manager.expandedSeasonId.value, 10)
})

test('快速切换时只定位最后展开的合集，关闭后不执行过期定位', async context => {
    const scrolls = []
    const h = harness()
    context.after(() => h.dispose())
    const wrap = {
        scrollTop: 0, clientTop: 0,
        getBoundingClientRect: () => ({ top: 100 }),
        querySelector: selector => {
            scrolls.push(selector)
            return { getBoundingClientRect: () => ({ top: 150 }) }
        }
    }
    h.manager.seasonScrollbar.value = { wrapRef: wrap, setScrollTop: () => {} }
    await Promise.all([h.manager.toggleSeason(entry(10, 0)), h.manager.toggleSeason(entry(20, 0))])
    await settle()
    assert.deepEqual(scrolls, ['[data-season-id="20"] > .season-heading'])
    const pending = h.manager.toggleSeason(entry(30, 0))
    h.props.modelValue = false
    await pending
    await settle()
    assert.equal(scrolls.length, 1)
})

test('修改单集标题使用 episode ID，不改变稿件标题与顺序；失败保留原值', async context => {
    const h = harness({ prompt: async () => ({ value: ' 新单集标题 ' }) })
    context.after(() => h.dispose())
    await h.manager.openSection(entry(), entry().sections.sections[0])
    await h.manager.editEpisodeTitle(h.manager.episodes.value[0])
    assert.deepEqual(h.calls.find(call => call[0] === 'title'), ['title', 1, 10, 11, 21, '新单集标题'])
    assert.deepEqual(Array.from(h.manager.episodes.value, row => row.id), [21, 22])
    assert.equal(h.manager.episodes.value[0].title, '新单集标题')
    h.manager.setEpisodePosition(0, 2)
    await h.manager.editEpisodeTitle(h.manager.episodes.value[0])
    assert.equal(h.calls.filter(call => call[0] === 'title').length, 1)
    const failed = harness({ title: async () => { throw new Error('保存失败') } })
    context.after(() => failed.dispose())
    await failed.manager.openSection(entry(), entry().sections.sections[0])
    await failed.manager.editEpisodeTitle(failed.manager.episodes.value[0])
    assert.equal(failed.manager.episodes.value[0].title, '稿件一')
    assert.equal(failed.messages.at(-1)[1], 'error')
})

test('编辑稿件复用父级编辑入口，成功关闭管理窗口，失败时保留列表', async context => {
    const calls = []
    const h = harness({ props: { openArchiveEditor: async (...args) => calls.push(args) } })
    context.after(() => h.dispose())
    await h.manager.openSection(entry(), entry().sections.sections[0])
    await h.manager.editArchive(h.manager.episodes.value[0])
    assert.equal(calls[0][0], 1)
    assert.equal(calls[0][1].aid, 121)
    assert.deepEqual(h.events.at(-1), ['update:modelValue', false])
    h.events.length = 0
    h.props.openArchiveEditor = async () => { throw new Error('加载失败') }
    await h.manager.editArchive(h.manager.episodes.value[0])
    assert.equal(h.events.length, 0)
    assert.equal(h.manager.episodes.value.length, 2)
    assert.equal(h.manager.busy.value, false)
})

test('管理界面文本覆盖所有语言', context => {
    const h = harness({ props: { modelValue: false } })
    context.after(() => h.dispose())
    const rows = readFileSync(
        new URL('../src/components/SeasonEpisodeRows.vue', import.meta.url),
        'utf8'
    )
    const keys = [...(component + rows).matchAll(/\bt\('([^']+)'/g)].map(match => match[1])
    for (const language of ['en-US', 'ja-JP', 'zh-TW']) {
        const catalog = JSON.parse(
            readFileSync(new URL(`../src/i18n/${language}.json`, import.meta.url), 'utf8')
        )
        for (const key of keys) assert.ok(catalog[key], `${language}: ${key}`)
    }
})

test('添加单集取消不提交，切换小节或账号后不能添加到旧目标', async context => {
    const h = harness()
    context.after(() => h.dispose())
    await h.manager.openSection(entry(), entry().sections.sections[0])
    await h.manager.addEpisode()
    h.manager.episodePickerVisible.value = false
    await h.manager.selectEpisodes(['BVtest'], [123])
    assert.equal(h.calls.some(call => call[0] === 'add'), false)
    await h.manager.addEpisode()
    await h.manager.closeSection(() => {})
    assert.equal(h.manager.episodePickerVisible.value, false)
    await h.manager.selectEpisodes(['BVtest'], [123])
    assert.equal(h.calls.some(call => call[0] === 'add'), false)
    await h.manager.openSection(entry(), entry().sections.sections[0])
    await h.manager.addEpisode()
    h.manager.uid.value = 2
    await h.manager.selectEpisodes(['BVtest'], [123])
    await nextTick()
    assert.equal(h.manager.episodePickerVisible.value, false)
    assert.equal(h.calls.some(call => call[0] === 'add'), false)
})

test('多选稿件只发一次批量命令，失败后仍刷新远端结果', async context => {
    const h = harness({ add: async (...args) => {
        if (args[3].includes('BVfail')) throw new Error('添加失败')
    } })
    context.after(() => h.dispose())
    await h.manager.openSection(entry(), entry().sections.sections[0])
    await h.manager.addEpisode()
    await h.manager.selectEpisodes(['BVone', 'BVtwo', 'BVone'], [121, 122])
    assert.equal(h.calls.filter(call => call[0] === 'add').length, 1)
    assert.equal(JSON.stringify(h.calls.find(call => call[0] === 'add')[4]), JSON.stringify(['BVone', 'BVtwo']))
    assert.equal(h.manager.episodes.value.length, 4)
    assert.equal(h.manager.episodePickerVisible.value, false)
    await h.manager.addEpisode()
    await h.manager.selectEpisodes(['BVthree', 'BVfail', 'BVfour'], [121, 122])
    assert.equal(h.manager.episodes.value.length, 4)
    assert.equal(h.calls.filter(call => call[0] === 'add').length, 2)
    assert.equal(h.calls.at(-2)[0], 'section')
    assert.equal(h.messages.at(-1)[1], 'error')
    assert.equal(h.manager.busy.value, false)
})

test('稿件编辑复用已有模板，名称冲突时新建独立模板，加载失败不报告成功', async () => {
    const handler = setupMethod(readFileSync(new URL('../src/views/MainView.vue', import.meta.url), 'utf8'), 'openArchiveEditor')
    const user = { uid: 1 }
    let queries = 0
    const templates = { existing: { aid: 121 }, av122: { aid: 999 } }
    const calls = []
    const context = {
        templateLoading: ref(false), submitting: ref(false), getCurrentAutoSubmitting: ref(false),
        autoSubmittingRecord: ref({}),
        getTemplateKey: (uid, name) => `${uid}:${name}`,
        loginUsers: ref([user]),
        userConfigStore: {
            configRoot: { config: { 1: { templates } } },
            createTemplateFromVideo: async (...args) => calls.push(['create', ...args]),
            syncVideoSeason: (...args) => { Object.assign(templates.existing, args[2]) }
        },
        utilsStore: { getVideoSeason: async () => { queries++; return { season_id: 10, section_id: 11 } } },
        selectTemplate: async (...args) => { calls.push(['select', ...args]); return true },
        t: (key, params) => key.replace(/\{(\w+)\}/g, (match, name) => params?.[name] ?? match)
    }
    vm.runInNewContext(handler + '\nglobalThis.edit = openArchiveEditor', context)
    await context.edit(1, { aid: 121 })
    assert.equal(templates.existing.season_id, 10)
    assert.equal(calls.length, 1)
    assert.deepEqual(calls[0], ['select', context.loginUsers.value[0], 'existing', true])
    await context.edit(1, { aid: 122 })
    assert.deepEqual(calls[1], ['create', 1, 'av122', 'av122 (2)', true])
    assert.equal(calls[2][2], 'av122 (2)')
    assert.equal(calls[2][3], false)
    assert.equal(queries, 1)
    assert.equal(templates.av122.aid, 999)
    await context.edit(1, { aid: 121, title: '测试稿件' }, 'copy')
    assert.deepEqual(calls[3], ['create', 1, 'av121', '测试稿件的复制', false])
    assert.equal(calls[4][2], '测试稿件的复制')
    templates['测试稿件的复制'] = { aid: undefined }
    await context.edit(1, { aid: 121, title: '测试稿件' }, 'copy')
    assert.deepEqual(calls[5], ['create', 1, 'av121', '测试稿件的复制 (2)', false])
    context.selectTemplate = async () => false
    await assert.rejects(context.edit(1, { aid: 121 }), /切换模板失败/)
    context.templateLoading.value = true
    await assert.rejects(context.edit(1, { aid: 121 }), /稍后再编辑/)
    context.templateLoading.value = false
    context.utilsStore.getVideoSeason = async () => { throw new Error('查询归属失败') }
    const beforeFailure = structuredClone(templates.existing)
    await assert.rejects(context.edit(1, { aid: 121 }), /查询归属失败/)
    assert.deepEqual(templates.existing, beforeFailure)
    context.autoSubmittingRecord.value['1:existing'] = true
    await assert.rejects(context.edit(1, { aid: 121 }), /稍后再编辑/)
})

test('拖动手柄提交目标序号并先还原 DOM，加载和提交期间禁止拖动', async context => {
    const body = {
        children: ['a', 'b', 'c'].map(id => ({ id, matches: selector => selector === '.el-table__row' })),
        insertBefore(item, target) {
            this.children.splice(this.children.indexOf(item), 1)
            const index = target === null ? this.children.length : this.children.indexOf(target)
            this.children.splice(index, 0, item)
        }
    }
    const setup = compileSetup(readFileSync(new URL('../src/components/SeasonEpisodeRows.vue', import.meta.url), 'utf8'))
    const props = reactive({ busy: false, loading: false, ready: true, episodes: [{ id: 1 }, { id: 2 }, { id: 3 }] })
    const events = [], options = []
    let mounted, unmount, settings, destroyed = false
    const sandbox = {
        ref, nextTick, defineProps: () => props,
        defineEmits: () => (...args) => events.push(args),
        onMounted: callback => { mounted = callback },
        onBeforeUnmount: callback => { unmount = callback },
        watch: (...args) => { const stop = watch(...args); context.after(stop) },
        Sortable: { create: (body, config) => {
            settings = config
            assert.equal(body.children.length, 3)
            return { option: (...args) => options.push(args), destroy: () => { destroyed = true } }
        } }
    }
    vm.runInNewContext(setup + '\nglobalThis.element = root', sandbox)
    sandbox.element.value = { querySelector: () => body }
    await mounted()
    assert.equal(settings.handle, '.episode-drag-handle')
    assert.equal(settings.forceFallback, true)
    assert.equal(settings.fallbackOnBody, true)
    assert.equal(settings.fallbackTolerance, 4)
    assert.equal(settings.disabled, false)
    const ids = () => Array.from(body.children, row => row.id)
    const drag = (from, to) => {
        const item = body.children[from]
        const remaining = Array.from(body.children).filter(row => row !== item)
        body.insertBefore(item, remaining[to] || null)
        settings.onEnd({ from: body, item, oldDraggableIndex: from, newDraggableIndex: to })
    }
    drag(0, 2)
    assert.deepEqual(ids(), ['a', 'b', 'c'])
    assert.deepEqual(events.pop(), ['position', 0, 3])
    drag(2, 0)
    assert.deepEqual(ids(), ['a', 'b', 'c'])
    assert.deepEqual(events.pop(), ['position', 2, 1])
    for (const flag of ['loading', 'busy']) {
        props[flag] = true
        await nextTick()
        assert.deepEqual(options.at(-1), ['disabled', true])
        drag(0, 1)
        assert.equal(events.length, 0)
        props[flag] = false
        await nextTick()
        assert.deepEqual(options.at(-1), ['disabled', false])
    }
    unmount()
    assert.equal(destroyed, true)
})


test('移出已在服务端执行但响应丢失时仍刷新列表，不自动重发操作', async context => {
    const h = harness({ remove: async () => { throw new Error('响应丢失') } })
    context.after(() => h.dispose())
    await h.manager.openSection(entry(), entry().sections.sections[0])
    await h.manager.removeEpisode(h.manager.episodes.value[0])
    assert.deepEqual(Array.from(h.manager.episodes.value, row => row.aid), [122])
    assert.equal(h.calls.filter(call => call[0] === 'remove').length, 1)
    assert.equal(h.messages.at(-1)[1], 'error')
})


test('排序和改标题只刷新当前小节，添加和移出同步刷新合集列表', async context => {
    const h = harness({ prompt: async () => ({ value: '新标题' }) })
    context.after(() => h.dispose())
    await settle()
    await h.manager.openSection(entry(), entry().sections.sections[0])
    h.calls.length = 0
    await h.manager.editEpisodeTitle(h.manager.episodes.value[0])
    assert.equal(h.calls.filter(([name]) => name === 'list').length, 0)
    assert.equal(h.calls.filter(([name]) => name === 'section').length, 1)
    h.calls.length = 0
    h.manager.setEpisodePosition(0, 2)
    await h.manager.saveOrder()
    assert.equal(h.calls.filter(([name]) => name === 'list').length, 0)
    assert.equal(h.calls.filter(([name]) => name === 'section').length, 1)
    h.calls.length = 0
    await h.manager.addEpisode()
    await h.manager.selectEpisodes(['BVnew'], [123])
    assert.equal(h.calls.filter(([name]) => name === 'list').length, 1)
    h.calls.length = 0
    await h.manager.removeEpisode(h.manager.episodes.value[0])
    assert.equal(h.calls.filter(([name]) => name === 'list').length, 1)
})

test('新建模板选择时跳过重复远端检查，已有模板继续安排检查', async () => {
    const handler = setupMethod(readFileSync(new URL('../src/views/MainView.vue', import.meta.url), 'utf8'), 'selectTemplate')
    const timers = []
    const context = {
        templateLoading: ref(false), selectedUser: ref(null), currentTemplateName: ref(''),
        currentTemplate: ref({ aid: 123 }), lastSubmit: ref(''),
        utilsStore: { initArchievePre: async () => {}, initTopicList: async () => {} },
        nextTick: () => {}, loadTemplate: async () => {}, saveTemplateSelection: () => {},
        setTimeout: callback => timers.push(callback), console: { log: () => {}, error: () => {} }, t: key => key
    }
    vm.runInNewContext(handler + '\nglobalThis.select = selectTemplate', context)
    assert.equal(await context.select({ uid: 1 }, '新模板', false), true)
    assert.equal(timers.length, 0)
    assert.equal(await context.select({ uid: 1 }, '已有模板'), true)
    assert.equal(timers.length, 1)
})
