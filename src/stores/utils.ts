import { t } from '../i18n'
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import type { MentionUserGroup } from '../types/mention'
import type { CreatorVideoPage } from '../types/creator_video'
import type { ArchiveProcessing, TranscodeDetail } from '../types/video_status'
import type { TemplateConfig } from './user_config'
import type { ManagedSeason, ManagedSection, SeasonEpisode } from '../types/season'

const originalConsoleError = console.error.bind(console)

export const useUtilsStore = defineStore('template', () => {
    const archieve_pre = ref<any>(null)
    const topiclist = ref<any[]>([])
    const seasonsByUid = ref<Record<number, any[]>>({})

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
            console.error(t('获取当前版本失败:'), error)
            return 'unknown' as string
        }
    }

    const getFileSize = async (filePath: string): Promise<number> => {
        try {
            const size = await invoke<number>('get_file_size', { filePath })
            return size
        } catch (error) {
            console.error(t('获取文件大小失败:'), error)
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
            console.error(t('递归读取目录失败:'), error)
            throw error
        }
    }

    const getAvatarCacheDir = async () => {
        try {
            const cacheDir = await invoke<string>('get_avatar_cache_dir')
            return cacheDir
        } catch (error) {
            console.error(t('获取头像缓存目录失败:'), error)
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
            console.error(t('下载封面失败:'), error)
            throw error
        }
    }

    const initArchievePre = async (uid: number) => {
        try {
            archieve_pre.value = await invoke('get_archive_pre', { uid })
            return archieve_pre
        } catch (error) {
            console.error(t('获取archieve pre失败:'), error)
            throw error
        }
    }

    const initTopicList = async (uid: number) => {
        try {
            topiclist.value = await invoke('get_topic_list', { uid })
            return topiclist
        } catch (error) {
            console.error(t('获取话题列表失败:'), error)
            throw error
        }
    }

    const searchTopics = async (uid: number, query: string) => {
        try {
            const results = await invoke('search_topics', { uid, query })
            return results
        } catch (error) {
            console.error(t('搜索话题失败:'), error)
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
            console.error(t('搜索@用户失败:'), error)
            throw error
        }
    }

    const getCachedSeasons = (uid: number): any[] => seasonsByUid.value[uid] || []

    const getSeasonList = async (uid: number) => {
        try {
            const result = await invoke<{ seasons: any[] }>('get_season_list', { uid })
            if (!Array.isArray(result.seasons)) throw new Error(t('合集列表响应格式错误'))
            seasonsByUid.value[uid] = result.seasons
            return result.seasons.length > 0
        } catch (error) {
            console.error(t('获取合集列表失败:'), error)
            seasonsByUid.value[uid] = []
            throw error
        }
    }

    const listManagedSeasons = (uid: number) =>
        invoke<ManagedSeason[]>('list_managed_seasons', { uid })

    const getManagedSection = (uid: number, seasonId: number, sectionId: number) =>
        invoke<ManagedSection>('get_managed_section', { uid, seasonId, sectionId })

    const getSeasonArchiveTitle = (uid: number, aid: number) =>
        invoke<Pick<SeasonEpisode, 'archiveTitle' | 'bvid'>>('get_season_archive_title', {
            uid,
            aid
        })

    const addSeasonEpisodes = (
        uid: number,
        seasonId: number,
        sectionId: number,
        videoIds: string[]
    ) => invoke<number[]>('add_season_episodes', { uid, seasonId, sectionId, videoIds })

    const removeSeasonEpisode = (
        uid: number,
        seasonId: number,
        sectionId: number,
        episodeId: number
    ) => invoke<void>('remove_season_episode', { uid, seasonId, sectionId, episodeId })

    const reorderSeasonEpisodes = (
        uid: number,
        seasonId: number,
        sectionId: number,
        episodeIds: number[]
    ) => invoke<void>('reorder_season_episodes', { uid, seasonId, sectionId, episodeIds })

    const editSeasonEpisodeTitle = (
        uid: number,
        seasonId: number,
        sectionId: number,
        episodeId: number,
        title: string
    ) => invoke<void>('edit_season_episode_title', { uid, seasonId, sectionId, episodeId, title })

    const uploadCover = async (uid: number, file: string) => {
        if (!file) {
            return undefined
        }
        try {
            console.log(t('上传文件:'), file)
            const cover_url: string = await invoke('upload_cover', { uid, file })
            console.log(t('上传封面成功:'), cover_url)
            return cover_url
        } catch (error) {
            console.error(t('上传封面失败:'), error)
            throw error
        }
    }

    const openVideoListSnapshot = (uid: number) =>
        invoke<string>('open_video_list_snapshot', { uid })
    const closeVideoListSnapshot = (uid: number, snapshotId: string) =>
        invoke<void>('close_video_list_snapshot', { uid, snapshotId })
    const getVideoList = (
        uid: number,
        page: number,
        pageSize: number,
        seasonAddOnly = false,
        snapshotId?: string
    ) =>
        invoke<CreatorVideoPage>('get_video_list', {
            uid,
            page,
            pageSize,
            seasonAddOnly,
            snapshotId
        })

    const getArchiveProcessing = (uid: number, aid: number, page: number) =>
        invoke<ArchiveProcessing>('get_archive_processing', { uid, aid, page })

    const getVideoTranscode = (uid: number, bvid: string, cid: number) =>
        invoke<TranscodeDetail>('get_video_transcode', { uid, bvid, cid })

    const getVideoDetail = async (uid: number, videoId: string) => {
        try {
            const detail = await invoke('get_video_detail', { uid, videoId })
            return detail
        } catch (error) {
            console.error(t('获取视频详情失败:'), error)
            throw error
        }
    }

    const getVideoSeason = async (uid: number, aid: number) => {
        try {
            const season = await invoke<{ season_id: number; section_id: number }>(
                'get_video_season',
                { uid, aid }
            )
            return season
        } catch (error) {
            console.error(t('获取视频合集失败:'), error)
            throw error
        }
    }

    const getVideoTemplate = async (uid: number, videoId: string, isEdit = true) => {
        const template = (await getVideoDetail(uid, videoId)) as TemplateConfig
        for (const video of template.videos || []) {
            video.id = video.filename
            video.path = ''
        }
        if (template.aid && (await getSeasonList(uid))) {
            const membership = await getVideoSeason(uid, template.aid)
            if (membership.season_id !== 0) {
                template.season_id = membership.season_id
                template.section_id = membership.section_id
            }
        }
        if (!isEdit) template.aid = undefined
        return template
    }

    const switchSeason = async (uid: number, aid: number, seasonId: number, sectionId: number) => {
        try {
            return await invoke<boolean>('switch_season', { uid, aid, seasonId, sectionId })
        } catch (error) {
            console.error(t('设置合集失败:'), error)
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
            showMessage(t('日志导出成功'), 'success')
            return result
        } catch (error) {
            console.error(t('导出日志失败:'), error)
            showMessage(t('导出日志失败: {0}', { 0: String(error) }), 'error')
            throw error
        }
    }

    // 检查更新
    const checkUpdate = async (): Promise<string | null> => {
        try {
            const result = await invoke<string | null>('check_update')
            return result
        } catch (error) {
            console.error(t('检查更新失败:'), error)
            showMessage(t('检查更新失败: {0}', { 0: String(error) }), 'error')
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
            originalConsoleError(t('日志转发失败:'), error)
        }
    }

    return {
        archieve_pre: computed(() => archieve_pre.value),
        typelist: computed<any[]>(() => (archieve_pre.value?.typelist || []) as any[]),
        typeListV2: computed<any[]>(() => (archieve_pre.value?.type_list_v2 || []) as any[]),
        common_staff_conf,
        topiclist: computed(() => topiclist.value),
        getCachedSeasons,
        listManagedSeasons,
        getManagedSection,
        getSeasonArchiveTitle,
        addSeasonEpisodes,
        removeSeasonEpisode,
        reorderSeasonEpisodes,
        editSeasonEpisodeTitle,
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
        getArchiveProcessing,
        getVideoTranscode,
        getVideoTemplate,
        getVideoList,
        openVideoListSnapshot,
        closeVideoListSnapshot,
        getSeasonList,
        getVideoSeason,
        switchSeason,
        showMessage,
        exportLogs,
        checkUpdate,
        log
    }
})
