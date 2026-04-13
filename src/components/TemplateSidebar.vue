<template>
    <el-aside width="300px" class="sidebar">
        <div class="sidebar-header">
            <h3></h3>
            <div class="header-buttons">
                <el-checkbox
                    v-model="highlightAutoSubmitting"
                    size="small"
                    class="highlight-checkbox"
                >
                    <span class="highlight-checkbox-text"> 高亮显示<br />自动提交 </span>
                </el-checkbox>
                <el-button
                    type="success"
                    size="small"
                    @click="emit('show-login')"
                    :disabled="templateLoading"
                >
                    <el-icon><User /></el-icon>
                    登录用户
                </el-button>
                <el-button
                    type="primary"
                    size="small"
                    @click="emit('show-new-template')"
                    :disabled="!loginUserCount || templateLoading"
                >
                    <el-icon><Plus /></el-icon>
                    新建模板
                </el-button>
            </div>
        </div>

        <div class="sidebar-content">
            <div class="user-template-list" ref="userSectionListRef">
                <div
                    v-for="userTemplate in userTemplates"
                    :key="userTemplate.user.uid"
                    class="user-section"
                >
                    <div
                        class="user-header"
                        @click="handleUserExpansion(userTemplate.user.uid)"
                        :class="{
                            disabled: templateLoading || userTemplate.user.expired,
                            'user-header-expired': userTemplate.user.expired
                        }"
                    >
                        <div class="user-drag-handle" title="拖动用户排序" @click.stop>
                            <span></span>
                            <span></span>
                        </div>
                        <el-avatar
                            :src="
                                userTemplate.user.expired
                                    ? ''
                                    : `data:image/jpeg;base64,${userTemplate.user.avatar}`
                            "
                            :size="24"
                            class="user-avatar"
                            :class="{ 'user-avatar-expired': userTemplate.user.expired }"
                        >
                            {{ userTemplate.user.username.charAt(0) }}
                        </el-avatar>
                        <span class="user-name">{{ userTemplate.user.username }}</span>
                        <el-icon
                            class="config-icon"
                            :class="{ disabled: userTemplate.user.expired }"
                            @click.stop="emit('open-user-config', userTemplate.user)"
                            title="用户配置"
                        >
                            <Setting />
                        </el-icon>
                        <el-badge
                            :value="userTemplate.templates.length"
                            class="template-count-badge"
                        />
                        <el-icon class="expand-icon" :class="{ expanded: userTemplate.expanded }">
                            <ArrowDown />
                        </el-icon>
                    </div>

                    <div
                        class="template-list"
                        v-show="userTemplate.expanded"
                        :ref="el => setTemplateListRef(userTemplate.user.uid, el)"
                    >
                        <div class="template-list-toolbar">
                            <span class="template-list-hint"></span>
                            <el-dropdown
                                v-if="userTemplate.templates.length > 1"
                                trigger="click"
                                :disabled="templateLoading || userTemplate.user.expired"
                                @command="
                                    (command: string) =>
                                        handleTemplateSortCommand(userTemplate.user.uid, command)
                                "
                            >
                                <el-button link size="small" class="template-sort-btn">
                                    排序
                                </el-button>
                                <template #dropdown>
                                    <el-dropdown-menu>
                                        <el-dropdown-item command="name-asc"
                                            >模板名正序</el-dropdown-item
                                        >
                                        <el-dropdown-item command="name-desc"
                                            >模板名倒序</el-dropdown-item
                                        >
                                        <el-dropdown-item command="recent-saved"
                                            >最近保存正序</el-dropdown-item
                                        >
                                        <el-dropdown-item command="recent-saved-desc"
                                            >最近保存倒序</el-dropdown-item
                                        >
                                    </el-dropdown-menu>
                                </template>
                            </el-dropdown>
                        </div>
                        <div
                            v-for="template in userTemplate.templates"
                            :key="`${userTemplate.user.uid}-${template.name}`"
                            class="template-item"
                            :class="{
                                active:
                                    selectedUserUid === userTemplate.user.uid &&
                                    currentTemplateName === template.name,
                                'auto-submitting':
                                    highlightAutoSubmitting &&
                                    autoSubmittingRecord[
                                        getTemplateKey(userTemplate.user.uid, template.name)
                                    ],
                                'auto-submitting-simple':
                                    !highlightAutoSubmitting &&
                                    autoSubmittingRecord[
                                        getTemplateKey(userTemplate.user.uid, template.name)
                                    ],
                                'template-loading':
                                    templateLoading &&
                                    selectedUserUid === userTemplate.user.uid &&
                                    currentTemplateName === template.name,
                                disabled: templateLoading || userTemplate.user.expired
                            }"
                            @click="handleTemplateSelection(userTemplate.user, template.name)"
                        >
                            <div class="template-main">
                                <div class="template-name">
                                    {{ template.name }}
                                    <span
                                        v-show="
                                            hasUnsavedChanges(userTemplate.user.uid, template.name)
                                        "
                                        class="unsaved-indicator"
                                        title="有未保存的修改"
                                    ></span>
                                </div>
                                <div class="template-desc">
                                    {{ template.config.title || '无标题' }}
                                </div>
                            </div>
                            <el-dropdown
                                @command="
                                    (command: string) =>
                                        emit(
                                            'template-command',
                                            command,
                                            userTemplate.user,
                                            template
                                        )
                                "
                                @click.stop
                                trigger="click"
                                :disabled="templateLoading"
                            >
                                <el-button link size="small" class="template-menu-btn">
                                    <el-icon><MoreFilled /></el-icon>
                                </el-button>
                                <template #dropdown>
                                    <el-dropdown-menu>
                                        <el-dropdown-item command="duplicate"
                                            >复制</el-dropdown-item
                                        >
                                        <el-dropdown-item command="rename">重命名</el-dropdown-item>
                                        <el-dropdown-item command="delete" divided
                                            >删除</el-dropdown-item
                                        >
                                    </el-dropdown-menu>
                                </template>
                            </el-dropdown>
                        </div>

                        <div v-if="userTemplate.user.expired" class="expired-mask">
                            <div class="expired-mask-content">
                                <div class="expired-mask-title">账号登录状态已失效</div>
                                <div class="expired-mask-desc">请重新登录后再编辑或选择模板</div>
                                <el-button
                                    type="primary"
                                    size="small"
                                    @click.stop="emit('show-login')"
                                >
                                    重新登录
                                </el-button>
                            </div>
                        </div>
                    </div>
                </div>

                <div v-if="userTemplates.length === 0" class="empty-users">
                    <el-empty description="暂无登录用户">
                        <el-button type="primary" @click="emit('show-login')">去登录</el-button>
                    </el-empty>
                </div>
            </div>
        </div>
    </el-aside>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { ArrowDown, MoreFilled, Plus, Setting, User } from '@element-plus/icons-vue'
