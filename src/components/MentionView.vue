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
import { useUtilsStore } from '../stores/utils'
import {
    createMentionAvatarCache,
    createDebouncedRequestScheduler,
    queryMentionOptions
} from '../composables/useMentionSearchShared'
import type { MentionOption, MentionUserItem } from '../types/mention'

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
const refreshTimer = ref<number | null>(null)
const avatarCache = createMentionAvatarCache(async () => await utilsStore.getAvatarCacheDir())
const searchScheduler = createDebouncedRequestScheduler(props.debounceMs)

const userUid = computed(() => Number(props.userUid || 0))

watch(
    () => props.modelValue,
    value => {
        if ((value || '') !== inputValue.value) {
            inputValue.value = value || ''
        }
    }
)

const clearRefresh = () => {
    if (refreshTimer.value !== null) {
        window.clearTimeout(refreshTimer.value)
        refreshTimer.value = null
    }
}

onBeforeUnmount(() => {
    searchScheduler.dispose()
    clearRefresh()
    avatarCache.dispose()
})

const fetchSuggestions = (queryString: string, callback: (items: MentionOption[]) => void) => {
    const uid = userUid.value
    if (!uid) {
        callback([])
        return
    }

    searchScheduler.cancel()
    clearRefresh()
    searchScheduler.run(async requestId => {
        try {
            await queryMentionOptions({
                uid,
                query: queryString || '',
                requestId,
                isLatest: searchScheduler.isLatest,
                searchMention: (targetUid, query) => utilsStore.searchMention(targetUid, query),
                avatarCache,
                applyOptions: options => callback(options)
            })

            // 候选列表打开后轻量刷新一次，尽快展示刚写入缓存的头像
            refreshTimer.value = window.setTimeout(async () => {
                try {
                    await queryMentionOptions({
                        uid,
                        query: queryString || '',
                        requestId,
                        isLatest: searchScheduler.isLatest,
                        searchMention: (targetUid, query) =>
                            utilsStore.searchMention(targetUid, query),
                        avatarCache,
                        applyOptions: options => callback(options)
                    })
                } catch {
                    // 轻量刷新失败时保留首屏结果
                }
            }, 500)
        } catch {
            if (!searchScheduler.isLatest(requestId)) {
                return
            }
            callback([])
        }
    })
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
