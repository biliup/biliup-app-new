import { readFile } from '@tauri-apps/plugin-fs'
import type { MentionOption, MentionUserGroup } from '../types/mention'

export const createDebouncedRequestScheduler = (delayMs: number) => {
    let timer: number | null = null
    let requestId = 0

    const clearTimer = () => {
        if (timer !== null) {
            window.clearTimeout(timer)
            timer = null
        }
    }

    const cancel = () => {
        clearTimer()
        requestId++
    }

    const run = (task: (id: number) => void | Promise<void>) => {
        clearTimer()
        const id = ++requestId
        timer = window.setTimeout(() => {
            void task(id)
        }, delayMs)
        return id
    }

    const isLatest = (id: number) => id === requestId

    const dispose = () => {
        cancel()
    }

    return {
        run,
        cancel,
        isLatest,
        dispose
    }
}

const normalizePath = (path: string) => path.replace(/\\/g, '/')

const resolveMimeType = (fileName: string) => {
    const lower = fileName.toLowerCase()
    if (lower.endsWith('.png')) return 'image/png'
    if (lower.endsWith('.webp')) return 'image/webp'
    if (lower.endsWith('.gif')) return 'image/gif'
    return 'image/jpeg'
}

export const flattenMentionGroups = (
    groups: MentionUserGroup[],
    buildAvatarSrc: (fileName: string) => string
): MentionOption[] => {
    const options: MentionOption[] = []

    for (const group of groups || []) {
        let first = true
        for (const item of group.items || []) {
            options.push({
                value: item.name || '',
                uid: String(item.uid || ''),
                name: item.name || '',
                face: item.face || '',
                avatarSrc: buildAvatarSrc(item.face || ''),
                fans: Number(item.fans || 0),
                groupName: group.group_name || '其他',
                showGroupLabel: first
            })
            first = false
        }
    }

    return options
}

type MentionSearchFn = (uid: number, query: string) => Promise<MentionUserGroup[]>

type QueryMentionOptionsParams = {
    uid: number
    query: string
    requestId: number
    isLatest: (id: number) => boolean
    searchMention: MentionSearchFn
    avatarCache: ReturnType<typeof createMentionAvatarCache>
    applyOptions: (options: MentionOption[]) => void
}

export const queryMentionOptions = async ({
    uid,
    query,
    requestId,
    isLatest,
    searchMention,
    avatarCache,
    applyOptions
}: QueryMentionOptionsParams) => {
    const groups = (await searchMention(uid, query)) || []
    if (!isLatest(requestId)) {
        return
    }

    try {
        await avatarCache.ensureCacheDir()
    } catch {}

    applyOptions(flattenMentionGroups(groups, fileName => avatarCache.getAvatarSrc(fileName)))

    await avatarCache.hydrateGroups(groups)
    if (!isLatest(requestId)) {
        return
    }

    applyOptions(flattenMentionGroups(groups, fileName => avatarCache.getAvatarSrc(fileName)))
}

export const createMentionAvatarCache = (getAvatarCacheDir: () => Promise<string>) => {
    let avatarCacheDir = ''
    const avatarUrlCache = new Map<string, string>()
    const avatarLoading = new Map<string, Promise<string>>()

    const ensureCacheDir = async () => {
        if (avatarCacheDir) {
            return avatarCacheDir
        }
        avatarCacheDir = (await getAvatarCacheDir()) || ''
        return avatarCacheDir
    }

    const getAvatarSrc = (fileName: string) => {
        if (!fileName) {
            return ''
        }
        return avatarUrlCache.get(fileName) || ''
    }

    const loadAvatar = async (fileName: string) => {
        if (!avatarCacheDir || !fileName) {
            return ''
        }

        const cached = avatarUrlCache.get(fileName)
        if (cached) {
            return cached
        }

        const loading = avatarLoading.get(fileName)
        if (loading) {
            return loading
        }

        const fullPath = `${normalizePath(avatarCacheDir)}/${fileName}`
        const task = readFile(fullPath)
            .then(bytes => {
                const normalizedBytes = Uint8Array.from(bytes)
                const blob = new Blob([normalizedBytes], { type: resolveMimeType(fileName) })
                const url = URL.createObjectURL(blob)
                avatarUrlCache.set(fileName, url)
                return url
            })
            .catch(() => '')
            .finally(() => {
                avatarLoading.delete(fileName)
            })

        avatarLoading.set(fileName, task)
        return task
    }

    const hydrateGroups = async (groups: MentionUserGroup[]) => {
        const files = new Set<string>()

        for (const group of groups || []) {
            for (const item of group.items || []) {
                if (item.face) {
                    files.add(item.face)
                }
            }
        }

        if (files.size === 0) {
            return
        }

        await Promise.all(Array.from(files).map(file => loadAvatar(file)))
    }

    const dispose = () => {
        for (const url of avatarUrlCache.values()) {
            URL.revokeObjectURL(url)
        }
        avatarUrlCache.clear()
        avatarLoading.clear()
    }

    return {
        ensureCacheDir,
        getAvatarSrc,
        hydrateGroups,
        dispose
    }
}