import Sortable, { type SortableEvent } from 'sortablejs'
import { useUserConfigStore } from '../stores/user_config'
import { useAuthStore } from '../stores/auth'
import { useUserOrderSortable } from '../composables/useUserOrderSortable'

interface TemplateUser {
    uid: number
    username: string
    avatar: string
    expired: boolean
}

interface TemplateEntry {
    name: string
    config: {
        title?: string
    }
}

interface UserTemplateGroup {
    user: TemplateUser
    templates: TemplateEntry[]
    expanded: boolean
}

const props = defineProps<{
    userTemplates: UserTemplateGroup[]
    templateLoading: boolean
    autoSubmittingRecord: Record<string, boolean>
    selectedUserUid: number | null
    currentTemplateName: string
    loginUserCount: number
    hasUnsavedChanges: (uid: number, templateName: string) => boolean
}>()

const emit = defineEmits<{
    (e: 'show-login'): void
    (e: 'show-new-template'): void
    (e: 'open-user-config', user: TemplateUser): void
    (e: 'select-template', user: TemplateUser, templateName: string): void
    (e: 'template-command', command: string, user: TemplateUser, template: TemplateEntry): void
}>()

const userConfigStore = useUserConfigStore()
const authStore = useAuthStore()
const templateListRefs = ref<Record<number, HTMLElement | null>>({})
const templateListSortables = new Map<number, Sortable>()
const userSectionListRef = ref<HTMLElement | null>(null)
const skipNextTemplateSelection = ref(false)
const highlightAutoSubmitting = ref<boolean>(
    localStorage.getItem('highlightAutoSubmitting') === 'true'
)
let suppressTemplateSelectionUntil = 0

