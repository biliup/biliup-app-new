import { t } from '../i18n'
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

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

interface BatchTaskActionResult {
    successCount: number
    noopTaskIds: string[]
    failedTaskIds: string[]
}

export const useUploadStore = defineStore('upload', () => {
    const uploadQueue = ref<UploadTask[]>([])
    const speedSamples = new Map<string, { bytes: number; at: number }>()
    let queueRequestId = 0
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
    let activeSubmitRequest: {
        uid: number
        upload: any
        cancelKey?: string
        cancelled: boolean
        resolve: (value: any) => void
        reject: (reason?: any) => void
    } | null = null

    const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms))

    const createSubmitCancelledError = () => {
        const error = new Error(t('提交已取消')) as Error & { code?: string }
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
                    console.error(t('提交视频失败:'), error)
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
            const pendingVideos = videoFiles.filter(video => {
                if (video.complete !== undefined && video.complete) {
                    return false
                }
                return !uploadQueue.value.some(task => task.id === video.id)
            })

            if (pendingVideos.length === 0) {
                return 0
            }

            const createdCount = await invoke<number>('create_upload_task', {
                uid,
                template,
                videos: pendingVideos
            })

            await getUploadQueue() // 刷新队列
            return createdCount
        } catch (error) {
            console.error(t('创建上传任务失败:'), error)
            throw error
        }
    }

    const normalizeTaskIds = (taskIds: string | string[]) => {
        return Array.isArray(taskIds) ? taskIds : [taskIds]
    }

    // 开始上传
    const startUpload = async (taskIds: string | string[]) => {
        try {
            const startedResult = await invoke<BatchTaskActionResult>('start_upload', {
                taskIds: normalizeTaskIds(taskIds)
            })
            await getUploadQueue()

            return startedResult
        } catch (error) {
            console.error(t('开始上传失败:'), error)
            throw error
        }
    }

    // 暂停上传
    const pauseUpload = async (taskIds: string | string[]) => {
        try {
            const pausedResult = await invoke<BatchTaskActionResult>('pause_upload', {
                taskIds: normalizeTaskIds(taskIds)
            })
            await getUploadQueue()

            return pausedResult
        } catch (error) {
            console.error(t('暂停上传失败:'), error)
            throw error
        }
    }

    // 取消上传
    const cancelUpload = async (taskIds: string | string[]) => {
        try {
            const cancelledResult = await invoke<BatchTaskActionResult>('cancel_upload', {
                taskIds: normalizeTaskIds(taskIds)
            })
            await getUploadQueue()

            return cancelledResult
        } catch (error) {
            console.error(t('取消上传失败:'), error)
            throw error
        }
    }

    // 获取上传队列
    const getUploadQueue = async () => {
        const requestId = ++queueRequestId
        try {
            const queue = await invoke<UploadTask[]>('get_upload_queue')
            if (requestId !== queueRequestId) return uploadQueue.value
            const now = Date.now()
            const taskIds = new Set(queue.map(task => task.id))
            for (const id of speedSamples.keys()) {
                if (!taskIds.has(id)) speedSamples.delete(id)
            }
            for (const task of queue) {
                const previous = speedSamples.get(task.id)
                task.speed =
                    task.status === 'Running' && previous && now > previous.at
                        ? (Math.max(0, task.total_transmit_bytes - previous.bytes) * 1000) /
                          (now - previous.at)
                        : 0
                speedSamples.set(task.id, { bytes: task.total_transmit_bytes, at: now })
            }
            uploadQueue.value = queue
            return queue
        } catch (error) {
            console.error(t('获取上传队列失败:'), error)
            throw error
        }
    }

    // 重试上传
    const retryUpload = async (taskIds: string | string[], refresh: boolean = true) => {
        try {
            const retriedResult = await invoke<BatchTaskActionResult>('retry_upload', {
                taskIds: normalizeTaskIds(taskIds)
            })
            if (refresh) {
                await getUploadQueue()
            }
            return retriedResult
        } catch (error) {
            console.error(t('重试上传失败:'), error)
            throw error
        }
    }

    // 提交视频
    const submitTemplate = async (uid: number, upload: any, options?: { cancelKey?: string }) => {
        const snapshot = JSON.parse(JSON.stringify(upload))
        return new Promise<any>((resolve, reject) => {
            submitRequestQueue.push({
                uid,
                upload: snapshot,
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
