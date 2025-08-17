<template>
    <el-dropdown trigger="click" class="upload-queue-dropdown">
        <el-button link class="queue-button">
            <el-icon><list /></el-icon>
            上传队列
            <el-badge
                :value="uploadQueue.filter(task => task.status !== 'Completed').length"
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
                    <template v-for="task in uploadQueue" :key="task.id">
                        <!-- 已完成任务分割线 -->
                        <div
                            v-if="
                                task.status === 'Completed' &&
                                isFirstCompletedTask(task, uploadQueue)
                            "
                            class="completed-divider"
                        >
                            <span>已完成任务</span>
                        </div>
                        <div
                            class="queue-item"
                            :class="[getTaskStatusClass(task.status), getTaskWarningClass(task)]"
                            :title="getTaskWarningTooltip(task)"
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
                                        <span class="status-text">{{
                                            getStatusText(task.status)
                                        }}</span>
                                        <span
                                            class="progress-text"
                                            v-if="task.status === 'Running'"
                                        >
                                            {{ formatUploadProgress(task) }}%
                                        </span>
                                        <span
                                            class="completed-time"
                                            v-if="task.status === 'Completed' && task.finished_at"
                                        >
                                            {{ formatFinishedTime(task.finished_at) }}
                                        </span>
                                    </div>
                                    <div class="action-buttons">
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
                    </template>
                </div>
            </el-dropdown-menu>
        </template>
    </el-dropdown>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useUploadStore } from '../stores/upload'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
    ArrowDown,
    List,
    Close,
    VideoPause,
    RefreshRight,
    VideoPlay
} from '@element-plus/icons-vue'

const uploadStore = useUploadStore()

// 计算属性
const uploadQueue = computed(() => {
    const queue = uploadStore.uploadQueue

    // 分离已完成和未完成的任务
    const completedTasks = queue.filter(task => task.status === 'Completed')
    const activeTasks = queue.filter(task => task.status !== 'Completed')

    // 对已完成的任务按完成时间排序（最新完成的在前）
    const sortedCompletedTasks = completedTasks.sort((a, b) => {
        // 如果有finished_at字段，按该字段排序
        if (a.finished_at && b.finished_at) {
            return new Date(b.finished_at).getTime() - new Date(a.finished_at).getTime()
        }
        // 如果没有finished_at，按任务ID或创建时间排序
        return b.id.localeCompare(a.id)
    })

    // 返回：活动任务在前，已完成任务在后
    return [...activeTasks, ...sortedCompletedTasks]
})

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

// 判断是否为第一个已完成的任务（用于显示分割线）
const isFirstCompletedTask = (currentTask: any, taskList: any[]): boolean => {
    if (currentTask.status !== 'Completed') return false

    const currentIndex = taskList.findIndex(task => task.id === currentTask.id)
    if (currentIndex === 0) return true

    // 检查前一个任务是否不是已完成状态
    const previousTask = taskList[currentIndex - 1]
    return previousTask.status !== 'Completed'
}

// 格式化完成时间
const formatFinishedTime = (timestamp: number | string): string => {
    try {
        const date = new Date(timestamp)
        const now = new Date()
        const diffMs = now.getTime() - date.getTime()
        const diffMins = Math.floor(diffMs / (1000 * 60))
        const diffHours = Math.floor(diffMs / (1000 * 60 * 60))
        const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24))

        if (diffMins < 1) return '刚刚完成'
        if (diffMins < 60) return `${diffMins}分钟前`
        if (diffHours < 24) return `${diffHours}小时前`
        if (diffDays < 7) return `${diffDays}天前`

        // 超过7天显示具体日期
        return date.toLocaleDateString('zh-CN', {
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit'
        })
    } catch {
        return '未知时间'
    }
}

// 检查任务是否超过8小时（需要警告）
const isTaskExpiredSoon = (task: any): boolean => {
    if (task.status !== 'Completed' || !task.finished_at) return false

    try {
        const finishedDate = new Date(task.finished_at)
        const now = new Date()
        const diffHours = (now.getTime() - finishedDate.getTime()) / (1000 * 60 * 60)

        return diffHours >= 8
    } catch {
        return false
    }
}

