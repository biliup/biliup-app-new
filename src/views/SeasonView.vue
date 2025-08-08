<template>
    <!-- 合集选择器模式：仅显示下拉选择框 -->
    <div class="season-selector">
        <div class="season-selector-wrapper">
            <el-button
                @click="showSeasonDialog = true"
                class="season-select-button"
                :type="selectedSeason ? 'primary' : 'default'"
            >
                <span v-if="selectedSeason" class="selected-season-text">
                    {{ selectedSeason.title }}
                </span>
                <span v-else class="placeholder-text">请选择</span>
            </el-button>
            <div class="action-buttons">
                <el-button
                    v-if="selectedSeason"
                    @click="clearSelection"
                    class="clear-button"
                    size="small"
                    text
                    :icon="Close"
                />
                <el-icon class="arrow-icon" @click="showSeasonDialog = true">
                    <arrow-down />
                </el-icon>
            </div>
        </div>

        <!-- 合集选择对话框 -->
        <el-dialog
            v-model="showSeasonDialog"
            title="加入合集"
            width="500px"
            :before-close="handleDialogClose"
            class="season-dialog"
        >
            <div class="season-dialog-content">
                <!-- 搜索框 -->
                <el-input
                    v-model="searchKeyword"
                    placeholder="搜索合集"
                    clearable
                    class="search-input"
                    @input="handleSearch"
                    :loading="isSearching"
                >
                    <template #suffix>
                        <el-icon v-if="!isSearching" class="search-icon"><search /></el-icon>
                        <el-icon v-else class="search-icon loading"><Loading /></el-icon>
                    </template>
                </el-input>

                <!-- 合集列表 -->
                <div class="season-list">
                    <el-radio-group
                        v-model="selectedIndex"
                        class="season-radio-group"
                        name="seasonSelection"
                    >
                        <div class="season-grid-container">
                            <div
                                v-for="(season, index) in filteredSeasons"
                                :key="`season-${season.season_id}-${season.section_id}`"
                                class="season-item-compact"
                            >
                                <el-radio
                                    :value="index"
                                    class="season-radio-compact"
                                    @change="selectSeason(season.season_id, season.section_id)"
                                >
                                    <div class="season-content">
                                        <span class="season-title-compact">{{ season.title }}</span>
                                        <div class="season-meta">
                                            <span class="season-id-compact"
                                                >ID: {{ season.season_id }}</span
                                            >
                                        </div>
                                    </div>
                                </el-radio>
                            </div>
                        </div>
                    </el-radio-group>
                </div>

                <!-- 如果没有搜索结果 -->
                <div v-if="filteredSeasons.length === 0" class="no-results">
                    <el-empty description="暂无相关合集" :image-size="80" />
                </div>
            </div>

            <template #footer>
                <div class="dialog-footer">
                    <el-button @click="showSeasonDialog = false">取消</el-button>
                    <el-button type="primary" @click="confirmSelection">确定</el-button>
                </div>
            </template>
        </el-dialog>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useUtilsStore } from '../stores/utils'
import { ArrowDown, Search, Loading, Close } from '@element-plus/icons-vue'

