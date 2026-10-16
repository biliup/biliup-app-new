<template>
    <el-dialog
        :model-value="modelValue"
        :title="seasonAddOnly ? t('添加单集') : t('稿件管理')"
        width="760px"
        style="max-width: calc(100vw - 32px)"
        append-to-body
        :close-on-click-modal="!actionLoading"
        :close-on-press-escape="!actionLoading"
        :show-close="!actionLoading"
        @update:model-value="emit('update:modelValue', $event)"
    >
        <div class="picker-toolbar">
            <span v-if="seasonAddOnly">{{ username }}</span>
            <el-select
                v-else
                :model-value="uid"
                :placeholder="t('请选择用户')"
                :disabled="actionLoading"
                @update:model-value="emit('update:uid', $event)"
            >
                <el-option
                    v-for="user in users"
                    :key="user.uid"
                    :label="user.username"
                    :value="user.uid"
                />
            </el-select>
            <el-button :disabled="loading || actionLoading || !uid" @click="loadPage(page, true)">{{
                t('刷新')
            }}</el-button>
        </div>
        <el-alert v-if="error" :title="error" type="error" :closable="false" show-icon />
        <el-table
            v-loading="loading || actionLoading"
            :data="items"
            row-key="aid"
            :max-height="420"
            :empty-text="
                loading
                    ? t('加载中...')
                    : error
                      ? t('请点击刷新重试')
                      : seasonAddOnly
                        ? t('当前页没有可添加的稿件')
                        : t('暂无稿件')
            "
            @selection-change="updateSelection"
        >
            <el-table-column
                v-if="seasonAddOnly"
                type="selection"
                width="48"
                :selectable="canSelectVideo"
            />
            <el-table-column :label="t('封面')" width="112">
                <template #default="{ row }">
                    <el-image
                        v-if="row.cover"
                        :src="'data:image/jpeg;base64,' + row.cover"
                        fit="cover"
                        class="video-cover"
                    >
                        <template #error
                            ><span>{{ t('暂无封面') }}</span></template
                        >
                    </el-image>
                </template>
            </el-table-column>
            <el-table-column :label="t('视频标题')" min-width="220">
                <template #default="{ row }">
                    <div class="video-title" :title="row.title">{{ row.title }}</div>
                    <div class="video-id">{{ row.bvid || `av${row.aid}` }}</div>
                </template>
            </el-table-column>
            <el-table-column v-if="!seasonAddOnly" :label="t('操作')" width="170" align="center">
                <template #default="{ row }">
                    <el-button
                        type="primary"
                        link
                        :disabled="loading || actionLoading"
                        @click="selectVideo(row, 'edit')"
                    >
                        {{ t('编辑') }}
                    </el-button>
                    <el-button
                        type="primary"
                        link
                        :disabled="loading || actionLoading"
                        @click="selectVideo(row, 'copy')"
                        >{{ t('复制') }}</el-button
                    >
                    <el-button
                        type="danger"
                        link
                        :disabled="loading || actionLoading"
                        @click="deleteVideo(row)"
                        >{{ t('删除') }}</el-button
                    >
                </template>
            </el-table-column>
        </el-table>
        <template #footer>
            <div class="picker-footer">
                <el-pagination
                    :current-page="page"
                    :page-size="pageSize"
                    :total="total"
                    :disabled="loading || actionLoading"
                    layout="total, prev, pager, next"
                    @current-change="loadPage"
                />
                <el-button :disabled="actionLoading" @click="emit('update:modelValue', false)">{{
                    seasonAddOnly ? t('取消') : t('关闭')
                }}</el-button>
                <el-button
                    v-if="seasonAddOnly"
                    type="primary"
                    :disabled="loading || !selectedVideos.length"
                    @click="confirmSelection"
                    >{{ t('确定') }}</el-button
                >
            </div>
        </template>
    </el-dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch, onBeforeUnmount } from 'vue'
import { ElMessageBox } from 'element-plus'
import { t } from '../i18n'
import { useUtilsStore } from '../stores/utils'
import type { CreatorVideo } from '../types/creator_video'

const props = defineProps<{
    modelValue: boolean
    uid: number | null
    username: string
    seasonAddOnly?: boolean
    users?: { uid: number; username: string }[]
    manageVideo?: (uid: number, video: CreatorVideo, action: 'edit' | 'copy') => Promise<void>
}>()
const emit = defineEmits<{
    'update:modelValue': [value: boolean]
    'update:uid': [uid: number]
    'select-many': [videoIds: string[], aids: number[]]
}>()
const utilsStore = useUtilsStore()
const items = ref<CreatorVideo[]>([])
const loading = ref(false)
const actionLoading = ref(false)
const error = ref('')
const page = ref(1)
const total = ref(0)
const pageSize = computed(() => (props.seasonAddOnly ? 5 : 10))
const selectedVideos = ref<CreatorVideo[]>([])
let requestId = 0
let snapshot: { uid: number; pending: Promise<string> } | null = null

const releaseSnapshot = () => {
    const previous = snapshot
    snapshot = null
    if (previous) {
        void previous.pending
            .then(id => utilsStore.closeVideoListSnapshot(previous.uid, id))
            .catch(() => {})
    }
}

