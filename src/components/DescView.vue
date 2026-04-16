<template>
    <div class="desc-view">
        <el-form-item label="简介">
            <div class="desc-editor-block">
                <div class="tiptap-shell" :class="{ disabled: disabled }">
                    <EditorContent v-if="editor" :editor="editor" class="tiptap-editor" />
                </div>
                <div
                    v-if="showMentionPicker"
                    class="mention-inline-popup"
                    :style="mentionPopupStyle"
                >
                    <div class="mention-inline-header">选择或输入你想@的人</div>
                    <div v-if="mentionLoading" class="mention-inline-empty">搜索中...</div>
                    <div v-else-if="mentionOptions.length === 0" class="mention-inline-empty">
                        暂无匹配用户
                    </div>
                    <div v-else class="mention-inline-list">
                        <div
                            v-for="item in mentionOptions"
                            :key="`${item.uid}-${item.name}-${item.groupName}`"
                            class="mention-inline-item"
                            @mousedown.prevent="handleMentionSelect(item)"
                        >
                            <div v-if="item.showGroupLabel" class="mention-inline-group">
                                {{ item.groupName }}
                            </div>
                            <div class="mention-inline-row">
                                <el-avatar :size="32" :src="item.avatarSrc" />
                                <div class="mention-inline-main">
                                    <div class="mention-inline-name">{{ item.name }}</div>
                                    <div class="mention-inline-fans">{{ item.fans }}粉丝</div>
                                </div>
                                <div class="mention-inline-meta">UID: {{ item.uid }}</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </el-form-item>

        <el-form-item label="粉丝动态">
            <el-input
                v-model="dynamicModel"
                placeholder="发布时的动态内容"
                maxlength="233"
                show-word-limit
                :disabled="disabled"
            />
        </el-form-item>
    </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { EditorContent, useEditor } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import Placeholder from '@tiptap/extension-placeholder'
import Mention from '@tiptap/extension-mention'
import { useUtilsStore } from '../stores/utils'
import {
    createMentionAvatarCache,
    createDebouncedRequestScheduler,
    queryMentionOptions
} from '../composables/useMentionSearchShared'
import type { MentionOption } from '../types/mention'
import type { DescV2Item } from '../stores/user_config'

type DescV2Token = DescV2Item

const props = defineProps<{
    desc?: string
    descV2?: DescV2Token[]
    dynamic?: string
    disabled?: boolean
    userUid?: number
}>()

const emit = defineEmits<{
    (e: 'update:desc', value: string): void
    (e: 'update:descV2', value: DescV2Token[] | undefined): void
    (e: 'update:dynamic', value: string): void
}>()

let suppressEditorSync = false
let editorReadyForEmit = false
const lastSyncedDesc = ref('')
const lastSyncedDescV2Signature = ref('[]')
const showMentionPicker = ref(false)
const mentionQuery = ref('')
const mentionRange = ref<{ from: number; to: number } | null>(null)
const mentionOptions = ref<MentionOption[]>([])
const mentionLoading = ref(false)
const mentionPopupStyle = ref<Record<string, string>>({
    left: '0px',
    top: '0px'
})
let popupPositionListenersBound = false

const userUid = computed(() => Number(props.userUid || 0))
const utilsStore = useUtilsStore()
const mentionAvatarCache = createMentionAvatarCache(async () => await utilsStore.getAvatarCacheDir())
const mentionSearchScheduler = createDebouncedRequestScheduler(220)

const dynamicModel = computed({
    get: () => props.dynamic || '',
    set: value => emit('update:dynamic', value)
})

const normalizeDescV2 = (value: DescV2Token[] | undefined): DescV2Token[] => {
    if (!Array.isArray(value)) {
        return []
    }

    return value
        .filter(item => item && (Number(item.type) === 1 || Number(item.type) === 2))
        .map(item => ({
            raw_text: String(item.raw_text || ''),
            type: Number(item.type),
            biz_id: item.biz_id ?? '',
            sub_type: Number(item.sub_type ?? 0),
            sub_biz_id: item.sub_biz_id ?? ''
        }))
}

const getDescV2Signature = (tokens: DescV2Token[]) =>
    JSON.stringify(
        tokens.map(item => ({
            raw_text: item.raw_text,
            type: Number(item.type),
            biz_id: String(item.biz_id ?? ''),
            sub_type: Number(item.sub_type ?? 0),
            sub_biz_id: String(item.sub_biz_id ?? '')
        }))
    )

const formatMentionText = (value: string) => {
    const text = String(value || '')
    if (!text) {
        return ''
    }
    const mentionText = text.startsWith('@') ? text : `@${text}`
    return `${mentionText} `
}

