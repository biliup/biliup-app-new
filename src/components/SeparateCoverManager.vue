<template>
    <el-drawer
        :model-value="modelValue"
        title="独立封面管理"
        size="560px"
        @update:model-value="emit('update:modelValue', $event)"
    >
        <div class="cover-manager-tip">
            独立封面仅用于当前多稿件提交，不会随模板保存；未单独设置时，将沿用基本信息中的公共封面。
        </div>

        <el-empty v-if="!videos.length" description="请先添加视频" />

        <div v-else class="cover-list">
            <div v-for="video in videos" :key="video.id" class="cover-item">
                <button
                    class="cover-preview"
                    type="button"
                    :disabled="Boolean(loading[video.id]) || disabled"
                    title="上传或更换独立封面"
                    @click="selectCover(video)"
                >
                    <img
                        v-if="getPreview(video)"
                        :src="getPreview(video)"
                        :alt="getVideoTitle(video)"
                    />
                    <el-icon v-else><Plus /></el-icon>
                    <span class="cover-preview-mask">
                        {{ loading[video.id] ? '上传中' : video.cover ? '更换' : '上传' }}
                    </span>
                </button>

                <div class="cover-info">
                    <div class="cover-title" :title="getVideoTitle(video)">
                        {{ getVideoTitle(video) }}
                    </div>
                    <el-tag v-if="video.cover" type="success" size="small">独立封面</el-tag>
                    <el-tag v-else type="info" size="small">沿用公共封面</el-tag>
                </div>

                <div class="cover-actions">
                    <el-button
                        size="small"
                        :loading="Boolean(loading[video.id])"
                        :disabled="disabled"
                        @click="selectCover(video)"
                    >
                        {{ video.cover ? '更换' : '上传' }}
                    </el-button>
                    <el-button
                        v-if="video.cover"
                        size="small"
                        text
                        type="danger"
                        :disabled="Boolean(loading[video.id]) || disabled"
                        @click="clearCover(video.id)"
                    >
                        恢复公共封面
                    </el-button>
                </div>
            </div>
        </div>
    </el-drawer>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import { Plus } from '@element-plus/icons-vue'
import { useUtilsStore } from '../stores/utils'
import { selectAndUploadCover } from '../utils/coverUpload'

const props = withDefaults(
    defineProps<{
        modelValue: boolean
        videos: any[]
        uid?: number
        defaultCoverPreview?: string
        disabled?: boolean
    }>(),
    {
        videos: () => [],
        defaultCoverPreview: '',
        disabled: false
    }
)

const emit = defineEmits<{
    'update:modelValue': [value: boolean]
    'update:videos': [videos: any[]]
}>()

const utilsStore = useUtilsStore()
const previews = reactive<Record<string, string>>({})
const loading = reactive<Record<string, boolean>>({})

const getVideoTitle = (video: any) =>
    video.title || video.videoname || video.filename || '未命名视频'
const getPreview = (video: any) =>
    video.cover ? previews[video.id] || '' : props.defaultCoverPreview

const loadPreview = async (video: any) => {
    if (!video.cover || !props.uid || previews[video.id] || loading[video.id]) return

    loading[video.id] = true
    try {
        previews[video.id] = (await utilsStore.downloadCover(props.uid, video.cover)) || ''
    } finally {
        loading[video.id] = false
    }
}

const loadPreviews = () => {
    props.videos.forEach(video => void loadPreview(video))
}

watch(
    () => props.modelValue,
    visible => {
        if (visible) loadPreviews()
    }
)

watch(() => props.videos.map(video => `${video.id}:${video.cover || ''}`).join('|'), loadPreviews)

const updateVideoCover = (id: string, cover?: string) => {
    emit(
        'update:videos',
        props.videos.map(video => {
            if (video.id !== id) return video
            const updated = { ...video }
            if (cover) updated.cover = cover
            else delete updated.cover
            return updated
        })
    )
}

const selectCover = async (video: any) => {
    if (!props.uid || props.disabled || loading[video.id]) return

    try {
        loading[video.id] = true
        const url = await selectAndUploadCover(props.uid, utilsStore.uploadCover)
        if (!url) return

        previews[video.id] = (await utilsStore.downloadCover(props.uid, url)) || ''
        updateVideoCover(video.id, url)
        utilsStore.showMessage('独立封面已更新', 'success')
    } catch (error) {
        console.error('独立封面选择失败:', error)
        utilsStore.showMessage(`独立封面选择失败: ${error}`, 'error')
    } finally {
        loading[video.id] = false
    }
}

const clearCover = (id: string) => {
    delete previews[id]
    updateVideoCover(id)
    utilsStore.showMessage('已恢复使用公共封面', 'success')
}
</script>

<style scoped>
.cover-manager-tip {
    margin: -4px 0 16px;
    padding: 10px 12px;
    color: #606266;
    font-size: 13px;
    background: #f5f7fa;
    border-radius: 6px;
}

.cover-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
}

.cover-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px;
    border: 1px solid #ebeef5;
    border-radius: 8px;
}

.cover-preview {
    position: relative;
    flex: 0 0 96px;
    width: 96px;
    height: 54px;
    padding: 0;
    overflow: hidden;
    color: #909399;
    background: #f5f7fa;
    border: 1px dashed #c0c4cc;
    border-radius: 5px;
    cursor: pointer;
}

.cover-preview:disabled {
    cursor: not-allowed;
}

.cover-preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.cover-preview-mask {
    position: absolute;
    inset: auto 0 0;
    padding: 2px 0;
    color: #fff;
    font-size: 11px;
    background: rgba(0, 0, 0, 0.55);
    opacity: 0;
    transition: opacity 0.2s;
}

.cover-preview:hover .cover-preview-mask {
    opacity: 1;
}

.cover-info {
    flex: 1;
    min-width: 0;
}

.cover-title {
    margin-bottom: 6px;
    overflow: hidden;
    color: #303133;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    text-overflow: ellipsis;
}

.cover-actions {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
}
</style>
