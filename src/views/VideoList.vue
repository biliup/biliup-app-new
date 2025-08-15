<template>
    <div class="video-list-container">
        <!-- 已上传文件列表 -->
        <div
            v-if="videos && videos.length > 0"
            class="uploaded-videos-section"
        >
            <div class="uploaded-videos-list">
                <div
                    v-for="(video, index) in updateVideo(videos)"
                    :key="video.id"
                    class="uploaded-video-item"
                >
                    <!-- 序号输入框 -->
                    <div class="video-order">
                        <el-input-number
                            :model-value="index + 1"
                            :min="1"
                            :max="videos.length"
                            size="small"
                            controls-position="right"
                            :step="-1"
                            @change="(newOrder: number) => handleReorderVideo(index, newOrder - 1)"
                            class="order-input"
                        />
                    </div>
                    
                    <div class="video-status-icon">
                        <!-- 上传完成 -->
                        <el-icon
                            v-if="video.complete"
                            class="status-complete"
                        >
                            <circle-check />
                        </el-icon>
                        <!-- 上传中 -->
                        <el-icon
                            v-else-if="
                                !video.complete &&
                                video.progress > 0
                            "
                            class="status-uploading"
                        >
                            <loading />
                        </el-icon>
                        <!-- 待上传 -->
                        <el-icon v-else class="status-pending">
                            <cloudy />
                        </el-icon>
                    </div>
                    <div class="video-info">
                        <!-- 文件名和状态在同一行 -->
                        <div class="video-title-row">
                            <div class="video-title-container">
                                <div
                                    v-if="editingFileId === video.id"
                                    class="video-title-edit"
                                >
                                    <el-input
                                        v-model="editingFileName"
                                        size="small"
                                        @keyup.enter="saveFileName(video.id)"
                                        @blur="saveFileName(video.id)"
                                        @keyup.esc="cancelEditFileName"
                                        ref="videoNameInput"
                                    />
                                </div>
                                <div
                                    v-else
                                    class="video-title"
                                    @click="startEditFileName(video.id, video.title || video.videoname)"
                                >
                                    {{ video.title || video.videoname }}
                                    <el-icon class="edit-icon"><edit /></el-icon>
                                </div>
                            </div>

                            <!-- 状态标签移动到文件名右侧 -->
                            <div class="video-status">
                                <span
                                    v-if="video.complete"
                                    class="status-text complete"
                                    >上传完成</span
                                >
                                <span
                                    v-else-if="video.progress > 0"
                                    class="status-text uploading"
                                >
                                    上传中
                                </span>
                                <span
                                    v-else
                                    class="status-text pending"
                                    >待上传</span
                                >
                            </div>
                        </div>

                        <!-- 进度条区域 -->
                        <div class="progress-section">
                            <div class="progress-bar-container">
                                <el-progress
                                    :percentage="video.progress"
                                    :show-text="false"
                                    size="small"
                                    :stroke-width="3"
                                    :color="video.complete ? '#67c23a' : '#409eff'"
                                />
                                <span class="progress-text">{{ formatUploadProgress(video) }}%</span>
                            </div>
                            <div
                                class="upload-speed"
                                v-if="!video.complete && video.speed > 0"
                            >
                                {{ formatUploadSpeed(video) }}
                            </div>
                        </div>
                    </div>

                    <!-- 文件操作按钮 -->
                    <div class="video-actions">
                        <el-button
                            type="danger"
                            size="small"
                            text
                            @click="handleRemoveFile(video.id)"
                        >
                            <el-icon><delete /></el-icon>
                        </el-button>
                    </div>
                </div>
            </div>
        </div>

        <!-- 视频操作按钮组 -->
        <div class="video-buttons-group">
            <el-button
                type="primary"
                @click="$emit('selectVideo')"
            >
                <el-icon><upload-filled /></el-icon>
                选择视频文件
            </el-button>
            <el-button
                type="success"
                :loading="props.uploading"
                @click="$emit('createUpload')"
                :disabled="!videos || videos.length === 0"
            >
                <el-icon><upload-filled /></el-icon>
                加入上传队列
            </el-button>
            <el-button
                type="danger"
                plain
                @click="$emit('clearAllVideos')"
                :disabled="!videos || videos.length === 0"
            >
                <el-icon><delete /></el-icon>
                清空{{ videos && videos.length > 0 ? `(${videos.length})` : '' }}
            </el-button>
        </div>
        
        <div class="upload-tip">
            <span v-if="!isDragOver">
                支持 MP4、AVI、MOV、MKV、WMV、FLV、M4V、WEBM 等格式
            </span>
            <span v-else class="drag-active-tip">
                💡 松开鼠标即可添加文件到当前模板
            </span>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, nextTick } from 'vue'
