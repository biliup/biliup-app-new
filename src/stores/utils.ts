import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export const useUtilsStore = defineStore('template', () => {
    const typelist = ref<any[]>([])
    const topiclist = ref<any[]>([])
    const seasonlist = ref<any[]>([])
    const hasSeason = ref<boolean>(false)

    const getCurrentVersion = async () => {
        try {
            const version = await invoke('get_current_version')
            return version
        } catch (error) {
            console.error('获取当前版本失败:', error)
            return 'unknown' as string
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

    const initTypeList = async (uid: number) => {
        try {
            typelist.value = await invoke('get_type_list', { uid })
            return typelist
        } catch (error) {
            console.error('获取分区列表失败:', error)
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
        seasonId: number,
        sectionId: number,
        title: string,
        add: boolean
    ) => {
        if (!hasSeason) {
            return
        }

        try {
            await invoke('switch_season', { uid, aid, seasonId, sectionId, title, add })
        } catch (error) {
            console.error('设置合集失败:', error)
            throw error
        }
    }

    return {
        typelist: computed(() => typelist.value),
        topiclist: computed(() => topiclist.value),
        seasonlist: computed(() => seasonlist.value),
        getCurrentVersion,
        uploadCover,
        downloadCover,
        initTypeList,
        initTopicList,
        searchTopics,
        getVideoDetail,
        hasSeason,
        getSeasonList,
        getVideoSeason,
        switchSeason
    }
})
