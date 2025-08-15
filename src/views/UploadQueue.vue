<template>
    <el-dropdown trigger="click" class="upload-queue-dropdown">
        <el-button link class="queue-button">
            <el-icon><list /></el-icon>
            上传队列
            <el-badge
                :value="uploadQueue.length"
                :hidden="uploadQueue.length === 0"
                class="queue-badge"
            />
            <el-icon><arrow-down /></el-icon>
        </el-button>
        <template #dropdown>
            <el-dropdown-menu class="queue-dropdown-menu">
                <div class="queue-header">
                    <el-button
                        link
                        size="small"
                        class="header-button clear-btn"
                        @click="clearCompleted"
                    >
                        清空已完成
                    </el-button>
                    <el-button link size="small" class="header-button start-btn" @click="startAll">
                        开始全部
                    </el-button>
                    <el-button link size="small" class="header-button pause-btn" @click="pauseAll">
                        暂停全部
                    </el-button>
                    <el-button
                        link
                        size="small"
                        class="header-button cancel-btn"
                        @click="cancelAll"
                    >
                        取消全部
                    </el-button>
                </div>
                <div class="queue-content">
                    <div v-if="uploadQueue.length === 0" class="empty-queue">暂无上传任务</div>
                    <div
                        v-for="task in uploadQueue"
                        :key="task.id"
                        class="queue-item"
                        :class="getTaskStatusClass(task.status)"
                    >
                        <div class="task-info">
                            <div class="task-avatar">
                                <el-avatar
                                    :src="`data:image/jpeg;base64,${task.user.avatar}`"
                                    :size="24"
                                ></el-avatar>
                            </div>
                            <div class="task-name">
                                {{ getTaskDisplayName(task) }}
                            </div>
                            <div class="task-status">
                                <div class="status-info">
                                    <span class="status-text">{{ getStatusText(task.status) }}</span>
                                    <span class="progress-text" v-if="task.status === 'Running'">
                                        {{ formatUploadProgress(task) }}%
                                    </span>
                                </div>
                                <div 
                                    class="action-buttons"
                                >
                                    <el-button
                                        link
                                        size="small"
                                        class="start-button"
                                        @click="startUpload(task.id)"
                                        v-if="canStart(task.status)"
                                    >
                                        <el-icon><video-play /></el-icon>
                                    </el-button>
                                    <el-button
                                        link
                                        size="small"
                                        class="pause-button"
                                        @click="pauseUpload(task.id)"
                                        v-if="canPause(task.status)"
                                    >
                                        <el-icon><video-pause /></el-icon>
                                    </el-button>
                                    <el-button
                                        link
                                        size="small"
                                        class="cancel-button"
                                        @click="cancelUpload(task.id)"
                                        v-if="canCancel(task.status)"
                                    >
                                        <el-icon><close /></el-icon>
                                    </el-button>
                                    <el-button
                                        link
                                        size="small"
                                        class="retry-button"
                                        @click="retryUpload(task.id)"
                                        v-if="canRetry(task.status)"
                                    >
                                        <el-icon><refresh-right /></el-icon>
                                    </el-button>
                                </div>
                            </div>
                        </div>
                        <el-progress
                            v-if="task.status === 'Running'"
                            :percentage="task.progress"
                            :show-text="false"
                            size="small"
                        />
                        <div
                            class="upload-speed"
                            v-if="task.status === 'Running' && task.speed > 0"
                        >
                            {{ formatUploadSpeed(task) }}
                        </div>
                    </div>
                </div>
            </el-dropdown-menu>
        </template>
    </el-dropdown>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useUploadStore } from '../stores/upload'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowDown, List, Close, VideoPause, RefreshRight, VideoPlay } from '@element-plus/icons-vue'

const uploadStore = useUploadStore()

// 计算属性
const uploadQueue = computed(() => uploadStore.uploadQueue)

// 上传队列相关方法
const clearCompleted = async () => {
    try {
        const completedTasks = uploadStore.uploadQueue.filter(task => task.status === 'Completed')
        if (completedTasks.length === 0) {
            ElMessage.info('没有已完成的任务')
            return
        }

        let successCount = 0
        for (const task of completedTasks) {
            try {
                successCount += await uploadStore.cancelUpload(task.id)
            } catch (error) {
                console.error(`清空任务 ${task.video.title} 失败:`, error)
            }
        }

        ElMessage.success(`已清空 ${successCount} 个完成的任务`)
    } catch (error) {
        console.error('清空已完成任务失败:', error)
        ElMessage.error(`清空已完成任务失败: ${error}`)
    }
}

const canStart = (status: string) => {
    return status === 'Paused' || status === 'Failed' || status === 'Waiting'
}