import { 
    CircleCheck, 
    Loading, 
    Cloudy, 
    Edit, 
    Delete, 
    UploadFilled 
} from '@element-plus/icons-vue'
import { useUploadStore } from '../stores/upload'

// Props
interface Props {
    videos: any[]
    isDragOver?: boolean
    uploading?: boolean
}

const props = withDefaults(defineProps<Props>(), {
    isDragOver: false,
    uploading: false
})

// Emits
const emit = defineEmits<{
    'update:videos': [videos: any[]]
    'selectVideo': []
    'clearAllVideos': []
    'removeFile': [id: string]
    'createUpload': []
}>()

// 文件编辑状态
const editingFileId = ref<string | null>(null)
const editingFileName = ref('')
const videoNameInput = ref()
const uploadStore = useUploadStore()

// 更新视频数据
const updateVideo = (videos: any[]) => {
    for (let i = 0; i < videos.length; i++) {
        if (videos[i].id == videos[i].filename || !videos[i].path) {
            videos[i].complete = true
        } else {
            const task = uploadStore.getUploadTask(videos[i].id)
            if (task) {
                videos[i].complete = task.status === 'Completed'
                videos[i].totalSize = task.total_size || 0
                videos[i].speed = task.speed || 0
                videos[i].progress = task.progress || 0
            } else {
                videos[i].complete = false
                videos[i].totalSize = 0
                videos[i].speed = 0
                videos[i].progress = 0
            }
        }
    }
    return videos
}

// 重新排序视频
const handleReorderVideo = (currentIndex: number, newIndex: number) => {
    if (currentIndex === newIndex || newIndex < 0 || newIndex >= props.videos.length) {
        return
    }
    
    const newVideos = [...props.videos]
    const [movedItem] = newVideos.splice(currentIndex, 1)
    newVideos.splice(newIndex, 0, movedItem)
    
    emit('update:videos', newVideos)
}

// 开始编辑文件名
const startEditFileName = (id: string, currentName: string) => {
    editingFileId.value = id
    editingFileName.value = currentName
    nextTick(() => {
        const input = videoNameInput.value
        if (input) {
            input.focus()
        }
    })
}

// 保存文件名
const saveFileName = (id: string) => {
    if (!editingFileName.value.trim()) {
        cancelEditFileName()
        return
    }
    
    const newVideos = props.videos.map(video => {
        if (video.id === id) {
            return {
                ...video,
                title: editingFileName.value.trim()
            }
        }
        return video
    })
    
    emit('update:videos', newVideos)
    cancelEditFileName()
}

// 取消编辑文件名
const cancelEditFileName = () => {
    editingFileId.value = null
    editingFileName.value = ''
}

// 格式化上传进度
const formatUploadProgress = (video: any) => {
    return Math.round(video.progress || 0)
}

// 格式化上传速度
const formatUploadSpeed = (video: any) => {
    const speed = video.speed || 0
    if (speed < 1024) {
        return `${speed.toFixed(1)} B/s`
    } else if (speed < 1024 * 1024) {
        return `${(speed / 1024).toFixed(1)} KB/s`
    } else {
        return `${(speed / 1024 / 1024).toFixed(1)} MB/s`
    }
}

// 处理删除文件
const handleRemoveFile = (id: string) => {
    emit('removeFile', id)
}
</script>

<style scoped>
.video-list-container {
    width: 100%;
}

.uploaded-videos-section {
    --video-item-height: 35px; /* 定义CSS变量 */
    margin-bottom: 20px;
    padding-bottom: 20px;
    border-bottom: 1px solid #f0f2f5;
}

