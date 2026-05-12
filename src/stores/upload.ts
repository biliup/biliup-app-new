import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useUtilsStore } from './utils'

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
    const utilsStore = useUtilsStore()
    const submitInvokeIntervalMs = 5000
    const submitCancelCode = 'SUBMIT_CANCELLED'
    const submitRequestQueue: Array<{
        uid: number
        upload: any
        cancelKey?: string
        cancelled: boolean
        resolve: (value: any) => void
        reject: (reason?: any) => void
    }> = []
    let isSubmitQueueProcessing = false
    let lastSubmitInvokeAt = 0
    let activeSubmitRequest:
        | {
              uid: number
              upload: any
              cancelKey?: string
              cancelled: boolean
              resolve: (value: any) => void
              reject: (reason?: any) => void
          }
        | null = null

    const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms))

    const createSubmitCancelledError = () => {
        const error = new Error('提交已取消') as Error & { code?: string }
        error.code = submitCancelCode
        return error
    }

    const waitForSubmitWindow = async (
        request: {
            cancelled: boolean
        },
        waitMs: number
    ) => {
        let elapsed = 0
        while (elapsed < waitMs) {
            if (request.cancelled) {
                return false
            }
            const chunk = Math.min(100, waitMs - elapsed)
            await sleep(chunk)
            elapsed += chunk
        }
        return !request.cancelled
    }

    const processSubmitQueue = async () => {
        if (isSubmitQueueProcessing) {
            return
        }

        isSubmitQueueProcessing = true
        try {
            while (submitRequestQueue.length > 0) {
                const request = submitRequestQueue.shift()
                if (!request) {
                    continue
                }

                activeSubmitRequest = request
                if (request.cancelled) {
                    request.reject(createSubmitCancelledError())
                    activeSubmitRequest = null
                    continue
                }

                const now = Date.now()
                const waitMs = Math.max(0, submitInvokeIntervalMs - (now - lastSubmitInvokeAt))
                if (waitMs > 0) {
                    const canContinue = await waitForSubmitWindow(request, waitMs)
                    if (!canContinue) {
                        request.reject(createSubmitCancelledError())
                        activeSubmitRequest = null
                        continue
                    }
                }

                const { uid, upload, resolve, reject } = request

                if (request.cancelled) {
                    reject(createSubmitCancelledError())
                    activeSubmitRequest = null
                    continue
                }

                try {
                    const result = await invoke('submit', { uid, form: upload })
                    resolve(result)
                } catch (error) {
                    console.error('提交视频失败:', error)
                    reject(error)
                } finally {
                    lastSubmitInvokeAt = Date.now()
                    activeSubmitRequest = null
                }
            }
        } finally {
            activeSubmitRequest = null
            isSubmitQueueProcessing = false
        }
    }

    const cancelPendingSubmitByKey = (cancelKey: string) => {
        let cancelledCount = 0

        for (let i = submitRequestQueue.length - 1; i >= 0; i -= 1) {
            const request = submitRequestQueue[i]
            if (request.cancelKey !== cancelKey) {
                continue
            }

            submitRequestQueue.splice(i, 1)
            request.cancelled = true
            request.reject(createSubmitCancelledError())
            cancelledCount += 1
        }

        if (activeSubmitRequest && activeSubmitRequest.cancelKey === cancelKey) {
            activeSubmitRequest.cancelled = true
            cancelledCount += 1
        }

        return cancelledCount
    }

    const isSubmitCancelledError = (error: unknown) => {
        return (
            typeof error === 'object' &&
            error !== null &&
            'code' in error &&
            (error as { code?: string }).code === submitCancelCode
        )
    }

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
                        const created = await invoke<boolean>('create_upload_task', {
                            uid,
                            template,
                            video
                        })
                        if (created) {
                            count++
                        }
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
                    if (task.retry_count && task.retry_count >= 3) {
                        utilsStore.showMessage(
                            `${task.user.username}-${task.video.title} 超过 3 次重试，取消任务`
                        )
                        cancelUpload(task.id)
                    } else {
                        utilsStore.showMessage(
                            `${task.user.username}-${task.video.title} 超过 30 秒未上传，正在重试...`
                        )
                        task.retry_count = task.retry_count ? task.retry_count + 1 : 1
                        retryUpload(task.id, false)
                    }
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
    const submitTemplate = async (uid: number, upload: any, options?: { cancelKey?: string }) => {
        return new Promise<any>((resolve, reject) => {
            submitRequestQueue.push({
                uid,
                upload,
                cancelKey: options?.cancelKey,
                cancelled: false,
                resolve,
                reject
            })
            void processSubmitQueue()
        })
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
        getUploadTask,
        cancelPendingSubmitByKey,
        isSubmitCancelledError
    }
})