const buildDescTextFromDescV2 = (tokens: DescV2Token[]) => {
    return tokens
        .map(token => {
            if (Number(token.type) === 2) {
                return formatMentionText(String(token.raw_text || ''))
            }
            return String(token.raw_text || '')
        })
        .join('')
}

const buildEditorDoc = (text: string) => {
    const lines = text.split('\n')
    return {
        type: 'doc',
        content: lines.map(line => {
            if (!line) {
                return { type: 'paragraph' }
            }
            return {
                type: 'paragraph',
                content: [{ type: 'text', text: line }]
            }
        })
    }
}

const buildEditorDocFromDescV2 = (tokens: DescV2Token[]) => {
    const paragraphs: Array<{ type: 'paragraph'; content: any[] }> = [{
        type: 'paragraph',
        content: []
    }]

    const pushParagraph = () => {
        paragraphs.push({ type: 'paragraph', content: [] })
    }

    const appendText = (rawText: string) => {
        const parts = String(rawText || '').split('\n')
        parts.forEach((part, index) => {
            if (part.length > 0) {
                paragraphs[paragraphs.length - 1].content.push({
                    type: 'text',
                    text: part
                })
            }
            if (index < parts.length - 1) {
                pushParagraph()
            }
        })
    }

    for (const token of tokens) {
        if (Number(token.type) === 2) {
            paragraphs[paragraphs.length - 1].content.push({
                type: 'mention',
                attrs: {
                    id: String(token.biz_id ?? ''),
                    label: String(token.raw_text || '')
                }
            })
        } else {
            appendText(String(token.raw_text || ''))
        }
    }

    return {
        type: 'doc',
        content: paragraphs.map(paragraph =>
            paragraph.content.length > 0 ? paragraph : { type: 'paragraph' }
        )
    }
}

const setEditorDoc = (doc: any) => {
    if (!editor.value) {
        return
    }
    suppressEditorSync = true
    try {
        editor.value.commands.setContent(doc, { emitUpdate: false })
    } finally {
        suppressEditorSync = false
    }
}

const hideMentionPicker = () => {
    showMentionPicker.value = false
    mentionQuery.value = ''
    mentionRange.value = null
    mentionLoading.value = false
    mentionOptions.value = []
    mentionSearchScheduler.cancel()
}

const refreshMentionPopupPosition = () => {
    updateMentionPopupPosition(editor.value)
}

const bindPopupPositionListeners = () => {
    if (popupPositionListenersBound) {
        return
    }
    window.addEventListener('scroll', refreshMentionPopupPosition, true)
    window.addEventListener('resize', refreshMentionPopupPosition)
    popupPositionListenersBound = true
}

const unbindPopupPositionListeners = () => {
    if (!popupPositionListenersBound) {
        return
    }
    window.removeEventListener('scroll', refreshMentionPopupPosition, true)
    window.removeEventListener('resize', refreshMentionPopupPosition)
    popupPositionListenersBound = false
}

const updateMentionPopupPosition = (instance: any) => {
    if (!instance || !mentionRange.value) {
        return
    }

    try {
        const coords = instance.view.coordsAtPos(mentionRange.value.to)
        const popupWidth = 320
        const margin = 8
        let left = coords.left
        if (left + popupWidth > window.innerWidth - margin) {
            left = Math.max(margin, window.innerWidth - popupWidth - margin)
        }
        const top = Math.min(window.innerHeight - margin, coords.bottom + 6)
        mentionPopupStyle.value = {
            left: `${Math.max(margin, left)}px`,
            top: `${top}px`
        }
    } catch {
        // Ignore transient position errors during editor updates.
    }
}

const searchMentionOptions = (query: string) => {
    if (!showMentionPicker.value || !userUid.value) {
        return
    }

    mentionSearchScheduler.run(async requestId => {
        mentionLoading.value = true
        try {
            await queryMentionOptions({
                uid: userUid.value,
                query,
                requestId,
                isLatest: mentionSearchScheduler.isLatest,
                searchMention: (targetUid, keyword) => utilsStore.searchMention(targetUid, keyword),
                avatarCache: mentionAvatarCache,
                applyOptions: options => {
                    mentionOptions.value = options
                }
            })
        } catch {
            if (!mentionSearchScheduler.isLatest(requestId)) {
                return
            }
            mentionOptions.value = []
        } finally {
            if (mentionSearchScheduler.isLatest(requestId)) {
                mentionLoading.value = false
            }
        }
    })
}