.uploaded-videos-section h4 {
    margin: 0 0 16px 0;
    font-size: 14px;
    font-weight: 500;
    color: #303133;
}

.uploaded-videos-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 250px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #c1c1c1 transparent;
    border-radius: 6px;
    border: 1px solid #e9ecef;
    padding: 8px;
    background: #fafbfc;
}

.uploaded-videos-list::-webkit-scrollbar {
    width: 6px;
}

.uploaded-videos-list::-webkit-scrollbar-track {
    background: transparent;
}

.uploaded-videos-list::-webkit-scrollbar-thumb {
    background-color: #c1c1c1;
    border-radius: 3px;
}

.uploaded-videos-list::-webkit-scrollbar-thumb:hover {
    background-color: #a8a8a8;
}

.uploaded-video-item {
    display: flex;
    align-items: center;
    padding: 4px 8px;
    background: #fff;
    border-radius: 4px;
    border: 1px solid #e9ecef;
    transition: all 0.3s;
    min-height: var(--video-item-height);
    flex-shrink: 0;
}

.uploaded-video-item:hover {
    background: #f0f9ff;
    border-color: #b3d8ff;
}

.video-order {
    margin-right: 12px;
    flex-shrink: 0;
}

.video-order .order-input {
    width: 70px;
}

.video-order :deep(.el-input-number .el-input__inner) {
    text-align: center;
    font-size: 12px;
    padding: 0 0px;
}

.video-order :deep(.el-input-number__increase),
.video-order :deep(.el-input-number__decrease) {
    width: 14px;
    font-size: 10px;
}

.video-status-icon {
    margin-right: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
}

.status-complete {
    color: #67c23a;
    font-size: 14px;
}

.status-uploading {
    color: #409eff;
    font-size: 12px;
    animation: rotate 1s linear infinite;
}

.status-pending {
    color: #909399;
    font-size: 12px;
}

@keyframes rotate {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

.video-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
}

.video-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
}

.video-title-container {
    flex: 1;
    min-width: 0;
}

.video-title {
    font-size: 12px;
    font-weight: 500;
    color: #303133;
    line-height: 1.2;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 1px 3px;
    border-radius: 2px;
    transition: all 0.3s;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.video-title:hover {
    background: #ecf5ff;
    color: #409eff;
}

.video-title:hover .edit-icon {
    opacity: 1;
}

.edit-icon {
    opacity: 0;
    font-size: 10px;
    transition: opacity 0.3s;
}

.video-title-edit {
    width: 150px;
}

.video-status {
    flex-shrink: 0;
}

.video-status .status-text {
    padding: 1px 4px;
    border-radius: 2px;
    font-size: 9px;
    font-weight: 500;
    line-height: 1.2;
}

.video-status .status-text.complete {
    background: #f0f9ff;
    color: #67c23a;
}

.video-status .status-text.uploading {
    background: #ecf5ff;
    color: #409eff;
}

.video-status .status-text.pending {
    background: #f4f4f5;
    color: #909399;
}

.progress-section {
    display: flex;
    flex-direction: column;
    gap: 1px;
    margin-top: 1px;
}

.progress-bar-container {
    display: flex;
    align-items: center;
    gap: 4px;
}

.progress-bar-container :deep(.el-progress) {
    flex: 1;
    min-width: 60px;
}

.progress-text {
    font-size: 9px;
    font-weight: 500;
    color: #606266;
    min-width: 25px;
    text-align: right;
}

.upload-speed {
    font-size: 9px;
    color: #909399;
    text-align: right;
    font-family: 'Courier New', monospace;
    line-height: 1.2;
}

.video-actions {
    margin-left: 6px;
    opacity: 1;
    display: flex;
    gap: 2px;
}

.video-buttons-group {
    display: flex;
    justify-content: center;
    gap: 12px;
    margin-bottom: 12px;
}

.upload-tip {
    font-size: 13px;
    color: #909399;
    text-align: center;
}

.drag-active-tip {
    color: #409eff;
    font-weight: 500;
}
</style>