const loadPage = async (nextPage: number, refresh = false) => {
    const uid = props.uid
    if (!props.modelValue || !uid || actionLoading.value) return
    if (refresh) releaseSnapshot()
    const request = ++requestId
    const seasonAddOnly = props.seasonAddOnly
    page.value = nextPage
    items.value = []
    selectedVideos.value = []
    loading.value = true
    error.value = ''
    try {
        let snapshotId: string | undefined
        if (seasonAddOnly) {
            if (snapshot && snapshot.uid !== uid) releaseSnapshot()
            snapshot ??= { uid, pending: utilsStore.openVideoListSnapshot(uid) }
            snapshotId = await snapshot.pending
            if (request !== requestId || !props.modelValue || props.uid !== uid) return
        }
        const result = seasonAddOnly
            ? await utilsStore.getVideoList(uid, nextPage, pageSize.value, true, snapshotId)
            : await utilsStore.getVideoList(uid, nextPage, pageSize.value)
        if (
            request !== requestId ||
            !props.modelValue ||
            props.uid !== uid ||
            props.seasonAddOnly !== seasonAddOnly
        )
            return
        items.value = result.items
        total.value = result.total
    } catch (cause) {
        if (request === requestId && props.modelValue && props.uid === uid) {
            error.value = t('获取稿件列表失败: {0}', { 0: String(cause) })
        }
    } finally {
        if (request === requestId) loading.value = false
    }
}

const selectVideo = async (video: CreatorVideo, action: 'edit' | 'copy' = 'edit') => {
    if (
        loading.value ||
        actionLoading.value ||
        !props.modelValue ||
        !props.uid ||
        !items.value.includes(video)
    )
        return
    if (props.seasonAddOnly || !props.manageVideo) return
    const uid = props.uid
    const request = requestId
    actionLoading.value = true
    error.value = ''
    try {
        await props.manageVideo(uid, video, action)
        if (request === requestId && props.modelValue && props.uid === uid)
            emit('update:modelValue', false)
    } catch (cause) {
        if (request === requestId && props.modelValue && props.uid === uid)
            error.value = t('稿件操作失败: {0}', { 0: String(cause) })
    } finally {
        actionLoading.value = false
        if (props.modelValue && props.uid !== uid) void loadPage(1)
    }
}

const deleteVideo = async (video: CreatorVideo) => {
    if (
        loading.value ||
        actionLoading.value ||
        !props.modelValue ||
        !props.uid ||
        props.seasonAddOnly ||
        !items.value.includes(video)
    )
        return
    const uid = props.uid
    const request = requestId
    let attempted = false
    let deletionError = ''
    actionLoading.value = true
    error.value = ''
    try {
        await ElMessageBox.confirm(
            t('确定删除稿件“{0}”吗？删除后无法恢复，本地模板将保留。', { 0: video.title }),
            t('删除稿件'),
            { type: 'warning', confirmButtonText: t('删除'), cancelButtonText: t('取消') }
        )
        if (request !== requestId || !props.modelValue || props.uid !== uid) return
        attempted = true
        await utilsStore.deleteCreatorVideo(uid, video.aid)
        utilsStore.showMessage(t('稿件已删除，本地模板已保留'), 'success')
    } catch (cause) {
        if (cause !== 'cancel' && cause !== 'close')
            deletionError = t('稿件操作失败: {0}', { 0: String(cause) })
    } finally {
        actionLoading.value = false
        if (props.modelValue && props.uid !== uid) void loadPage(1)
        else if (attempted && request === requestId && props.modelValue) {
            const refreshRequest = requestId + 1
            await loadPage(page.value, true)
            if (
                requestId === refreshRequest &&
                !error.value &&
                !items.value.length &&
                page.value > 1
            ) {
                await loadPage(Math.max(1, Math.ceil(total.value / pageSize.value)))
            }
        }
        if (props.modelValue && props.uid === uid && deletionError)
            error.value = deletionError + (error.value ? `\n${error.value}` : '')
    }
}

const canSelectVideo = (video: CreatorVideo) => !loading.value && video.season_add_state === 0

const updateSelection = (videos: CreatorVideo[]) => {
    if (!props.seasonAddOnly || loading.value) return
    selectedVideos.value = videos.filter(
        video => items.value.includes(video) && canSelectVideo(video)
    )
}

const confirmSelection = () => {
    if (loading.value || !props.modelValue || !props.uid || !props.seasonAddOnly) return
    const selected = new Set(selectedVideos.value.map(video => video.aid))
    const videos = items.value.filter(video => selected.has(video.aid) && canSelectVideo(video))
    if (!videos.length) return
    emit(
        'select-many',
        videos.map(video => video.bvid || `av${video.aid}`),
        videos.map(video => video.aid)
    )
    emit('update:modelValue', false)
}

watch(
    () => [props.modelValue, props.uid, props.seasonAddOnly] as const,
    ([visible, uid]) => {
        releaseSnapshot()
        requestId += 1
        items.value = []
        selectedVideos.value = []
        error.value = ''
        loading.value = false
        total.value = 0
        page.value = 1
        if (visible && uid) void loadPage(1)
    },
    { immediate: true }
)

onBeforeUnmount(() => {
    requestId += 1
    releaseSnapshot()
})
</script>

<style scoped>
.picker-toolbar,
.picker-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
}

.picker-toolbar {
    margin-bottom: 12px;
}

.video-cover {
    width: 96px;
    height: 60px;
}

.video-title {
    overflow-wrap: anywhere;
}

.video-id {
    color: var(--el-text-color-secondary);
    font-size: 12px;
}
</style>
