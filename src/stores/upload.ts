import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'

interface UploadTask {
    id: string
    template: string
    user: any
    video: any
    status: string
    progress: number
    total_transmit_bytes: number
    speed: number
    total_size: number
    error_message?: string
    created_at: number
    started_at?: number
    finished_at?: number
    retry_count: number
}

export const useUploadStore = defineStore('upload', () => {
    const uploadQueue = ref<UploadTask[]>([])

    // 创建上传任务
    const createUploadTask = async (uid: number, template: string, videoFiles: any[]) => {
        try {
            var count = 0
            // 遍历videoFiles, 将所有id不存在uploadQueue中的任务添加到uploadQueue
            for (const video of videoFiles) {
                console.log(video)
                if (video.complete !== undefined && video.complete) {
                    console.log('跳过已完成视频文件:', video)
                    continue
                }
                if (!uploadQueue.value.some(task => task.id === video.id)) {
                    try {
                        await invoke('create_upload_task', { uid, template, video })
                        count++
                    } catch (error) {
                        console.error('创建上传任务失败:', error)
                        throw error
                    }
                }
            }

            await getUploadQueue() // 刷新队列
            return count
        } catch (error) {
            console.error('创建上传任务失败:', error)
            throw error
        }
    }

    // 开始上传
    const startUpload = async (taskId: string) => {
        try {
            const success = await invoke('start_upload', { taskId })
            await getUploadQueue()

            return success ? 1 : 0
        } catch (error) {
            console.error('开始上传失败:', error)
            throw error
        }
    }

    // 暂停上传
    const pauseUpload = async (taskId: string) => {
        try {
            const paused = await invoke('pause_upload', { taskId })
            await getUploadQueue()

            return paused ? 1 : 0
        } catch (error) {
            console.error('暂停上传失败:', error)
            throw error
        }
    }

    // 取消上传
    const cancelUpload = async (taskId: string) => {
        try {
            const canceled = await invoke('cancel_upload', { taskId })
            await getUploadQueue()

            return canceled ? 1 : 0
        } catch (error) {
            console.error('取消上传失败:', error)
            throw error
        }
    }

    // 获取上传队列
    const getUploadQueue = async () => {
        try {
            const queue: UploadTask[] = await invoke('get_upload_queue')
            uploadQueue.value = queue
            // 更新速率
            const now = Date.now()
            queue
                .filter(task => task.started_at && task.status === 'Running')
                .forEach(task => {
                    const elapsed = now - task.started_at!
                    task.speed = elapsed > 0 ? (task.total_transmit_bytes * 1000) / elapsed : 0
                })

            queue
                .filter(
                    task =>
                        task.started_at &&
                        task.status === 'Running' &&
                        task.speed === 0 &&
                        now - task.started_at! > 30000
                )
                .forEach(task => {
                    ElMessage.warning(
                        `${task.user.username}-${task.video.title} 超过 30 秒未上传，正在重试...`
                    )
                    retryUpload(task.id, false)
                })

            return queue
        } catch (error) {
            console.error('获取上传队列失败:', error)
            throw error
        }
    }

    // 重试上传
    const retryUpload = async (taskId: string, refresh: boolean = true) => {
        try {
            await invoke('retry_upload', { taskId })
            if (refresh) {
                await getUploadQueue()
            }
        } catch (error) {
            console.error('重试上传失败:', error)
            throw error
        }
    }

    // 提交视频
    const submitTemplate = async (uid: number, upload: any) => {
        try {
            const result = await invoke('submit', { uid, form: upload })
            return result
        } catch (error) {
            console.error('提交视频失败:', error)
            throw error
        }
    }

    const getUploadTask = (taskId: string) => {
        const task = uploadQueue.value.find(t => t.id === taskId)
        if (task) {
            return task
        } else {
            return null
        }
    }

    return {
        uploadQueue,
        createUploadTask,
        startUpload,
        pauseUpload,
        cancelUpload,
        getUploadQueue,
        retryUpload,
        submitTemplate,
        getUploadTask
    }
})