watch(highlightAutoSubmitting, newValue => {
    localStorage.setItem('highlightAutoSubmitting', String(newValue))
})

const userTemplates = computed(() => props.userTemplates)
const templateLoading = computed(() => props.templateLoading)
const autoSubmittingRecord = computed(() => props.autoSubmittingRecord)
const selectedUserUid = computed(() => props.selectedUserUid)
const currentTemplateName = computed(() => props.currentTemplateName)
const loginUserCount = computed(() => props.loginUserCount)
const hasUnsavedChanges = computed(() => props.hasUnsavedChanges)

const getTemplateKey = (uid: number, templateName: string) => `${uid}-${templateName}`

const setTemplateListRef = (userUid: number, element: Element | { $el?: Element } | null) => {
    if (element instanceof Element) {
        templateListRefs.value[userUid] = element as HTMLElement
        return
    }

    if (element && '$el' in element && element.$el instanceof Element) {
        templateListRefs.value[userUid] = element.$el as HTMLElement
        return
    }

    templateListRefs.value[userUid] = null
}

const handleUserExpansion = (userUid: number) => {
    const targetUser = userTemplates.value.find(user => user.user.uid === userUid)?.user
    if (targetUser?.expired) {
        emit('show-login')
        ElMessage.warning('该用户登录状态已过期，请重新登录')
        return
    }

    if (!templateLoading.value) {
        userConfigStore.toggleUserExpanded(userUid)
    }
}

const handleTemplateSelection = (user: TemplateUser, templateName: string) => {
    if (Date.now() < suppressTemplateSelectionUntil) {
        return
    }

    if (skipNextTemplateSelection.value) {
        skipNextTemplateSelection.value = false
        return
    }

    if (user.expired) {
        emit('show-login')
        ElMessage.warning('该用户登录状态已过期，请重新登录')
        return
    }

    if (!templateLoading.value) {
        emit('select-template', user, templateName)
    }
}

const isTemplateDragEnabled = (userTemplate: UserTemplateGroup) => {
    return !templateLoading.value && !userTemplate.user.expired && userTemplate.templates.length > 1
}

const canDragUserSections = computed(() => !templateLoading.value && userTemplates.value.length > 1)

const destroyTemplateSortables = () => {
    for (const sortable of templateListSortables.values()) {
        sortable.destroy()
    }
    templateListSortables.clear()
}

const destroyTemplateSortable = (userUid: number) => {
    const sortable = templateListSortables.get(userUid)
    if (!sortable) {
        return
    }

    sortable.destroy()
    templateListSortables.delete(userUid)
}

const { syncSortable: syncUserSectionSortable, destroySortable: destroyUserSectionSortable } =
    useUserOrderSortable({
        containerRef: userSectionListRef,
        enabled: canDragUserSections,
        getCurrentOrder: () => userTemplates.value.map(item => item.user.uid),
        onOrderChange: async nextOrder => {
            try {
                await authStore.reorderLoginUsers(nextOrder)
                ElMessage.success('用户顺序已保存')
            } catch (error) {
                console.error('用户排序失败:', error)
                ElMessage.error(`用户排序失败: ${error}`)
                await syncUserSectionSortable()
            }
        },
        sortable: {
            animation: 180,
            draggable: '.user-section',
            handle: '.user-drag-handle',
            chosenClass: 'user-drag-source',
            ghostClass: 'user-drag-placeholder',
            dragClass: 'user-drag-clone',
            fallbackClass: 'user-drag-clone',
            forceFallback: true,
            fallbackOnBody: true,
            fallbackTolerance: 4
        }
    })

