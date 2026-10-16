<template>
    <el-dialog
        :model-value="modelValue"
        :title="t('合集管理')"
        width="1000px"
        style="max-width: calc(100vw - 32px)"
        :before-close="closeManager"
        :close-on-click-modal="false"
        :close-on-press-escape="!busy"
        :show-close="!busy"
    >
        <div class="manager-toolbar">
            <el-select v-model="uid" :placeholder="t('请选择用户')" :disabled="busy || orderDirty">
                <el-option
                    v-for="user in authStore.loginUsers"
                    :key="user.uid"
                    :label="user.username"
                    :value="user.uid"
                />
            </el-select>
            <el-button :disabled="!uid || loading || busy || orderDirty" @click="refreshSeasons">{{
                t('刷新合集列表')
            }}</el-button>
        </div>
        <el-alert v-if="listError" :title="listError" type="error" :closable="false" show-icon />
        <el-scrollbar
            ref="seasonScrollbar"
            v-loading="loading"
            max-height="60vh"
            class="season-list"
        >
            <el-empty
                v-if="!loading && !listError && !seasons.length"
                :description="uid ? t('暂无合集') : t('请选择用户')"
            />
            <article
                v-for="entry in seasons"
                :key="entry.season.id"
                :data-season-id="entry.season.id"
                class="season-group"
            >
                <button
                    type="button"
                    class="season-heading"
                    :aria-expanded="expandedSeasonId === entry.season.id"
                    :disabled="busy"
                    @click="toggleSeason(entry)"
                >
                    <span
                        class="expand-arrow"
                        :class="{ expanded: expandedSeasonId === entry.season.id }"
                        aria-hidden="true"
                        >▸</span
                    >
                    <strong>{{ entry.season.title }}</strong>
                </button>
                <div v-if="expandedSeasonId === entry.season.id" class="season-content">
                    <div
                        v-for="section in entry.sections.sections"
                        :key="section.id"
                        class="section-group"
                    >
                        <button
                            v-if="hasSections(entry)"
                            type="button"
                            class="section-heading"
                            :aria-expanded="sectionVisible && target?.sectionId === section.id"
                            :disabled="busy"
                            @click="toggleSection(entry, section)"
                        >
                            <span
                                class="expand-arrow"
                                :class="{
                                    expanded: sectionVisible && target?.sectionId === section.id
                                }"
                                aria-hidden="true"
                                >▸</span
                            >
                            <span>{{ section.title }}</span>
                        </button>
                        <SeasonEpisodeRows
                            v-if="sectionVisible && target?.sectionId === section.id"
                            :episodes="episodes"
                            :loading="sectionLoading"
                            :busy="busy"
                            :error="sectionError"
                            :dirty="orderDirty"
                            :ready="Boolean(detail)"
                            @add="addEpisode"
                            @refresh="loadSection(true)"
                            @save="saveOrder"
                            @reset="resetOrder"
                            @position="setEpisodePosition"
                            @title="editEpisodeTitle"
                            @edit="editArchive"
                            @remove="removeEpisode"
                        />
                    </div>
                    <el-empty v-if="!entry.sections.sections.length" :description="t('暂无稿件')" />
                </div>
            </article>
        </el-scrollbar>
        <template #footer>
            <el-button :disabled="busy" @click="closeManager(() => {})">{{ t('关闭') }}</el-button>
        </template>
        <CreatorVideoPicker
            v-model="episodePickerVisible"
            :uid="episodePickerContext?.uid ?? null"
            :username="
                authStore.loginUsers.find(user => user.uid === episodePickerContext?.uid)
                    ?.username || ''
            "
            season-add-only
            @select-many="selectEpisodes"
        />
    </el-dialog>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { ElMessageBox, type ElScrollbar } from 'element-plus'
import { t } from '../i18n'
import { useAuthStore } from '../stores/auth'
import { useUtilsStore } from '../stores/utils'
import { useUserConfigStore } from '../stores/user_config'
import SeasonEpisodeRows from './SeasonEpisodeRows.vue'
import CreatorVideoPicker from './CreatorVideoPicker.vue'
import type { ManagedSeason, ManagedSection, SeasonEpisode, SeasonSection } from '../types/season'

