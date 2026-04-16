import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import type { MentionUserGroup } from '../types/mention'

export const useUtilsStore = defineStore('template', () => {
    const archieve_pre = ref<any>(null)
    const topiclist = ref<any[]>([])
    const seasonlist = ref<any[]>([])
    const hasSeason = ref<boolean>(false)

    const common_staff_conf = computed(() => {
        const conf = archieve_pre.value?.common_staff_conf || {}

        const titles = Array.isArray(conf.titles)
            ? [...new Set(conf.titles.map((item: any) => String(item).trim()).filter(Boolean))]
            : []

        const missions = Array.isArray(conf.missions)
            ? [
                  ...new Set(
                      conf.missions
                          .map((item: any) => Number(item))
                          .filter((item: number) => Number.isInteger(item) && item > 0)
                  )
              ]
            : []

        return {
            max_staff: Number(conf.max_staff) > 0 ? Number(conf.max_staff) : 10,
            title_ids: conf.title_ids ?? null,
            titles,
            missions
        }
    })

    const getCurrentVersion = async () => {
        try {
            const version = await invoke('get_current_version')
            return version
        } catch (error) {
            console.error('获取当前版本失败:', error)
            return 'unknown' as string
        }
    }

    const getFileSize = async (filePath: string): Promise<number> => {
        try {
            const size = await invoke<number>('get_file_size', { filePath })
            return size
        } catch (error) {
            console.error('获取文件大小失败:', error)
            throw error
        }
    }

    const readDirRecursive = async (
        dirPath: string,
        includeSubdirs: boolean,
        maxDepth?: number
    ): Promise<Array<{ name: string; path: string; is_directory: boolean }>> => {
        try {
            const files = await invoke<
                Array<{ name: string; path: string; is_directory: boolean }>
            >('read_dir_recursive', {
                dirPath,
                includeSubdirs,
                maxDepth: maxDepth || 20
            })
            return files
        } catch (error) {
            console.error('递归读取目录失败:', error)
            throw error
        }
    }

    const getAvatarCacheDir = async () => {
        try {
            const cacheDir = await invoke<string>('get_avatar_cache_dir')
            return cacheDir
        } catch (error) {
            console.error('获取头像缓存目录失败:', error)
            throw error
        }
    }

    const downloadCover = async (uid: number, url: string) => {
        if (!url) {
            return undefined
        }
        try {
            const cover: string = await invoke('download_cover', { uid, url })
            return 'data:image/jpeg;base64,' + cover
        } catch (error) {
            console.error('下载封面失败:', error)
            throw error
        }
    }

    const initArchievePre = async (uid: number) => {
        try {
            archieve_pre.value = await invoke('get_archive_pre', { uid })
            return archieve_pre
        } catch (error) {
            console.error('获取archieve pre失败:', error)
            throw error
        }
    }

    const initTopicList = async (uid: number) => {
        try {
            topiclist.value = await invoke('get_topic_list', { uid })
            return topiclist
        } catch (error) {
            console.error('获取话题列表失败:', error)
            throw error
        }
    }

    const searchTopics = async (uid: number, query: string) => {
        try {
            const results = await invoke('search_topics', { uid, query })
            return results
        } catch (error) {
            console.error('搜索话题失败:', error)
            throw error
        }
    }

    const searchMention = async (uid: number, keyword?: string) => {
        try {
            const query = (keyword || '').trim()
            const groups = await invoke<MentionUserGroup[]>('search_mention', {
                uid,
                keyword: query || undefined
            })
            return groups || []
        } catch (error) {
            console.error('搜索@用户失败:', error)
            throw error
        }
    }

    const getSeasonList = async (uid: number) => {
        hasSeason.value = false

        try {
            seasonlist.value = ((await invoke('get_season_list', { uid })) as any).seasons
            // {"seasons": [{season_id: 1, section_id: 2, title: '合集1'}, {season_id: 2, section_id: 2, title: '合集2'}]}
            hasSeason.value = seasonlist.value.length > 0
        } catch (error) {
            console.error('获取合集列表失败:', error)
            seasonlist.value = []
            throw error
        }
        return hasSeason.value
    }

    const uploadCover = async (uid: number, file: string) => {
        if (!file) {
            return undefined
        }
        try {
            console.log('上传文件:', file)
            const cover_url: string = await invoke('upload_cover', { uid, file })
            console.log('上传封面成功:', cover_url)
            return cover_url
        } catch (error) {
            console.error('上传封面失败:', error)
            throw error
        }
    }

    const getVideoDetail = async (uid: number, videoId: string) => {
        try {
            const detail = await invoke('get_video_detail', { uid, videoId })
            return detail
        } catch (error) {
            console.error('获取视频详情失败:', error)
            throw error
        }
    }

    const getVideoSeason = async (uid: number, aid: number) => {
        if (!hasSeason) {
            return 0
        }

        try {
            const season = (await invoke('get_video_season', { uid, aid })) as number
            return season
        } catch (error) {
            console.error('获取视频合集失败:', error)
            throw error
        }
    }

    const switchSeason = async (
        uid: number,
        aid: number,
        cid: number,
        seasonId: number,
        sectionId: number,
        title: string,
        add: boolean
    ) => {
        if (!hasSeason) {
            return
        }

        try {
            await invoke('switch_season', { uid, aid, cid, seasonId, sectionId, title, add })
        } catch (error) {
            console.error('设置合集失败:', error)
            throw error
        }
    }

    // 消息提示帮助函数
    const showMessage = (
        message: string,
        type: 'success' | 'error' | 'warning' | 'info' = 'info'
    ) => {
        ElMessage({
            message,
            type,
            showClose: true,
            duration: type === 'error' ? 0 : 3000
        })
    }

    // 导出日志
    const exportLogs = async (): Promise<string> => {
        try {
            const result = await invoke<string>('export_logs')
            showMessage('日志导出成功', 'success')
            return result
        } catch (error) {
            console.error('导出日志失败:', error)
            showMessage(`导出日志失败: ${error}`, 'error')
            throw error
        }
    }

    // 检查更新
    const checkUpdate = async (): Promise<string | null> => {
        try {
            const result = await invoke<string | null>('check_update')
            return result
        } catch (error) {
            console.error('检查更新失败:', error)
            showMessage(`检查更新失败: ${error}`, 'error')
            throw error
        }
    }

    const log = async (level: string, ...messages: any[]) => {
        try {
            await invoke('console_log', {
                level,
                messages: messages.map(msg => (typeof msg === 'string' ? msg : JSON.stringify(msg)))
            })
        } catch (error) {
            console.error('日志转发失败:', error)
        }
    }

    return {
        archieve_pre: computed(() => archieve_pre.value),
        typelist: computed<any[]>(() => (archieve_pre.value?.typelist || []) as any[]),
        common_staff_conf,
        topiclist: computed(() => topiclist.value),
        seasonlist: computed(() => seasonlist.value),
        getCurrentVersion,
        getFileSize,
        getAvatarCacheDir,
        readDirRecursive,
        uploadCover,
        downloadCover,
        initArchievePre,
        initTopicList,
        searchTopics,
        searchMention,
        getVideoDetail,
        hasSeason,
        getSeasonList,
        getVideoSeason,
        switchSeason,
        showMessage,
        exportLogs,
        checkUpdate,
        log
    }
})