const createTemplateSortable = (userTemplate: UserTemplateGroup, container: HTMLElement) => {
    const userUid = userTemplate.user.uid

    destroyTemplateSortable(userUid)

    const sortable = Sortable.create(container, {
        animation: 180,
        draggable: '.template-item',
        handle: '.template-main',
        chosenClass: 'drag-source',
        dragClass: 'drag-clone',
        ghostClass: 'drag-placeholder',
        fallbackClass: 'drag-clone',
        forceFallback: true,
        fallbackOnBody: true,
        fallbackTolerance: 4,
        disabled: !isTemplateDragEnabled(userTemplate),
        onStart: () => {
            suppressTemplateSelectionUntil = Date.now() + 300
        },
        onEnd: async (event: SortableEvent) => {
            suppressTemplateSelectionUntil = Date.now() + 300

            const fromIndex = event.oldDraggableIndex ?? event.oldIndex
            const toIndex = event.newDraggableIndex ?? event.newIndex

            if (
                typeof fromIndex !== 'number' ||
                typeof toIndex !== 'number' ||
                fromIndex === toIndex
            ) {
                return
            }

            const orderedTemplates = userConfigStore
                .getUserTemplates(userUid)
                .map(template => template.name)
            const nextOrder = [...orderedTemplates]
            const [movedTemplate] = nextOrder.splice(fromIndex, 1)

            if (!movedTemplate) {
                return
            }

            nextOrder.splice(toIndex, 0, movedTemplate)

            try {
                skipNextTemplateSelection.value = true
                await userConfigStore.reorderUserTemplates(userUid, nextOrder)
                ElMessage.success('模板顺序已保存')
            } catch (error) {
                console.error('拖动排序失败:', error)
                ElMessage.error(`拖动排序失败: ${error}`)
                void syncTemplateSortables()
            } finally {
                window.setTimeout(() => {
                    skipNextTemplateSelection.value = false
                }, 0)
            }
        }
    })

    templateListSortables.set(userUid, sortable)
}

const syncTemplateSortables = async () => {
    await nextTick()

    const activeUserIds = new Set(userTemplates.value.map(userTemplate => userTemplate.user.uid))

    for (const userUid of templateListSortables.keys()) {
        if (!activeUserIds.has(userUid)) {
            destroyTemplateSortable(userUid)
        }
    }

    for (const userTemplate of userTemplates.value) {
        const container = templateListRefs.value[userTemplate.user.uid]
        if (!container) {
            destroyTemplateSortable(userTemplate.user.uid)
            continue
        }

        if (!isTemplateDragEnabled(userTemplate)) {
            destroyTemplateSortable(userTemplate.user.uid)
            continue
        }

        createTemplateSortable(userTemplate, container)
    }
}

const handleTemplateSortCommand = async (userUid: number, command: string) => {
    if (templateLoading.value) {
        return
    }

    try {
        if (['name-asc', 'name-desc', 'recent-saved', 'recent-saved-desc'].includes(command)) {
            await userConfigStore.sortUserTemplates(
                userUid,
                command as 'name-asc' | 'name-desc' | 'recent-saved' | 'recent-saved-desc'
            )
            ElMessage.success('模板顺序已更新')
        }
    } catch (error) {
        console.error('模板排序失败:', error)
        ElMessage.error(`模板排序失败: ${error}`)
    }
}

watch(
    () => props.userTemplates,
    () => {
        void syncTemplateSortables()
        void syncUserSectionSortable()
    },
    { deep: true }
)

watch(
    () => props.templateLoading,
    () => {
        void syncTemplateSortables()
        void syncUserSectionSortable()
    }
)

onMounted(async () => {
    await syncTemplateSortables()
    await syncUserSectionSortable()
})

onUnmounted(() => {
    destroyTemplateSortables()
    destroyUserSectionSortable()
})
</script>

<style scoped>
.sidebar {
    background: #f5f7fa;
    border-right: 1px solid #e4e7ed;
    padding: 20px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.sidebar-content {
    flex: 1;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #c1c1c1 transparent;
}

.sidebar-content::-webkit-scrollbar {
    width: 6px;
}

.sidebar-content::-webkit-scrollbar-track {
    background: transparent;
}

.sidebar-content::-webkit-scrollbar-thumb {
    background-color: #c1c1c1;
    border-radius: 3px;
}

.sidebar-content::-webkit-scrollbar-thumb:hover {
    background-color: #a8a8a8;
}

.sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
}