const props = defineProps<{
    modelValue: boolean
    initialUid?: number
    openArchiveEditor: (uid: number, episode: SeasonEpisode) => Promise<void>
}>()
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()
const authStore = useAuthStore()
const utilsStore = useUtilsStore()
const userConfigStore = useUserConfigStore()
const uid = ref<number>()
const seasons = ref<ManagedSeason[]>([])
const loading = ref(false)
const listError = ref('')
const expandedSeasonId = ref<number | null>(null)
const seasonScrollbar = ref<InstanceType<typeof ElScrollbar> | null>(null)
const sectionVisible = ref(false)
const sectionLoading = ref(false)
const sectionError = ref('')
const busy = ref(false)
const episodePickerVisible = ref(false)
const episodePickerContext = ref<{ uid: number; seasonId: number; sectionId: number } | null>(null)
const detail = ref<ManagedSection | null>(null)
const episodes = ref<SeasonEpisode[]>([])
const target = ref<{
    seasonId: number
    sectionId: number
} | null>(null)
let listRequest = 0
let sectionRequest = 0
const titleCache = new Map<string, Pick<SeasonEpisode, 'archiveTitle' | 'bvid'>>()
let titleQueue: { account: number; request: number; episode: SeasonEpisode }[] = []
let activeTitleRequests = 0

const pumpArchiveTitles = () => {
    while (activeTitleRequests < 4 && titleQueue.length) {
        const { account, request, episode } = titleQueue.shift()!
        if (request !== sectionRequest || !props.modelValue || uid.value !== account) continue
        const key = `${account}:${episode.aid}`
        const cached = titleCache.get(key)
        if (cached) {
            Object.assign(episode, cached)
            episode.archiveTitleState = undefined
            continue
        }
        activeTitleRequests += 1
        void utilsStore
            .getSeasonArchiveTitle(account, episode.aid)
            .then(result => {
                if (request !== sectionRequest || !props.modelValue || uid.value !== account) return
                titleCache.set(key, result)
                Object.assign(episode, result)
                episode.archiveTitleState = undefined
            })
            .catch(() => {
                if (request === sectionRequest && props.modelValue && uid.value === account)
                    episode.archiveTitleState = 'error'
            })
            .finally(() => {
                activeTitleRequests -= 1
                pumpArchiveTitles()
            })
    }
}

const hasSections = (entry: ManagedSeason) =>
    entry.season.no_section === undefined
        ? entry.sections.sections.length > 1
        : entry.season.no_section === 0

const orderDirty = computed(() =>
    episodes.value.some((episode, index) => episode.id !== detail.value?.episodes[index]?.id)
)

const loadSeasons = async () => {
    const account = uid.value
    if (!props.modelValue || !account) return
    const request = ++listRequest
    loading.value = true
    listError.value = ''
    try {
        const result = await utilsStore.listManagedSeasons(account)
        if (request === listRequest && props.modelValue && uid.value === account)
            seasons.value = result
    } catch (error) {
        if (request === listRequest)
            listError.value = t('获取合集列表失败: {0}', { 0: String(error) })
    } finally {
        if (request === listRequest) loading.value = false
    }
}

const loadSection = async (refreshTitles = false) => {
    const account = uid.value
    const context = target.value
    if (!props.modelValue || !sectionVisible.value || !account || !context) return
    const request = ++sectionRequest
    titleQueue = []
    sectionLoading.value = true
    sectionError.value = ''
    detail.value = null
    episodes.value = []
    try {
        const result = await utilsStore.getManagedSection(
            account,
            context.seasonId,
            context.sectionId
        )
        if (
            request !== sectionRequest ||
            !props.modelValue ||
            !sectionVisible.value ||
            uid.value !== account
        )
            return
        detail.value = result
        episodes.value = [...result.episodes]
        for (const episode of episodes.value) {
            const key = `${account}:${episode.aid}`
            if (refreshTitles) titleCache.delete(key)
            if (episode.archiveTitle) {
                titleCache.set(key, { archiveTitle: episode.archiveTitle, bvid: episode.bvid })
                continue
            }
            const cached = titleCache.get(key)
            if (cached) Object.assign(episode, cached)
            else {
                episode.archiveTitleState = 'loading'
                titleQueue.push({ account, request, episode })
            }
        }
        pumpArchiveTitles()
    } catch (error) {
        if (request === sectionRequest)
            sectionError.value = t('获取小节稿件失败: {0}', { 0: String(error) })
    } finally {
        if (request === sectionRequest) sectionLoading.value = false
    }
}

const openSection = async (entry: ManagedSeason, section?: SeasonSection) => {
    if (busy.value || !section) return
    expandedSeasonId.value = entry.season.id
    target.value = {
        seasonId: entry.season.id,
        sectionId: section.id
    }
    sectionVisible.value = true
    await loadSection()
}

const resetOrder = () => {
    episodes.value = [...(detail.value?.episodes || [])]
}

