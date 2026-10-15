<template>
    <div ref="root" class="episode-rows">
        <div class="section-toolbar">
            <el-button
                type="primary"
                :disabled="busy || loading || dirty || !ready"
                @click="emit('add')"
                >{{ t('添加单集') }}</el-button
            >
            <el-button :disabled="busy || loading || dirty" @click="emit('refresh')">{{
                t('刷新')
            }}</el-button>
            <el-button type="success" :disabled="busy || loading || !dirty" @click="emit('save')">{{
                t('保存排序')
            }}</el-button>
            <el-button v-if="dirty" :disabled="busy" @click="emit('reset')">{{
                t('取消排序')
            }}</el-button>
        </div>
        <el-alert v-if="error" :title="error" type="error" :closable="false" show-icon />
        <el-table
            v-loading="loading || busy"
            :data="episodes"
            row-key="id"
            :empty-text="t('暂无稿件')"
        >
            <el-table-column :label="t('序号')" width="120" align="center">
                <template #default="{ $index }">
                    <el-input-number
                        :model-value="$index + 1"
                        :min="1"
                        :max="episodes.length"
                        :precision="0"
                        :controls="false"
                        :disabled="busy || loading"
                        style="width: 80px"
                        @change="(value: number | undefined) => emit('position', $index, value)"
                    />
                </template>
            </el-table-column>
            <el-table-column :label="t('单集标题')" min-width="220">
                <template #default="{ row }"
                    ><el-button
                        link
                        :disabled="busy || loading || dirty"
                        @click="emit('title', row)"
                    >
                        <span class="episode-title" :title="row.title">{{
                            row.title || t('无标题')
                        }}</span>
                        <el-icon><Edit /></el-icon> </el-button
                ></template>
            </el-table-column>
            <el-table-column :label="t('稿件标题')" min-width="240" show-overflow-tooltip>
                <template #default="{ row }">{{ row.archiveTitle || '—' }}</template>
            </el-table-column>
            <el-table-column :label="t('操作')" width="180" align="center">
                <template #default="{ row }">
                    <el-button
                        link
                        type="primary"
                        :disabled="busy || loading || dirty"
                        @click="emit('edit', row)"
                        >{{ t('编辑稿件') }}</el-button
                    >
                    <el-button
                        type="danger"
                        link
                        :disabled="busy || loading || dirty"
                        @click="emit('remove', row)"
                        >{{ t('移出合集') }}</el-button
                    >
                </template>
            </el-table-column>
            <el-table-column :label="t('排序')" width="70" align="center">
                <template #default>
                    <el-button
                        class="episode-drag-handle"
                        link
                        :disabled="busy || loading || !ready || episodes.length < 2"
                        :aria-label="t('拖动排序')"
                        :title="t('拖动排序')"
                        ><el-icon><Sort /></el-icon
                    ></el-button>
                </template>
            </el-table-column>
        </el-table>
    </div>
</template>

<script setup lang="ts">
import { t } from '../i18n'
import { Edit, Sort } from '@element-plus/icons-vue'
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import Sortable from 'sortablejs'
import type { SeasonEpisode } from '../types/season'

const props = defineProps<{
    episodes: SeasonEpisode[]
    loading: boolean
    busy: boolean
    error: string
    dirty: boolean
    ready: boolean
}>()
const emit = defineEmits<{
    add: []
    refresh: []
    save: []
    reset: []
    position: [index: number, position: number | undefined]
    title: [episode: SeasonEpisode]
    edit: [episode: SeasonEpisode]
    remove: [episode: SeasonEpisode]
}>()

const root = ref<HTMLElement | null>(null)
let sortable: Sortable | null = null
const dragDisabled = () => props.busy || props.loading || !props.ready || props.episodes.length < 2

onMounted(async () => {
    await nextTick()
    const body = root.value?.querySelector<HTMLElement>('.el-table__body-wrapper tbody')
    if (!body) return
    sortable = Sortable.create(body, {
        handle: '.episode-drag-handle',
        draggable: '.el-table__row',
        animation: 150,
        // Tauri 已启用文件拖放，需使用模拟拖动以避开原生拖放事件冲突。
        forceFallback: true,
        fallbackOnBody: true,
        fallbackTolerance: 4,
        disabled: dragDisabled(),
        onEnd: event => {
            const from = event.oldDraggableIndex ?? event.oldIndex
            const to = event.newDraggableIndex ?? event.newIndex
            if (from === undefined) return
            // 先还原 Sortable 移动的 DOM，再让 Vue 按新顺序更新，避免连续拖动后行错位。
            const rows = Array.from(event.from.children).filter(
                row => row !== event.item && row.matches('.el-table__row')
            )
            event.from.insertBefore(event.item, rows[from] ?? null)
            if (to !== undefined && from !== to && !dragDisabled()) {
                emit('position', from, to + 1)
            }
        }
    })
})

watch(dragDisabled, disabled => sortable?.option('disabled', disabled))
onBeforeUnmount(() => sortable?.destroy())
</script>

<style scoped>
.section-toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 12px 0;
}
.section-toolbar .el-button {
    margin: 0;
}
.episode-title {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.episode-drag-handle:not(.is-disabled) {
    cursor: grab;
    touch-action: none;
}
.episode-drag-handle:not(.is-disabled):active {
    cursor: grabbing;
}
</style>