.sidebar-header h3 {
    margin: 0;
    color: #303133;
}

.header-buttons {
    display: flex;
    gap: 8px;
    align-items: center;
}

.highlight-checkbox {
    margin-right: 8px;
    font-size: 12px;
}

.highlight-checkbox :deep(.el-checkbox__label) {
    font-size: 12px;
    color: #606266;
}

.highlight-checkbox-text {
    line-height: 1.2;
    text-align: center;
}

.user-section {
    margin-bottom: 10px;
}

.user-header {
    display: flex;
    align-items: center;
    padding: 10px;
    background: #fff;
    border-radius: 6px;
    cursor: pointer;
    transition: background-color 0.3s;
}

.user-drag-handle {
    width: 18px;
    height: 18px;
    margin-right: 8px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 2px;
    cursor: grab;
    flex-shrink: 0;
}

.user-drag-handle:active {
    cursor: grabbing;
}

.user-drag-handle span {
    width: 10px;
    height: 2px;
    border-radius: 999px;
    background: #94a3b8;
}

.user-header:hover {
    background: #ecf5ff;
}

.user-header.user-header-expired {
    background: #f3f4f6;
}

.user-header.user-header-expired:hover {
    background: #f3f4f6;
}

.user-header.disabled {
    cursor: not-allowed;
    opacity: 0.7;
}

.user-header.disabled:hover {
    background: #fff;
}

.user-avatar {
    margin-right: 10px;
}

.user-avatar.user-avatar-expired {
    background: #000000;
    color: #ffffff;
}

.user-name {
    flex: 1;
    font-weight: 500;
}

.template-count-badge {
    margin-right: 10px;
}

.template-count-badge :deep(.el-badge__content) {
    background-color: #909399 !important;
    color: #ffffff !important;
    border: none !important;
}

.expand-icon {
    transition: transform 0.3s;
}

.expand-icon.expanded {
    transform: rotate(180deg);
}

.config-icon {
    color: #909399;
    cursor: pointer;
    margin-left: 8px;
    margin-right: 4px;
    transition: color 0.3s;
}

.config-icon:hover {
    color: #409eff;
}

.config-icon.disabled {
    color: #c0c4cc;
    cursor: not-allowed;
}

.config-icon.disabled:hover {
    color: #c0c4cc;
}

.template-list {
    margin-left: 20px;
    margin-top: 10px;
    position: relative;
}

.template-list-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
    padding-right: 4px;
}

.template-list-hint {
    font-size: 12px;
    color: #909399;
}

.template-sort-btn {
    padding: 0;
}

.template-item {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    background: #fff;
    border-radius: 4px;
    margin-bottom: 5px;
    cursor: pointer;
    transition: all 0.3s;
}

.template-item:hover {
    background: #f0f9ff;
}

.template-item.drag-source {
    opacity: 0 !important;
    box-shadow: none !important;
    border-color: transparent !important;
}

.template-item.drag-clone {
    opacity: 0.95;
    box-shadow: 0 10px 24px rgba(15, 23, 42, 0.18);
    transform: rotate(1deg);
}

.template-item.drag-placeholder {
    background: #f0f9ff;
    box-shadow: inset 0 0 0 1px #409eff;
}

.template-item.active {
    background: #ecf5ff;
    border-left: 3px solid #409eff;
}