const setEpisodePosition = (index: number, position: number | undefined) => {
    if (
        busy.value ||
        sectionLoading.value ||
        !Number.isInteger(position) ||
        position === undefined ||
        position < 1 ||
        position > episodes.value.length ||
        index < 0 ||
        index >= episodes.value.length
    )
        return
    const list = [...episodes.value]
    const [episode] = list.splice(index, 1)
    list.splice(position - 1, 0, episode)
    episodes.value = list
}

const editEpisodeTitle = async (episode: SeasonEpisode) => {
    const account = uid.value
    const context = target.value
    if (!account || !context || busy.value || sectionLoading.value || orderDirty.value) return
    let title: string
    try {
        const result = await ElMessageBox.prompt(t('单集标题'), t('编辑单集标题'), {
            inputValue: episode.title,
            inputValidator: value => Boolean(value?.trim()) || t('请输入单集标题'),
            confirmButtonText: t('保存'),
            cancelButtonText: t('取消')
        })
        title = result.value.trim()
    } catch {
        return
    }
    if (
        title === episode.title ||
        uid.value !== account ||
        target.value !== context ||
        !sectionVisible.value ||
        orderDirty.value
    )
        return
    await runMutation(() =>
        utilsStore.editSeasonEpisodeTitle(
            account,
            context.seasonId,
            context.sectionId,
            episode.id,
            title
        )
    )
}

const editArchive = async (episode: SeasonEpisode) => {
    const account = uid.value
    if (!account || busy.value || sectionLoading.value || orderDirty.value) return
    busy.value = true
    try {
        await props.openArchiveEditor(account, episode)
        emit('update:modelValue', false)
    } catch (error) {
        utilsStore.showMessage(t('合集操作失败: {0}', { 0: String(error) }), 'error')
    } finally {
        busy.value = false
    }
}

const runMutation = async (action: () => Promise<void>, refreshOnError = false) => {
    if (busy.value) return
    busy.value = true
    try {
        await action()
        utilsStore.showMessage(t('合集操作成功'), 'success')
        await loadSection()
        await loadSeasons()
    } catch (error) {
        utilsStore.showMessage(t('合集操作失败: {0}', { 0: String(error) }), 'error')
        if (refreshOnError) {
            await loadSection()
            await loadSeasons()
        }
    } finally {
        busy.value = false
    }
}

const addEpisode = async () => {
    const account = uid.value
    const context = target.value
    if (!account || !context || busy.value || orderDirty.value || !detail.value) return
    episodePickerContext.value = {
        uid: account,
        seasonId: context.seasonId,
        sectionId: context.sectionId
    }
    episodePickerVisible.value = true
}

const selectEpisodes = async (videoIds: string[]) => {
    const context = episodePickerContext.value
    if (
        !videoIds.length ||
        !episodePickerVisible.value ||
        !context ||
        !props.modelValue ||
        !sectionVisible.value ||
        busy.value ||
        orderDirty.value ||
        sectionLoading.value ||
        !detail.value ||
        uid.value !== context.uid ||
        target.value?.seasonId !== context.seasonId ||
        target.value?.sectionId !== context.sectionId
    )
        return
    episodePickerVisible.value = false
    await runMutation(async () => {
        const aids = await utilsStore.addSeasonEpisodes(
            context.uid,
            context.seasonId,
            context.sectionId,
            [...new Set(videoIds)]
        )
        await userConfigStore.syncVideoSeason(context.uid, aids, {
            season_id: context.seasonId,
            section_id: context.sectionId
        })
    }, true)
}

const removeEpisode = async (episode: SeasonEpisode) => {
    const account = uid.value
    const context = target.value
    if (!account || !context || busy.value || orderDirty.value) return
    try {
        await ElMessageBox.confirm(
            t('从合集移出“{0}”？此操作不会删除稿件。', { 0: episode.title || t('无标题') }),
            t('移出合集'),
            {
                confirmButtonText: t('确定'),
                cancelButtonText: t('取消'),
                type: 'warning'
            }
        )
    } catch {
        return
    }
    if (uid.value !== account || target.value !== context || !sectionVisible.value) return
    await runMutation(async () => {
        await utilsStore.removeSeasonEpisode(
            account,
            context.seasonId,
            context.sectionId,
            episode.id
        )
        await userConfigStore.syncVideoSeason(account, [episode.aid], {
            season_id: 0,
            section_id: 0
        })
    })
}