const updateMentionTriggerState = (instance: any) => {
    if (!instance || props.disabled || !userUid.value) {
        hideMentionPicker()
        return
    }

    const { selection } = instance.state
    if (!selection.empty) {
        hideMentionPicker()
        return
    }

    const from = selection.from
    const parentPrefix = selection.$from.parent.textBetween(0, selection.$from.parentOffset, '', '')
    const atIndex = parentPrefix.lastIndexOf('@')

    if (atIndex < 0) {
        hideMentionPicker()
        return
    }

    const query = parentPrefix.slice(atIndex + 1)
    if (/[\s]/.test(query)) {
        hideMentionPicker()
        return
    }

    const shouldSearch = !showMentionPicker.value || mentionQuery.value !== query
    showMentionPicker.value = true
    mentionQuery.value = query
    mentionRange.value = {
        from: from - query.length - 1,
        to: from
    }
    updateMentionPopupPosition(instance)
    if (shouldSearch) {
        searchMentionOptions(query)
    }
}

const handleMentionSelect = (item: { uid: string; name: string }) => {
    if (!editor.value || !mentionRange.value) {
        return
    }

    const range = mentionRange.value
    const uid = String(item.uid || '')
    const name = String(item.name || '')
    if (!uid || !name) {
        return
    }

    const currentTriggerText = editor.value.state.doc.textBetween(range.from, range.to, '', '')
    if (!currentTriggerText.startsWith('@')) {
        hideMentionPicker()
        return
    }

    editor.value
        .chain()
        .focus()
        .insertContentAt(range, [
            {
                type: 'mention',
                attrs: {
                    id: uid,
                    label: name
                }
            },
            {
                type: 'text',
                text: ' '
            }
        ])
        .run()

    hideMentionPicker()
}


const resolveEditorDocFromProps = (desc: string | undefined, descV2: DescV2Token[] | undefined) => {
    const normalized = normalizeDescV2(descV2)
    if (normalized.length > 0) {
        return buildEditorDocFromDescV2(normalized)
    }
    return buildEditorDoc(desc || '')
}

const buildDescV2FromEditorDoc = (doc: any): DescV2Token[] => {
    const result: DescV2Token[] = []
    const paragraphs = Array.isArray(doc?.content) ? doc.content : []

    const appendTextToken = (rawText: string) => {
        if (!rawText) {
            return
        }

        const lastItem = result[result.length - 1]
        if (lastItem && Number(lastItem.type) === 1) {
            lastItem.raw_text = String(lastItem.raw_text || '') + rawText
            return
        }

        result.push({
            raw_text: rawText,
            type: 1,
            biz_id: '',
            sub_type: 0,
            sub_biz_id: ''
        })
    }

    paragraphs.forEach((paragraph: any, paragraphIndex: number) => {
        const content = Array.isArray(paragraph?.content) ? paragraph.content : []

        content.forEach((node: any) => {
            if (node?.type === 'mention') {
                result.push({
                    raw_text: String(node?.attrs?.label ?? ''),
                    type: 2,
                    biz_id: String(node?.attrs?.id ?? ''),
                    sub_type: 0,
                    sub_biz_id: ''
                })
                return
            }

            if (node?.type === 'text') {
                appendTextToken(String(node?.text ?? ''))
                return
            }
        })

        if (paragraphIndex < paragraphs.length - 1) {
            appendTextToken('\n')
        }
    })

    return result
}

const editor = useEditor({
    extensions: [
        StarterKit,
        Placeholder.configure({
            placeholder: '请输入视频简介'
        }),
        Mention.configure({
            HTMLAttributes: {
                class: 'desc-mention'
            },
            renderText({ node }) {
                return formatMentionText(String(node.attrs.label || ''))
            }
        })
    ],
    content: {
        type: 'doc',
        content: [{ type: 'paragraph' }]
    },
    editorProps: {
        handleKeyDown(_view, event) {
            if (
                showMentionPicker.value &&
                event.key === ' ' &&
                !event.isComposing &&
                mentionOptions.value.length > 0
            ) {
                event.preventDefault()
                handleMentionSelect(mentionOptions.value[0])
                return true
            }
            return false
        }
    },
    editable: !props.disabled,
    onUpdate({ editor: instance }) {
        if (suppressEditorSync || !editorReadyForEmit) {
            return
        }

        updateMentionTriggerState(instance)

        const nextDescV2 = buildDescV2FromEditorDoc(instance.getJSON())
        const hasMention = nextDescV2.some(item => Number(item.type) === 2)
        const nextDesc = hasMention
            ? buildDescTextFromDescV2(nextDescV2)
            : instance.getText({ blockSeparator: '\n' })

        emit('update:desc', nextDesc)
        emit('update:descV2', hasMention ? nextDescV2 : undefined)

        lastSyncedDesc.value = nextDesc
        lastSyncedDescV2Signature.value = hasMention ? getDescV2Signature(nextDescV2) : '[]'
    },
    onSelectionUpdate({ editor: instance }) {
        if (!editorReadyForEmit || suppressEditorSync) {
            return
        }
        updateMentionTriggerState(instance)
    },
    onBlur() {
        hideMentionPicker()
    }
})