.template-item.auto-submitting {
    position: relative;
    overflow: hidden;
    background: linear-gradient(45deg, #e3f2fd, #f3e5f5);
    border: 2px solid #409eff;
    box-shadow: 0 0 20px rgba(64, 158, 255, 0.4);
    animation: pulse-border 1.5s ease-in-out infinite alternate;
}

.template-item.auto-submitting::before {
    content: '';
    position: absolute;
    top: 0;
    left: -100%;
    width: 100%;
    height: 100%;
    background: linear-gradient(90deg, transparent, rgba(64, 158, 255, 0.6), transparent);
    animation: shimmer 1.5s infinite;
    z-index: 1;
}

.template-item.auto-submitting::after {
    content: '⚡ 自动上传中...';
    position: absolute;
    top: 2px;
    right: 6px;
    font-size: 10px;
    color: #409eff;
    font-weight: bold;
    animation: blink 1s infinite;
    z-index: 3;
}

.template-item.auto-submitting .template-main {
    position: relative;
    z-index: 2;
}

.template-item.auto-submitting .template-name {
    color: #1976d2;
    font-weight: 600;
    text-shadow: 0 1px 3px rgba(25, 118, 210, 0.2);
}

.template-item.auto-submitting-simple {
    border: 2px solid #409eff !important;
    background: rgba(64, 158, 255, 0.05) !important;
    position: relative;
}

.template-item.auto-submitting-simple::after {
    content: '⚡ 自动上传中...';
    position: absolute;
    top: 2px;
    right: 6px;
    font-size: 10px;
    color: #409eff;
    font-weight: bold;
    z-index: 3;
}

.template-item.template-loading {
    position: relative;
    background: #f5f7fa;
    border: 2px solid #e4e7ed;
    cursor: not-allowed;
    opacity: 0.8;
}

.template-item.template-loading::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: linear-gradient(90deg, transparent, rgba(230, 244, 255, 0.8), transparent);
    animation: loading-shimmer 1.5s infinite;
    z-index: 1;
}

.template-item.template-loading::after {
    content: '🔄 加载中...';
    position: absolute;
    top: 2px;
    right: 6px;
    font-size: 10px;
    color: #909399;
    font-weight: bold;
    z-index: 3;
    animation: loading-spin 1s linear infinite;
}

.template-item.template-loading .template-main {
    position: relative;
    z-index: 2;
}

.template-item.disabled {
    cursor: not-allowed;
    opacity: 0.55;
    pointer-events: none;
}

.template-item.disabled:hover {
    background: #fff;
}

.expired-mask {
    position: absolute;
    inset: 0;
    background: rgba(17, 24, 39, 0.45);
    backdrop-filter: blur(1px);
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 8;
}

.expired-mask-content {
    text-align: center;
    color: #ffffff;
    padding: 12px;
}

.expired-mask-title {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 6px;
}

.expired-mask-desc {
    font-size: 12px;
    opacity: 0.9;
    margin-bottom: 10px;
}

.template-main {
    flex: 1;
}

.template-name {
    font-weight: 500;
    color: #303133;
    display: flex;
    align-items: center;
    gap: 6px;
}

.unsaved-indicator {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: #f56c6c;
    flex-shrink: 0;
    animation: pulse-red 2s infinite;
}

.template-desc {
    font-size: 12px;
    color: #909399;
    margin-top: 2px;
}

.user-drag-source {
    opacity: 0 !important;
    box-shadow: none !important;
}

.user-drag-placeholder {
    border-radius: 6px;
    background: #e0f2fe;
    box-shadow: inset 0 0 0 1px #60a5fa;
}

.user-drag-clone {
    opacity: 0.95;
    box-shadow: 0 10px 24px rgba(15, 23, 42, 0.18);
    transform: rotate(1deg);
}

.empty-users {
    text-align: center;
    margin-top: 50px;
}

@keyframes shimmer {
    0% {
        left: -100%;
    }
    100% {
        left: 100%;
    }
}

@keyframes pulse-border {
    0% {
        border-color: #409eff;
        box-shadow: 0 0 20px rgba(64, 158, 255, 0.4);
    }
    100% {
        border-color: #1890ff;
        box-shadow: 0 0 30px rgba(24, 144, 255, 0.6);
    }
}

@keyframes blink {
    0%,
    50% {
        opacity: 1;
    }
    51%,
    100% {
        opacity: 0.3;
    }
}

@keyframes loading-shimmer {
    0% {
        transform: translateX(-100%);
    }
    100% {
        transform: translateX(100%);
    }
}

@keyframes loading-spin {
    0% {
        opacity: 1;
    }
    50% {
        opacity: 0.5;
    }
    100% {
        opacity: 1;
    }
}

@keyframes pulse-red {
    0% {
        opacity: 1;
        transform: scale(1);
    }
    50% {
        opacity: 0.7;
        transform: scale(1.1);
    }
    100% {
        opacity: 1;
        transform: scale(1);
    }
}
</style>
