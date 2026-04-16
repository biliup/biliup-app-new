<template>
    <div class="mention-view">
        <el-autocomplete
            v-model="inputValue"
            :fetch-suggestions="fetchSuggestions"
            :placeholder="placeholder"
            :trigger-on-focus="true"
            :disabled="disabled || !userUid"
            clearable
            @select="handleSelect"
            @input="handleInput"
        >
            <template #default="{ item }">
                <div class="mention-option">
                    <div v-if="item.showGroupLabel" class="mention-group">{{ item.groupName }}</div>
                    <div class="mention-row">
                        <el-avatar :size="34" :src="item.avatarSrc" />
                        <div class="mention-main">
                            <div class="mention-name">{{ item.name }}</div>
                            <div class="mention-meta">{{ item.fans }}粉丝</div>
                        </div>
                        <div class="mention-uid">UID: {{ item.uid }}</div>
                    </div>
                </div>
            </template>
        </el-autocomplete>
    </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { readFile } from '@tauri-apps/plugin-fs'
import { useUtilsStore } from '../stores/utils'

interface MentionUserItem {
    face: string
    fans: number
    name: string
    official_verify_type: number
    uid: string
}

interface MentionUserGroup {
    group_name: string
    group_type: number
    items: MentionUserItem[]
}

interface MentionOption {
    value: string
    uid: string
    name: string
    face: string
    avatarSrc: string
    fans: number
    groupName: string
    showGroupLabel: boolean
}

const props = withDefaults(
    defineProps<{
        modelValue?: string
        userUid?: number
        placeholder?: string
        disabled?: boolean
        debounceMs?: number
    }>(),
    {
        modelValue: '',
        placeholder: '请输入 UID 或昵称',
        disabled: false,
        debounceMs: 350
    }
)

const emit = defineEmits<{
    (e: 'update:modelValue', value: string): void
    (e: 'select', value: MentionUserItem): void
}>()

const utilsStore = useUtilsStore()
const inputValue = ref(props.modelValue || '')
const debounceTimer = ref<number | null>(null)
const refreshTimer = ref<number | null>(null)
const currentRequestId = ref(0)
const avatarCacheDir = ref('')
const avatarUrlCache = ref(new Map<string, string>())
const avatarLoading = ref(new Map<string, Promise<string>>())

const userUid = computed(() => Number(props.userUid || 0))

watch(
    () => props.modelValue,
    value => {
        if ((value || '') !== inputValue.value) {
            inputValue.value = value || ''
        }
    }
)

const clearDebounce = () => {
    if (debounceTimer.value !== null) {
        window.clearTimeout(debounceTimer.value)
        debounceTimer.value = null
    }
}

const clearRefresh = () => {
    if (refreshTimer.value !== null) {
        window.clearTimeout(refreshTimer.value)
        refreshTimer.value = null
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

const buildAvatarSrc = (fileName: string) => {
    if (!fileName) {
        return ''
    }
    return avatarUrlCache.value.get(fileName) || ''
}

const loadAvatarFromCache = async (fileName: string) => {
    if (!avatarCacheDir.value || !fileName) {
        return ''
    }

    const cached = avatarUrlCache.value.get(fileName)
    if (cached) {
        return cached
    }

    const loading = avatarLoading.value.get(fileName)
    if (loading) {
        return loading
    }

    const fullPath = `${normalizePath(avatarCacheDir.value)}/${fileName}`
    const task = readFile(fullPath)
        .then(bytes => {
            const normalizedBytes = Uint8Array.from(bytes)
            const blob = new Blob([normalizedBytes], { type: resolveMimeType(fileName) })
            const url = URL.createObjectURL(blob)
            avatarUrlCache.value.set(fileName, url)
            return url
        })
        .catch(() => '')
        .finally(() => {
            avatarLoading.value.delete(fileName)
        })

    avatarLoading.value.set(fileName, task)
    return task
}

const ensureAvatarCacheDir = async () => {
    if (avatarCacheDir.value) {
        return avatarCacheDir.value
    }
    const path = await utilsStore.getAvatarCacheDir()
    avatarCacheDir.value = path || ''
    return avatarCacheDir.value
}

onBeforeUnmount(() => {
    clearDebounce()
    clearRefresh()
    for (const url of avatarUrlCache.value.values()) {
        URL.revokeObjectURL(url)
    }
    avatarUrlCache.value.clear()
    avatarLoading.value.clear()
})

const buildOptions = (groups: MentionUserGroup[]): MentionOption[] => {
    const options: MentionOption[] = []
    for (const group of groups || []) {
        let first = true
        for (const item of group.items || []) {
            options.push({
                value: item.name,
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

const hydrateAvatars = async (
    groups: MentionUserGroup[],
    requestId: number,
    callback: (items: MentionOption[]) => void
) => {
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

    await Promise.all(Array.from(files).map(file => loadAvatarFromCache(file)))
    if (requestId !== currentRequestId.value) {
        return
    }
    callback(buildOptions(groups))
}

const fetchSuggestions = (queryString: string, callback: (items: MentionOption[]) => void) => {
    const uid = userUid.value
    if (!uid) {
        callback([])
        return
    }

    clearDebounce()
    clearRefresh()
    const requestId = ++currentRequestId.value

    debounceTimer.value = window.setTimeout(async () => {
        try {
            try {
                await ensureAvatarCacheDir()
            } catch {
                avatarCacheDir.value = ''
            }
            const groups = await utilsStore.searchMention(uid, queryString || '')
            if (requestId !== currentRequestId.value) {
                return
            }
            const normalizedGroups = groups as MentionUserGroup[]
            callback(buildOptions(normalizedGroups))
            void hydrateAvatars(normalizedGroups, requestId, callback)

            // 候选列表打开后轻量刷新一次，尽快展示刚写入缓存的头像
            refreshTimer.value = window.setTimeout(async () => {
                try {
                    const refreshedGroups = await utilsStore.searchMention(uid, queryString || '')
                    if (requestId !== currentRequestId.value) {
                        return
                    }
                    const normalizedRefreshedGroups = refreshedGroups as MentionUserGroup[]
                    callback(buildOptions(normalizedRefreshedGroups))
                    void hydrateAvatars(normalizedRefreshedGroups, requestId, callback)
                } catch {
                    // 轻量刷新失败时保留首屏结果
                }
            }, 500)
        } catch {
            if (requestId !== currentRequestId.value) {
                return
            }
            callback([])
        }
    }, props.debounceMs)
}

const handleSelect = (item: MentionOption) => {
    const uid = String(item.uid || '')
    inputValue.value = uid
    emit('update:modelValue', uid)
    emit('select', {
        uid,
        name: item.name,
        fans: item.fans,
        face: item.face,
        official_verify_type: -1
    })
}

const handleInput = (value: string) => {
    emit('update:modelValue', value || '')
}
</script>

<style scoped>
.mention-view {
    width: 100%;
}

:deep(.el-autocomplete) {
    width: 100%;
}

.mention-option {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 2px 0;
}

.mention-group {
    color: #909399;
    font-size: 12px;
    line-height: 1;
    padding: 2px 0 0;
}

.mention-row {
    display: flex;
    align-items: center;
    gap: 10px;
}

.mention-main {
    min-width: 0;
    flex: 1;
}

.mention-name {
    color: #303133;
    font-size: 14px;
    line-height: 1.3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.mention-meta {
    color: #909399;
    font-size: 12px;
    line-height: 1.2;
    margin-top: 2px;
}

.mention-uid {
    color: #909399;
    font-size: 12px;
    flex-shrink: 0;
}
</style>