watch(
    () => editor.value,
    instance => {
        if (!instance) {
            return
        }
        editorReadyForEmit = false
        instance.setEditable(!props.disabled)
        const normalizedDescV2 = normalizeDescV2(props.descV2)
        const incomingDesc = props.desc || ''
        lastSyncedDesc.value = incomingDesc
        lastSyncedDescV2Signature.value = getDescV2Signature(normalizedDescV2)
        setEditorDoc(resolveEditorDocFromProps(incomingDesc, normalizedDescV2))
        editorReadyForEmit = true
        updateMentionTriggerState(instance)
    },
    { immediate: true }
)

onBeforeUnmount(() => {
    hideMentionPicker()
    unbindPopupPositionListeners()
    mentionSearchScheduler.dispose()
    mentionAvatarCache.dispose()
    editor.value?.destroy()
})

watch(showMentionPicker, value => {
    if (value) {
        bindPopupPositionListeners()
        refreshMentionPopupPosition()
        return
    }
    unbindPopupPositionListeners()
})

watch(
    () => props.disabled,
    value => {
        editor.value?.setEditable(!value)
        if (value) {
            hideMentionPicker()
        }
    },
    { immediate: true }
)

watch(
    () => props.userUid,
    value => {
        if (!Number(value || 0)) {
            hideMentionPicker()
        }
    }
)

watch(
    [() => props.desc, () => props.descV2],
    ([nextDesc, nextDescV2]) => {
        const incomingDesc = nextDesc || ''
        const normalizedDescV2 = normalizeDescV2(nextDescV2)
        const incomingDescV2Signature = getDescV2Signature(normalizedDescV2)

        if (
            incomingDesc === lastSyncedDesc.value &&
            incomingDescV2Signature === lastSyncedDescV2Signature.value
        ) {
            return
        }

        lastSyncedDesc.value = incomingDesc
        lastSyncedDescV2Signature.value = incomingDescV2Signature
        editorReadyForEmit = false
        setEditorDoc(resolveEditorDocFromProps(incomingDesc, normalizedDescV2))
        editorReadyForEmit = true
        updateMentionTriggerState(editor.value)
    }
)
</script>

<style scoped>
.desc-editor-block {
    width: 100%;
}

.tiptap-shell {
    position: relative;
    border: 1px solid #dcdfe6;
    border-radius: 6px;
    min-height: 150px;
    background: #fff;
}

.tiptap-shell:focus-within {
    border-color: #409eff;
}

.tiptap-shell.disabled {
    background: #f5f7fa;
    border-color: #e4e7ed;
}

.tiptap-editor {
    min-height: 150px;
}

.tiptap-editor :deep(.tiptap) {
    min-height: 150px;
    padding: 10px 12px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
    outline: none;
}

.tiptap-editor :deep(.tiptap p.is-editor-empty:first-child::before) {
    content: attr(data-placeholder);
    color: #c0c4cc;
    pointer-events: none;
    height: 0;
    float: left;
}

.tiptap-editor :deep(.desc-mention) {
    color: #409eff;
    background: rgba(64, 158, 255, 0.12);
    border-radius: 4px;
    padding: 0 2px;
}

.mention-inline-popup {
    position: fixed;
    width: 320px;
    max-height: 320px;
    overflow: hidden;
    border: 1px solid #dcdfe6;
    border-radius: 8px;
    background: #fff;
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.14);
    z-index: 2200;
}

.mention-inline-header {
    padding: 10px 12px 6px;
    font-size: 14px;
    color: #606266;
    border-bottom: 1px solid #f0f2f5;
}

.mention-inline-list {
    max-height: 270px;
    overflow-y: auto;
}

.mention-inline-item {
    padding: 8px 12px;
    cursor: pointer;
}

.mention-inline-item:hover {
    background: #f5f7fa;
}

.mention-inline-group {
    font-size: 12px;
    color: #909399;
    margin-bottom: 4px;
}

.mention-inline-row {
    display: flex;
    align-items: center;
    gap: 10px;
}

.mention-inline-main {
    min-width: 0;
    flex: 1;
}

.mention-inline-name {
    color: #303133;
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.mention-inline-meta,
.mention-inline-fans {
    color: #909399;
    font-size: 12px;
}

.mention-inline-meta {
    flex-shrink: 0;
}

.mention-inline-empty {
    padding: 12px;
    color: #909399;
    font-size: 13px;
}
</style>