const saveOrder = async () => {
    const account = uid.value
    const context = target.value
    if (!account || !context || !orderDirty.value || busy.value || sectionLoading.value) return
    const ids = episodes.value.map(episode => episode.id)
    await runMutation(() =>
        utilsStore.reorderSeasonEpisodes(account, context.seasonId, context.sectionId, ids)
    )
}

const closeSection = async (done: () => void) => {
    if (busy.value) return
    if (orderDirty.value) {
        try {
            await ElMessageBox.confirm(t('排序尚未保存，确定放弃更改吗？'), t('提示'), {
                confirmButtonText: t('确定'),
                cancelButtonText: t('取消'),
                type: 'warning'
            })
        } catch {
            return
        }
    }
    sectionRequest += 1
    titleQueue = []
    episodePickerVisible.value = false
    episodePickerContext.value = null
    sectionVisible.value = false
    sectionLoading.value = false
    detail.value = null
    episodes.value = []
    target.value = null
    done()
}

const scrollToSeason = async (seasonId: number) => {
    const account = uid.value
    await nextTick()
    if (!props.modelValue || uid.value !== account || expandedSeasonId.value !== seasonId) return
    const scrollbar = seasonScrollbar.value
    const wrap = scrollbar?.wrapRef
    const heading = wrap?.querySelector<HTMLElement>(
        `[data-season-id="${seasonId}"] > .season-heading`
    )
    if (!scrollbar || !wrap || !heading) return
    // 只调整合集列表，避免 scrollIntoView 同时滚动外层弹窗或主界面。
    scrollbar.setScrollTop(
        wrap.scrollTop +
            heading.getBoundingClientRect().top -
            wrap.getBoundingClientRect().top -
            wrap.clientTop
    )
}

const toggleSeason = async (entry: ManagedSeason) => {
    const collapsing = expandedSeasonId.value === entry.season.id
    await closeSection(() => {
        expandedSeasonId.value = collapsing ? null : entry.season.id
        if (!collapsing && !hasSections(entry)) void openSection(entry, entry.sections.sections[0])
        if (!collapsing) void scrollToSeason(entry.season.id)
    })
}

const toggleSection = async (entry: ManagedSeason, section: SeasonSection) => {
    const collapsing = sectionVisible.value && target.value?.sectionId === section.id
    await closeSection(() => {
        if (!collapsing) void openSection(entry, section)
    })
}

const refreshSeasons = async () => {
    await closeSection(() => {
        expandedSeasonId.value = null
        titleCache.clear()
        void loadSeasons()
    })
}

const closeManager = async (done: () => void) => {
    await closeSection(() => {
        emit('update:modelValue', false)
        done()
    })
}

watch(
    () => props.modelValue,
    visible => {
        if (visible) {
            const users = authStore.loginUsers
            uid.value =
                users.find(user => user.uid === props.initialUid)?.uid ??
                users.find(user => user.uid === uid.value)?.uid ??
                users[0]?.uid
        }
    },
    { immediate: true }
)

watch(
    () => authStore.loginUsers.map(user => user.uid),
    users => {
        if (!users.includes(uid.value || 0)) uid.value = users[0]
    }
)

watch(
    () => [props.modelValue, uid.value] as const,
    ([visible, account]) => {
        listRequest += 1
        sectionRequest += 1
        titleQueue = []
        episodePickerVisible.value = false
        episodePickerContext.value = null
        seasons.value = []
        listError.value = ''
        loading.value = false
        expandedSeasonId.value = null
        sectionVisible.value = false
        detail.value = null
        episodes.value = []
        target.value = null
        if (visible && account) void loadSeasons()
    },
    { immediate: true }
)

onBeforeUnmount(() => {
    listRequest += 1
    sectionRequest += 1
    titleQueue = []
})
</script>

<style scoped>
.manager-toolbar {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 16px;
}
.manager-toolbar .el-select {
    width: 220px;
}
.season-list {
    min-height: 180px;
}
.season-group {
    border-bottom: 1px solid var(--el-border-color-lighter);
}
.season-heading,
.section-heading {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 16px 8px;
    border: 0;
    background: transparent;
    color: var(--el-text-color-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
    overflow-wrap: anywhere;
}
.season-heading:hover,
.section-heading:hover {
    background: var(--el-fill-color-light);
}
.season-heading:disabled,
.section-heading:disabled {
    cursor: not-allowed;
}
.season-heading strong {
    font-size: 16px;
}
.season-content {
    padding: 0 8px 16px 26px;
}
.section-heading {
    padding: 12px 0;
}
.expand-arrow {
    flex-shrink: 0;
    transition: transform 0.2s;
}
.expand-arrow.expanded {
    transform: rotate(90deg);
}
</style>