const canPause = (status: string) => {
    return status === 'Pending' || status === 'Running'
}

const canCancel = (status: string) => {
    return status !== 'Completed'
}


const canRetry = (status: string) => {
    return status !== 'Completed' && status !== 'Waiting'
}

const startAll = async () => {
    try {
        const canBeStarted = uploadStore.uploadQueue.filter(task => canStart(task.status))
        if (canBeStarted.length === 0) {
            ElMessage.info('没有可开始的任务')
            return
        }

        let successCount = 0
        for (const task of canBeStarted) {
            try {
                successCount += await uploadStore.startUpload(task.id)
            } catch (error) {
                console.error(`开始任务 ${task.video.title} 失败:`, error)
            }
        }

        ElMessage.success(`已开始 ${successCount} 个任务`)
    } catch (error) {
        console.error('开始所有任务失败:', error)
        ElMessage.error(`开始所有任务失败: ${error}`)
    }
}

const startUpload = async (taskId: string) => {
    try {
        await uploadStore.startUpload(taskId)
        ElMessage.success('任务已开始')
    } catch (error) {
        console.error('开始上传失败:', error)
        ElMessage.error(`开始上传失败: ${error}`)
    }
}

const pauseAll = async () => {
    try {
        const activeTasks = uploadStore.uploadQueue.filter(task => canPause(task.status))
        if (activeTasks.length === 0) {
            ElMessage.info('没有可暂停的任务')
            return
        }

        let successCount = 0
        for (const task of activeTasks) {
            try {
                successCount += await uploadStore.pauseUpload(task.id)
            } catch (error) {
                console.error(`暂停任务 ${task.video.title} 失败:`, error)
            }
        }

        ElMessage.success(`已暂停 ${successCount} 个任务`)
    } catch (error) {
        console.error('暂停所有任务失败:', error)
        ElMessage.error(`暂停所有任务失败: ${error}`)
    }
}

const pauseUpload = async (taskId: string) => {
    try {
        await uploadStore.pauseUpload(taskId)
        ElMessage.success('任务已暂停')
    } catch (error) {
        console.error('暂停上传失败:', error)
        ElMessage.error(`暂停上传失败: ${error}`)
    }
}

const cancelAll = async () => {
    try {
        const activeTasks = uploadStore.uploadQueue.filter(task => canCancel(task.status))
        if (activeTasks.length === 0) {
            ElMessage.info('没有可取消的任务')
            return
        }

        // 添加确认弹窗
        await ElMessageBox.confirm(
            `确定要取消所有 ${activeTasks.length} 个任务吗？此操作不可撤销。`,
            '确认取消所有任务',
            {
                confirmButtonText: '确定取消',
                cancelButtonText: '取消',
                type: 'warning'
            }
        )

        let successCount = 0
        for (const task of activeTasks) {
            try {
                successCount += await uploadStore.cancelUpload(task.id)
            } catch (error) {
                console.error(`取消任务 ${task.video.title} 失败:`, error)
            }
        }

        ElMessage.success(`已取消 ${successCount} 个任务`)
    } catch (error) {
        // 如果用户取消了确认框，不显示错误消息
        if (error !== 'cancel') {
            console.error('取消所有上传失败:', error)
            ElMessage.error(`取消所有上传失败: ${error}`)
        }
    }
}

const cancelUpload = async (taskId: string) => {
    try {
        // 获取任务信息用于确认提示
        const task = uploadStore.uploadQueue.find(t => t.id === taskId)
        const taskTitle = task?.video?.title || '未知任务'

        // 添加确认弹窗
        await ElMessageBox.confirm(
            `确定要取消上传任务"${taskTitle}"吗？此操作不可撤销。`,
            '确认取消任务',
            {
                confirmButtonText: '确定取消',
                cancelButtonText: '取消',
                type: 'warning'
            }
        )

        await uploadStore.cancelUpload(taskId)
        ElMessage.success('任务已取消')
    } catch (error) {
        // 如果用户取消了确认框，不显示错误消息
        if (error !== 'cancel') {
            console.error('取消上传失败:', error)
            ElMessage.error(`取消上传失败: ${error}`)
        }
    }
}


const retryUpload = async (taskId: string) => {
    try {
        await uploadStore.retryUpload(taskId)
        ElMessage.success('任务已重试')
    } catch (error) {
        console.error('重试上传失败:', error)
        ElMessage.error(`重试上传失败: ${error}`)
    }
}

const getTaskStatusClass = (status: string) => {
    return {
        'task-pending': status === 'Pending',
        'task-running': status === 'Running',
        'task-completed': status === 'Completed',
        'task-failed': status === 'Failed'
    }
}