// 获取任务警告样式类
const getTaskWarningClass = (task: any): string => {
    if (isTaskExpiredSoon(task)) {
        try {
            const finishedDate = new Date(task.finished_at)
            const now = new Date()
            const diffHours = (now.getTime() - finishedDate.getTime()) / (1000 * 60 * 60)

            if (diffHours >= 8) {
                return 'task-warning task-expired'
            } else {
                return 'task-warning'
            }
        } catch {
            return 'task-warning'
        }
    }
    return ''
}

// 获取任务警告提示文本
const getTaskWarningTooltip = (task: any): string => {
    if (isTaskExpiredSoon(task)) {
        try {
            const finishedDate = new Date(task.finished_at)
            const now = new Date()
            const diffHours = Math.floor(
                (now.getTime() - finishedDate.getTime()) / (1000 * 60 * 60)
            )

            if (diffHours >= 10) {
                return '此任务完成超过10小时，服务器可能已删除相关文件'
            } else {
                return `此任务完成已${diffHours}小时，服务器将在10小时后删除相关文件`
            }
        } catch {
            return '任务完成时间较长，可能无法上传'
        }
    }
    return ''
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

/* 已完成任务分割线 */
.completed-divider {
    display: flex;
    align-items: center;
    margin: 10px 0 5px 0;
    font-size: 12px;
    color: #909399;
}

.completed-divider::before,
.completed-divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: #e4e7ed;
}

.completed-divider::before {
    margin-right: 10px;
}

.completed-divider::after {
    margin-left: 10px;
}

.completed-divider span {
    white-space: nowrap;
    padding: 0 4px;
    background: #fff;
    font-weight: 500;
}

/* 已完成任务样式优化 */
.task-completed {
    opacity: 0.8;
    background: #fafbfc;
}

.task-completed .task-name {
    color: #909399;
}

.completed-time {
    font-size: 11px;
    color: #67c23a;
    font-weight: 500;
}

/* 警告任务样式 */
.task-warning {
    border: 2px solid #e6a23c;
    border-radius: 6px;
    background: linear-gradient(to right, rgba(230, 162, 60, 0.05), rgba(230, 162, 60, 0.02));
    cursor: help;
    position: relative;
}

.task-warning::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 4px;
    background: linear-gradient(to bottom, #e6a23c, #f39c12);
    border-radius: 2px 0 0 2px;
}

.task-warning:hover {
    border-color: #f39c12;
    background: linear-gradient(to right, rgba(230, 162, 60, 0.1), rgba(230, 162, 60, 0.05));
    box-shadow: 0 2px 8px rgba(230, 162, 60, 0.3);
    transform: translateY(-1px);
    transition: all 0.3s ease;
}

.task-warning .completed-time {
    color: #e6a23c;
    font-weight: 600;
    animation: pulse-warning 2s infinite;
}

@keyframes pulse-warning {
    0%,
    100% {
        opacity: 1;
    }
    50% {
        opacity: 0.7;
    }
}

/* 超过10小时的任务使用更强烈的警告颜色 */
.task-warning.task-expired {
    border-color: #f56c6c;
    background: linear-gradient(to right, rgba(245, 108, 108, 0.05), rgba(245, 108, 108, 0.02));
}

.task-warning.task-expired::before {
    background: linear-gradient(to bottom, #f56c6c, #e74c3c);
}

.task-warning.task-expired:hover {
    border-color: #e74c3c;
    background: linear-gradient(to right, rgba(245, 108, 108, 0.1), rgba(245, 108, 108, 0.05));
    box-shadow: 0 2px 8px rgba(245, 108, 108, 0.3);
}

.task-warning.task-expired .completed-time {
    color: #f56c6c;
    animation: pulse-danger 1.5s infinite;
}

@keyframes pulse-danger {
    0%,
    100% {
        opacity: 1;
        transform: scale(1);
    }
    50% {
        opacity: 0.8;
        transform: scale(1.05);
    }
}
</style>
