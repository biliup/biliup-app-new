<template>
    <!-- 合集选择器：使用下拉框 -->
    <div class="season-selector">
        <el-select
            v-model="selectedSeasonKey"
            placeholder="请选择合集"
            clearable
            filterable
            :loading="isSearching"
            class="season-select"
            @change="handleSeasonChange"
            @clear="clearSelection"
        >
            <el-option
                v-for="season in allSeasons"
                :key="`${season.season_id}-${season.section_id}`"
                :label="season.title"
                :value="`${season.season_id}-${season.section_id}`"
                class="season-option"
            >
                <div class="season-option-content">
                    <span class="season-option-title">{{ season.title }}</span>
                    <span class="season-option-id">ID: {{ season.season_id }}</span>
                </div>
            </el-option>
        </el-select>
    </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useUtilsStore } from '../stores/utils'
import { ElMessage } from 'element-plus'

// 定义组件props
interface Props {
    modelValue?: number // season_id
    sectionId?: number // section_id
    userUid?: number
}

const props = withDefaults(defineProps<Props>(), {})

// 定义emits
const emit = defineEmits<{
    'update:modelValue': [value: number | undefined]
    'update:sectionId': [value: number | undefined]
}>()

const utilsStore = useUtilsStore()

// 合集数据接口
interface Season {
    season_id: number
    section_id: number
    title: string
}

// 响应式数据
const allSeasons = ref<Season[]>([])
const selectedSeasonId = ref<number | undefined>(props.modelValue)
const selectedSectionId = ref<number | undefined>(props.sectionId)
const selectedSeasonKey = ref<string | undefined>()
const isSearching = ref(false)

// 计算属性

// 处理合集选择变化
const handleSeasonChange = (value: string) => {
    if (!value) {
        clearSelection()
        return
    }

    const [seasonId, sectionId] = value.split('-').map(Number)
    selectedSeasonId.value = seasonId
    selectedSectionId.value = sectionId
    selectedSeasonKey.value = value
}

// 清空选择
const clearSelection = () => {
    selectedSeasonId.value = undefined
    selectedSectionId.value = undefined
    selectedSeasonKey.value = undefined
}

// 更新selectedSeasonKey的辅助函数
const updateSelectedKey = () => {
    if (selectedSeasonId.value !== undefined && selectedSectionId.value !== undefined) {
        selectedSeasonKey.value = `${selectedSeasonId.value}-${selectedSectionId.value}`
    } else {
        selectedSeasonKey.value = undefined
    }
}

// 加载合集列表
const loadSeasons = async () => {
    if (!props.userUid) {
        return
    }

    try {
        isSearching.value = true
        await utilsStore.getSeasonList(props.userUid)

        if (utilsStore.seasonlist) {
            allSeasons.value = utilsStore.seasonlist.map((season: any) => ({
                season_id: season.season_id,
                section_id: season.section_id,
                title: season.title
            }))

            // 使用联合主键去重
            allSeasons.value = allSeasons.value.filter(
                (season, index, self) =>
                    index ===
                    self.findIndex(
                        s => s.season_id === season.season_id && s.section_id === season.section_id
                    )
            )
            updateSelectedKey()
        }
    } catch (error) {
        console.error('加载合集列表失败:', error)
        ElMessage.error(`加载合集列表失败: ${error}`)
    } finally {
        isSearching.value = false
    }
}

// 监听props变化
watch(
    () => props.modelValue,
    newValue => {
        selectedSeasonId.value = newValue
        updateSelectedKey()
    }
)

watch(
    () => props.sectionId,
    newValue => {
        selectedSectionId.value = newValue
        updateSelectedKey()
    }
)

watch(
    () => props.userUid,
    () => {
        loadSeasons()
    }
)

// 监听选中值变化，向父组件发送更新
watch(selectedSeasonId, newValue => {
    emit('update:modelValue', newValue)
})

watch(selectedSectionId, newValue => {
    emit('update:sectionId', newValue)
})

// 组件挂载时加载数据
onMounted(() => {
    selectedSeasonId.value = props.modelValue
    selectedSectionId.value = props.sectionId
    loadSeasons()
})
</script>

<style scoped>
/* 合集选择器样式 */
.season-selector {
    width: 100%;
}

.season-select {
    width: 100%;
}

/* 自定义下拉框选项样式 */
.season-option-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 8px 0;
}

.season-option-title {
    flex: 1;
    font-size: 14px;
    color: #303133;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-right: 12px;
}

.season-option-id {
    flex-shrink: 0;
    font-size: 12px;
    color: #909399;
    background: #f0f2f5;
    padding: 2px 6px;
    border-radius: 3px;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
}

/* 下拉框样式优化 */
.season-select :deep(.el-input__inner) {
    border-radius: 6px;
    border-color: #dcdfe6;
    transition: all 0.3s;
}

.season-select :deep(.el-input__inner:focus) {
    border-color: #409eff;
    box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.1);
}

.season-select :deep(.el-input__inner:hover) {
    border-color: #409eff;
}

.season-select :deep(.el-input__suffix) {
    color: #c0c4cc;
}

/* 下拉面板样式 */
:deep(.el-select-dropdown) {
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    border: 1px solid #e1e6ea;
}

:deep(.el-select-dropdown .el-select-dropdown__item) {
    padding: 12px 16px;
    line-height: 1.4;
}

:deep(.el-select-dropdown .el-select-dropdown__item:hover) {
    background-color: #f0f9ff;
}

:deep(.el-select-dropdown .el-select-dropdown__item.is-selected) {
    background-color: #f0f9ff;
    color: #409eff;
}

/* 加载状态 */
:deep(.el-select .el-input.is-focus .el-input__inner) {
    border-color: #409eff;
}

/* 清空按钮 */
:deep(.el-select .el-input__suffix .el-input__suffix-inner .el-select__caret) {
    transition: all 0.3s;
}

:deep(.el-select .el-input__suffix .el-input__suffix-inner .el-select__caret.is-reverse) {
    transform: rotateZ(180deg);
}

/* 响应式设计 */
@media (max-width: 768px) {
    .season-option-title {
        font-size: 13px;
    }

    .season-option-id {
        font-size: 11px;
        padding: 1px 4px;
    }
}
</style>