const getTaskDisplayName = (task: any) => {
    return task.video?.title || '未知任务'
}

const getStatusText = (status: string) => {
    const statusMap = {
        Waiting: '待开始',
        Pending: '等待中',
        Running: '上传中',
        Completed: '已完成',
        Cancelled: '已取消',
        Paused: '已暂停',
        Failed: '失败'
    }
    return statusMap[status as keyof typeof statusMap] || status
}

const formatUploadProgress = (video: any): string => {
    if (!video || video.progress === undefined) return '0'
    if (video.progress >= 100) return '100'
    return `${video.progress.toFixed(2)}`
}

const formatUploadSpeed = (video: any): string => {
    if (!video || video.speed === undefined) return '0 B/s'

    const units = ['B/s', 'KB/s', 'MB/s', 'GB/s']
    let size = video.speed
    let unitIndex = 0

    while (size >= 1024 && unitIndex < units.length - 1) {
        size /= 1024
        unitIndex++
    }

    return `${size.toFixed(unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`
}
</script>

<style scoped>
/* 上传队列样式 */

.queue-button {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #606266;
    font-size: 14px;
}

.queue-badge {
    margin: 0 4px;
}

.queue-dropdown-menu {
    min-width: 300px;
    max-height: 250px;
}

.queue-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 15px;
    border-bottom: 1px solid #e4e7ed;
    font-weight: 500;
    background: #fff;
    position: sticky;
    top: 0;
    z-index: 10;
    gap: 8px;
}

.header-button {
    padding: 4px 8px !important;
    border-radius: 4px !important;
    font-size: 12px !important;
    font-weight: 500 !important;
    transition: all 0.3s !important;
    border: 1px solid transparent !important;
}

.header-button:hover {
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1) !important;
}

.clear-btn {
    color: #909399 !important;
}

.clear-btn:hover {
    color: #606266 !important;
    background: #f5f7fa !important;
    border-color: #dcdfe6 !important;
}

.start-btn {
    color: #67c23a !important;
}

.start-btn:hover {
    color: #529b2e !important;
    background: #f0f9ff !important;
    border-color: #b3e19d !important;
}

.pause-btn {
    color: #e6a23c !important;
}

.pause-btn:hover {
    color: #b88230 !important;
    background: #fdf6ec !important;
    border-color: #f5dab1 !important;
}

.cancel-btn {
    color: #f56c6c !important;
}

.cancel-btn:hover {
    color: #dd6161 !important;
    background: #fef0f0 !important;
    border-color: #fbc4c4 !important;
}

.queue-content {
    max-height: 250px;
    overflow-y: auto;
}

.empty-queue {
    padding: 20px;
    text-align: center;
    color: #909399;
}

.queue-item {
    padding: 10px 15px;
    border-bottom: 1px solid #f0f0f0;
    position: relative;
}

.queue-item:last-child {
    border-bottom: none;
}

.task-info {
    display: flex;
    align-items: center;
    margin-bottom: 5px;
}

.task-avatar {
    margin-right: 8px;
    flex-shrink: 0;
}

.task-name {
    font-weight: 500;
    color: #303133;
    flex: 1;
}

.task-status {
    display: flex;
    align-items: center;
    font-size: 12px;
    margin-left: auto;
    flex-shrink: 0;
    gap: 8px;
    min-width: 0;
    justify-content: space-between;
}

.status-info {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
}

.action-buttons {
    display: flex;
    align-items: center;
    gap: 2px;
}

.task-pending .status-text {
    color: #909399;
}

.task-running .status-text {
    color: #409eff;
}

.task-completed .status-text {
    color: #67c23a;
}

.task-failed .status-text {
    color: #f56c6c;
}

.cancel-button {
    color: #909399 !important;
    font-size: 12px;
    padding: 2px !important;
    margin: 0 2px;
}

.cancel-button:hover {
    color: #f56c6c !important;
}

.pause-button {
    color: #e6a23c !important;
    font-size: 12px;
    padding: 2px !important;
    margin: 0 2px;
}

.pause-button:hover {
    color: #b88230 !important;
}

.start-button {
    color: #67c23a !important;
    font-size: 12px;
    padding: 2px !important;
    margin: 0 2px;
}

.start-button:hover {
    color: #529b2e !important;
}

.retry-button {
    color: #409eff !important;
    font-size: 12px;
    padding: 2px !important;
    margin: 0 2px;
}

.retry-button:hover {
    color: #337ecc !important;
}

.upload-speed {
    font-size: 9px;
    color: #909399;
    text-align: right;
    font-family: 'Courier New', monospace;
    line-height: 1.2;
    margin-top: 2px;
}
</style>