// 定义组件props
interface Props {
    modelValue?: number // season_id
    sectionId?: number // section_id
    userUid?: number
    aid?: number // 当aid为null或0时隐藏整个组件
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
const filteredSeasons = ref<Season[]>([])
const selectedSeasonId = ref<number | undefined>(props.modelValue)
const selectedSectionId = ref<number | undefined>(props.sectionId)
const selectedIndex = ref<number | undefined>()
const searchKeyword = ref('')
const showSeasonDialog = ref(false)
const isSearching = ref(false)

// 计算属性
const selectedSeason = computed(() => {
    return filteredSeasons.value.find(
        season =>
            season.season_id === selectedSeasonId.value &&
            season.section_id === selectedSectionId.value
    )
})

// 是否显示组件
const shouldShowComponent = computed(() => {
    return props.aid && props.aid > 0
})

// 搜索处理函数
const handleSearch = (query?: string) => {
    const searchQuery = query !== undefined ? query : searchKeyword.value
    searchKeyword.value = searchQuery

    performLocalSearch(searchQuery)
}

// 执行本地搜索
const performLocalSearch = (searchQuery: string) => {
    if (!searchQuery) {
        filteredSeasons.value = allSeasons.value
        return
    }

    const lowerQuery = searchQuery.toLowerCase()
    const filtered = allSeasons.value.filter(
        season =>
            season.title.toLowerCase().includes(lowerQuery) ||
            season.season_id.toString().includes(searchQuery)
    )

    // 使用联合主键去重
    filteredSeasons.value = filtered.filter(
        (season, index, self) =>
            index ===
            self.findIndex(
                s => s.season_id === season.season_id && s.section_id === season.section_id
            )
    )
}

// 选择合集
const selectSeason = (seasonId: number, sectionId: number) => {
    selectedSeasonId.value = seasonId
    selectedSectionId.value = sectionId

    // 更新selectedIndex以保持radio的选中状态
    const index = filteredSeasons.value.findIndex(
        s => s.season_id === seasonId && s.section_id === sectionId
    )
    if (index !== -1) {
        selectedIndex.value = index
    }
}

// 清空选择
const clearSelection = () => {
    selectedSeasonId.value = undefined
    selectedSectionId.value = undefined
    selectedIndex.value = undefined
}

// 确认选择
const confirmSelection = () => {
    showSeasonDialog.value = false
}

// 处理对话框关闭
const handleDialogClose = (done: () => void) => {
    done()
}

// 更新selectedIndex的辅助函数
const updateSelectedIndex = () => {
    if (selectedSeasonId.value !== undefined && selectedSectionId.value !== undefined) {
        const index = filteredSeasons.value.findIndex(
            s => s.season_id === selectedSeasonId.value && s.section_id === selectedSectionId.value
        )
        selectedIndex.value = index !== -1 ? index : undefined
    } else {
        selectedIndex.value = undefined
    }
}

// 加载合集列表
const loadSeasons = async () => {
    if (!props.userUid || !shouldShowComponent.value) {
        return
    }

    try {
        isSearching.value = true
        const result = (await utilsStore.getSeasonList(props.userUid)) as any

        if (result && result.seasions) {
            allSeasons.value = result.seasions.map((season: any) => ({
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

            handleSearch('')
            updateSelectedIndex()
        }
    } catch (error) {
        console.error('加载合集列表失败:', error)
    } finally {
        isSearching.value = false
    }
}

// 监听props变化
watch(
    () => props.modelValue,
    newValue => {
        selectedSeasonId.value = newValue
    }
)

watch(
    () => props.sectionId,
    newValue => {
        selectedSectionId.value = newValue
    }
)

watch(
    () => props.aid,
    () => {
        if (shouldShowComponent.value) {
            loadSeasons()
        }
    }
)

watch(
    () => props.userUid,
    () => {
        if (shouldShowComponent.value) {
            loadSeasons()
        }
    }
)

// 监听selectedIndex变化，更新实际的选中值
watch(selectedIndex, newIndex => {
    if (newIndex !== undefined && filteredSeasons.value[newIndex]) {
        const season = filteredSeasons.value[newIndex]
        selectedSeasonId.value = season.season_id
        selectedSectionId.value = season.section_id
    }
})

// 监听选中值变化，向父组件发送更新
watch(selectedSeasonId, newValue => {
    emit('update:modelValue', newValue)
})

watch(selectedSectionId, newValue => {
    emit('update:sectionId', newValue)
})

// 监听props变化，更新selectedIndex
watch([() => props.modelValue, () => props.sectionId], () => {
    updateSelectedIndex()
})

// 组件挂载时加载数据
onMounted(() => {
    if (shouldShowComponent.value) {
        loadSeasons()
    }
})
</script>

<style scoped>
/* 选择器模式样式 */
.season-selector {
    width: 100%;
}

.season-selector-wrapper {
    display: flex;
    align-items: center;
    border: 1px solid #dcdfe6;
    border-radius: 4px;
    background: #fff;
    transition: border-color 0.3s;
}

.season-selector-wrapper:hover {
    border-color: #c0c4cc;
}

.season-selector-wrapper:has(.season-select-button.el-button--primary) {
    border-color: #409eff;
}

.season-select-button {
    flex: 1;
    display: flex;
    justify-content: flex-start;
    align-items: center;
    padding: 8px 12px;
    text-align: left;
    border: none;
    background: transparent;
    color: #606266;
    border-radius: 0;
    transition: none;
}

.season-select-button:hover {
    border: none;
    background: transparent;
}

.season-select-button.el-button--primary {
    border: none;
    background: transparent;
    color: #409eff;
}

.action-buttons {
    display: flex;
    align-items: center;
    padding: 0 8px;
    gap: 4px;
}

.clear-button {
    padding: 4px;
    min-height: auto;
    color: #909399;
    transition: color 0.3s;
}

.clear-button:hover {
    color: #f56c6c;
    background: transparent;
}

.arrow-icon {
    color: #c0c4cc;
    cursor: pointer;
    transition: color 0.3s;
    padding: 4px;
}

.arrow-icon:hover {
    color: #409eff;
}

.selected-season-text {
    flex: 1;
    text-align: left;
    font-weight: normal;
}

.placeholder-text {
    flex: 1;
    text-align: left;
    color: #c0c4cc;
    font-weight: normal;
}

.season-dialog-content {
    display: flex;
    flex-direction: column;
    padding: 0 4px;
}

.search-input {
    margin-bottom: 16px;
}

.search-input :deep(.el-input__inner) {
    background-color: #f4f5f7;
    border: 1px solid #e3e5e7;
    border-radius: 6px;
    padding: 8px 12px;
}

.search-input :deep(.el-input__inner:focus) {
    border-color: #00aeec;
    background-color: #fff;
}

.search-icon {
    color: #c9ccd0;
}

.search-icon.loading {
    animation: rotate 1s linear infinite;
}

@keyframes rotate {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

.season-list {
    max-height: 300px;
    overflow-y: auto;
    border: none;
    border-radius: 0;
}

.season-radio-group {
    width: 100%;
    display: flex;
    flex-direction: column;
}

.season-grid-container {
    display: grid;
    grid-template-columns: 1fr;
    gap: 8px;
    padding: 8px 0;
}

.season-item-compact {
    display: flex;
    align-items: center;
    border: 1px solid #f1f2f3;
    border-radius: 6px;
    transition: all 0.2s;
    background: #fff;
    overflow: hidden;
}

.season-item-compact:hover {
    border-color: #00aeec;
    background-color: #f7f9fa;
}

.season-radio-compact {
    width: 100%;
    margin: 0;
    display: flex;
    align-items: center;
    padding: 12px;
    cursor: pointer;
}

:deep(.season-radio-compact .el-radio__input) {
    margin-right: 8px;
    margin-top: 0;
    flex-shrink: 0;
}

:deep(.season-radio-compact .el-radio__inner) {
    border-color: #dcdfe6;
}

:deep(.season-radio-compact.is-checked .el-radio__inner) {
    background-color: #00aeec;
    border-color: #00aeec;
}

:deep(.season-radio-compact:hover .el-radio__inner) {
    border-color: #00aeec;
}

:deep(.season-radio-compact .el-radio__label) {
    padding-left: 0;
    font-weight: normal;
    color: #18191c;
    font-size: 14px;
    line-height: 1.4;
    width: 100%;
    overflow: hidden;
}

.season-content {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    min-width: 0;
}

.season-title-compact {
    font-weight: normal;
    font-size: 14px;
    color: #18191c;
    line-height: 1.4;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.season-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
}

.season-id-compact {
    color: #909399;
    font-size: 12px;
    white-space: nowrap;
    flex-shrink: 0;
}

.no-results {
    padding: 40px 0;
    text-align: center;
    color: #99a2aa;
}

.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 16px;
}

.dialog-footer .el-button {
    min-width: 64px;
    padding: 8px 16px;
}

.dialog-footer .el-button--primary {
    background-color: #00aeec;
    border-color: #00aeec;
}

.dialog-footer .el-button--primary:hover {
    background-color: #40c9ff;
    border-color: #40c9ff;
}

/* 对话框全局样式 */
:deep(.season-dialog .el-dialog__header) {
    padding: 16px 20px;
    border-bottom: 1px solid #e3e5e7;
}

:deep(.season-dialog .el-dialog__title) {
    font-size: 16px;
    font-weight: 500;
    color: #18191c;
}

:deep(.season-dialog .el-dialog__body) {
    padding: 20px;
}

:deep(.season-dialog .el-dialog__footer) {
    padding: 16px 20px;
    border-top: 1px solid #e3e5e7;
}

/* 移动端适配 */
@media (max-width: 768px) {
    .season-grid-container {
        grid-template-columns: 1fr;
    }
}
</style>
