<template>
    <el-dialog
        :model-value="modelValue"
        :title="t('选择已有稿件')"
        width="760px"
        style="max-width: calc(100vw - 32px)"
        append-to-body
        @update:model-value="emit('update:modelValue', $event)"
    >
        <div class="picker-toolbar">
            <span>{{ username }}</span>
            <el-button :disabled="loading" @click="loadPage(page)">{{ t('刷新') }}</el-button>
        </div>
        <el-alert v-if="error" :title="error" type="error" :closable="false" show-icon />
        <el-table
            v-loading="loading"
            :data="items"
            row-key="aid"
            :max-height="420"
            :empty-text="loading ? t('加载中...') : error ? t('请点击刷新重试') : t('暂无稿件')"
        >
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
            <el-table-column :label="t('操作')" width="80" align="center">
                <template #default="{ row }">
                    <el-button type="primary" link :disabled="loading" @click="selectVideo(row)">
                        {{ t('选择') }}
                    </el-button>
                </template>
            </el-table-column>
        </el-table>
        <template #footer>
            <div class="picker-footer">
                <el-pagination
                    :current-page="page"
                    :page-size="pageSize"
                    :total="total"
                    :disabled="loading"
                    layout="total, prev, pager, next"
                    @current-change="loadPage"
                />
                <el-button @click="emit('update:modelValue', false)">{{ t('取消') }}</el-button>
            </div>
        </template>
    </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, onBeforeUnmount } from 'vue'
import { t } from '../i18n'
import { useUtilsStore } from '../stores/utils'
import type { CreatorVideo } from '../types/creator_video'

const props = defineProps<{
    modelValue: boolean
    uid: number | null
    username: string
}>()
const emit = defineEmits<{
    'update:modelValue': [value: boolean]
    select: [videoId: string]
}>()
const utilsStore = useUtilsStore()
const items = ref<CreatorVideo[]>([])
const loading = ref(false)
const error = ref('')
const page = ref(1)
const total = ref(0)
const pageSize = 10
let requestId = 0

const loadPage = async (nextPage: number) => {
    const uid = props.uid
    if (!props.modelValue || !uid) return
    const request = ++requestId
    page.value = nextPage
    items.value = []
    loading.value = true
    error.value = ''
    try {
        const result = await utilsStore.getVideoList(uid, nextPage, pageSize)
        if (request !== requestId || !props.modelValue || props.uid !== uid) return
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

const selectVideo = (video: CreatorVideo) => {
    if (loading.value || !props.modelValue || !props.uid || !items.value.includes(video)) return
    emit('select', video.bvid || `av${video.aid}`)
    emit('update:modelValue', false)
}

watch(
    () => [props.modelValue, props.uid] as const,
    ([visible, uid]) => {
        requestId += 1
        items.value = []
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
