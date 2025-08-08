<template>
    <!-- 选择器模式：仅显示下拉选择框 -->
    <div class="topic-selector">
        <div class="topic-selector-wrapper">
            <el-button
                @click="showTopicDialog = true"
                class="topic-select-button"
                :type="selectedTopic ? 'primary' : 'default'"
            >
                <span v-if="selectedTopic" class="selected-topic-text">
                    {{ selectedTopic.topic_name }}
                </span>
                <span v-else class="placeholder-text">请输入</span>
            </el-button>
            <div class="action-buttons">
                <el-button
                    v-if="selectedTopic"
                    @click="clearSelection"
                    class="clear-button"
                    size="small"
                    text
                    :icon="Close"
                />
                <el-icon class="arrow-icon" @click="showTopicDialog = true">
                    <arrow-down />
                </el-icon>
            </div>
        </div>

        <!-- 话题选择对话框 -->
        <el-dialog
            v-model="showTopicDialog"
            title="参与活动"
            width="500px"
            :before-close="handleDialogClose"
            class="topic-dialog"
        >
            <div class="topic-dialog-content">
                <!-- 搜索框 -->
                <el-input
                    v-model="searchKeyword"
                    placeholder="请输入"
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

                <!-- 话题列表 -->
                <div class="topic-list">
                    <el-radio-group
                        v-model="selectedIndex"
                        class="topic-radio-group"
                        name="topicSelection"
                    >
                        <div class="topic-grid-container">
                            <div
                                v-for="(topic, index) in filteredTopics"
                                :key="`topic-${topic.topic_id}-${topic.mission_id}`"
                                class="topic-item-compact"
                            >
                                <el-radio
                                    :value="index"
                                    class="topic-radio-compact"
                                    @change="selectTopic(topic.mission_id, topic.topic_id)"
                                >
                                    <div class="topic-content">
                                        <span class="topic-title-compact">{{
                                            topic.topic_name
                                        }}</span>
                                        <div class="topic-meta">
                                            <span class="activity-badge-compact">{{
                                                topic.activity_text
                                            }}</span>
                                            <span class="play-count-compact">
                                                {{ formatPlayCount(topic.arc_play_vv) }}
                                            </span>
                                        </div>
                                    </div>
                                </el-radio>
                            </div>
                        </div>
                    </el-radio-group>
                </div>

                <!-- 如果没有搜索结果 -->
                <div v-if="filteredTopics.length === 0" class="no-results">
                    <el-empty description="暂无相关活动" :image-size="80" />
                </div>
            </div>

            <template #footer>
                <div class="dialog-footer">
                    <el-button @click="showTopicDialog = false">取消</el-button>
                    <el-button type="primary" @click="confirmSelection">确定</el-button>
                </div>
            </template>
        </el-dialog>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import { useUtilsStore } from '../stores/utils'
import { ArrowDown, Search, Loading, Close } from '@element-plus/icons-vue'

// 定义组件props
interface Props {
    modelValue?: number // mission_id
    topicId?: number // topic_id
    userUid?: number
    mode?: 'selector' | 'full' // selector: 仅显示选择器, full: 显示完整界面
}

const props = withDefaults(defineProps<Props>(), {
    mode: 'full'
})

// 定义emits
const emit = defineEmits<{
    'update:modelValue': [value: number | undefined]
    'update:topicId': [value: number | undefined]
}>()

const utilsStore = useUtilsStore()

// 话题数据接口
interface Topic {
    topic_id: number
    topic_name: string
    description: string
    mission_id: number
    activity_text: string
    activity_description: string
    arc_play_vv: number
}

// 响应式数据
const filteredTopics = ref<Topic[]>([])
const selectedMissionId = ref<number | undefined>(props.modelValue)
const selectedTopicId = ref<number | undefined>(props.topicId)
const selectedIndex = ref<number | undefined>() // 新增：用于radio group的选中索引
const searchKeyword = ref('')
const showTopicDialog = ref(false)
const searchTimer = ref<number | null>(null) // 防抖定时器
const isSearching = ref(false) // 搜索状态

// 计算属性 - 直接使用utilsStore中的topiclist，并使用联合主键去重
const allTopics = computed(() => {
    const topics = utilsStore.topiclist as Topic[]
    const uniqueTopics = topics.filter(
        (topic, index, self) =>
            index ===
            self.findIndex(t => t.topic_id === topic.topic_id && t.mission_id === topic.mission_id)
    )

    return uniqueTopics
})

const selectedTopic = computed(() => {
    return filteredTopics.value.find(
        topic =>
            topic.mission_id === selectedMissionId.value && topic.topic_id === selectedTopicId.value
    )
})

// 格式化播放量
const formatPlayCount = (count: number): string => {
    if (count === 0) return '0'
    if (count < 1000) return count.toString()
    if (count < 10000) return (count / 1000).toFixed(1) + 'K'
    if (count < 100000000) return (count / 10000).toFixed(1) + '万'
    return (count / 100000000).toFixed(1) + '亿'
}

// 搜索处理函数
const handleSearch = (query?: string) => {
    const searchQuery = query !== undefined ? query : searchKeyword.value
    searchKeyword.value = searchQuery

    // 清除之前的定时器
    if (searchTimer.value) {
        clearTimeout(searchTimer.value)
        searchTimer.value = null
    }

    // 立即进行本地搜索
    performLocalSearch(searchQuery)

    // 如果有搜索内容，设置防抖定时器进行远程搜索
    if (searchQuery && searchQuery.trim()) {
        searchTimer.value = setTimeout(async () => {
            await performRemoteSearch(searchQuery.trim())
        }, 1000)
    }
}

// 执行本地搜索
const performLocalSearch = (searchQuery: string) => {
    if (!searchQuery) {
        filteredTopics.value = allTopics.value
        return
    }

    const lowerQuery = searchQuery.toLowerCase()
    const filtered = allTopics.value.filter(
        topic =>
            topic.topic_name.toLowerCase().includes(lowerQuery) ||
            topic.description.toLowerCase().includes(lowerQuery) ||
            topic.activity_description.toLowerCase().includes(lowerQuery) ||
            topic.mission_id.toString().includes(searchQuery)
    )

    // 使用联合主键去重
    filteredTopics.value = filtered.filter(
        (topic, index, self) =>
            index ===
            self.findIndex(t => t.topic_id === topic.topic_id && t.mission_id === topic.mission_id)
    )
}

// 执行远程搜索
const performRemoteSearch = async (query: string) => {
    if (!props.userUid) {
        console.warn('用户UID未提供，跳过远程搜索')
        return
    }

    try {
        isSearching.value = true
        console.log('开始远程搜索:', query)

        // 调用utils store的搜索方法
        const searchResults = (await utilsStore.searchTopics(props.userUid, query)) as any[]

        if (searchResults && searchResults.length > 0) {
            console.log('远程搜索返回结果:', searchResults.length)

            // 将搜索结果的id字段映射为topic_id，转换为Topic格式
            const mappedResults: Topic[] = searchResults.map(result => ({
                topic_id: result.id || result.topic_id, // 将id映射为topic_id
                topic_name: result.topic_name || result.name || '',
                description: result.description || '',
                mission_id: result.mission_id || 0,
                activity_text: result.activity_text || '',
                activity_description: result.activity_description || '',
                arc_play_vv: result.arc_play_vv || 0
            }))

            // 将搜索结果添加到当前过滤结果的后面
            const existingIds = new Set(
                filteredTopics.value.map(t => `${t.topic_id}-${t.mission_id}`)
            )

            // 过滤掉重复的结果
            const newResults = mappedResults.filter(
                topic => !existingIds.has(`${topic.topic_id}-${topic.mission_id}`)
            )

            if (newResults.length > 0) {
                filteredTopics.value = [...filteredTopics.value, ...newResults]
                console.log('添加了新的搜索结果:', newResults.length)
            }
        }
    } catch (error) {
        console.error('远程搜索失败:', error)
    } finally {
        isSearching.value = false
    }
}

// 选择话题
const selectTopic = (missionId: number, topicId: number) => {
    selectedMissionId.value = missionId
    selectedTopicId.value = topicId

    // 更新selectedIndex以保持radio的选中状态
    const index = filteredTopics.value.findIndex(
        t => t.mission_id === missionId && t.topic_id === topicId
    )
    if (index !== -1) {
        selectedIndex.value = index
    }
}

// 确认选择
const confirmSelection = () => {
    showTopicDialog.value = false
}

// 清空选择
const clearSelection = () => {
    selectedMissionId.value = undefined
    selectedTopicId.value = undefined
    selectedIndex.value = undefined
}

// 处理对话框关闭
const handleDialogClose = (done: () => void) => {
    done()
}

// 更新selectedIndex的辅助函数
const updateSelectedIndex = () => {
    if (selectedMissionId.value !== undefined && selectedTopicId.value !== undefined) {
        const index = filteredTopics.value.findIndex(
            t => t.mission_id === selectedMissionId.value && t.topic_id === selectedTopicId.value
        )
        selectedIndex.value = index !== -1 ? index : undefined
    } else {
        selectedIndex.value = undefined
    }
}

// 监听modelValue变化
watch(
    () => props.modelValue,
    newValue => {
        selectedMissionId.value = newValue
    }
)

// 监听topicId变化
watch(
    () => props.topicId,
    newValue => {
        selectedTopicId.value = newValue
    }
)

// 监听selectedIndex变化，更新实际的选中值
watch(selectedIndex, newIndex => {
    if (newIndex !== undefined && filteredTopics.value[newIndex]) {
        const topic = filteredTopics.value[newIndex]
        selectedMissionId.value = topic.mission_id
        selectedTopicId.value = topic.topic_id
    }
})

// 监听selectedMissionId变化，向父组件发送更新
watch(selectedMissionId, newValue => {
    emit('update:modelValue', newValue)
})

// 监听selectedTopicId变化，向父组件发送更新
watch(selectedTopicId, newValue => {
    emit('update:topicId', newValue)
})

// 监听allTopics变化，更新过滤列表
watch(
    allTopics,
    newTopics => {
        if (newTopics.length > 0) {
            handleSearch('')
            // 更新selectedIndex
            updateSelectedIndex()
        }
    },
    { immediate: true }
)

// 监听props变化，更新selectedIndex
watch([() => props.modelValue, () => props.topicId], () => {
    updateSelectedIndex()
})

// 组件卸载时清理定时器
onUnmounted(() => {
    if (searchTimer.value) {
        clearTimeout(searchTimer.value)
        searchTimer.value = null
    }
})
</script>

<style scoped>
/* 选择器模式样式 */
.topic-selector {
    width: 100%;
}

.topic-selector-wrapper {
    display: flex;
    align-items: center;
    border: 1px solid #dcdfe6;
    border-radius: 4px;
    background: #fff;
    transition: border-color 0.3s;
}

.topic-selector-wrapper:hover {
    border-color: #c0c4cc;
}

.topic-selector-wrapper:has(.topic-select-button.el-button--primary) {
    border-color: #409eff;
}

.topic-select-button {
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

.topic-select-button:hover {
    border: none;
    background: transparent;
}

.topic-select-button.el-button--primary {
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

.selected-topic-text {
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

.topic-dialog-content {
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

.topic-list {
    max-height: 300px;
    overflow-y: auto;
    border: none;
    border-radius: 0;
}

.topic-radio-group {
    width: 100%;
    display: flex;
    flex-direction: column;
}

.topic-grid-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    padding: 8px 0;
}

.topic-item-compact {
    display: flex;
    align-items: center;
    border: 1px solid #f1f2f3;
    border-radius: 6px;
    transition: all 0.2s;
    background: #fff;
    overflow: hidden;
}

.topic-item-compact:hover {
    border-color: #00aeec;
    background-color: #f7f9fa;
}

.topic-radio-compact {
    width: 100%;
    margin: 0;
    display: flex;
    align-items: center;
    padding: 8px;
    cursor: pointer;
}

:deep(.topic-radio-compact .el-radio__input) {
    margin-right: 6px;
    margin-top: 0;
    flex-shrink: 0;
}

:deep(.topic-radio-compact .el-radio__inner) {
    border-color: #dcdfe6;
}

:deep(.topic-radio-compact.is-checked .el-radio__inner) {
    background-color: #00aeec;
    border-color: #00aeec;
}

:deep(.topic-radio-compact:hover .el-radio__inner) {
    border-color: #00aeec;
}

:deep(.topic-radio-compact .el-radio__label) {
    padding-left: 0;
    font-weight: normal;
    color: #18191c;
    font-size: 13px;
    line-height: 1.3;
    width: 100%;
    overflow: hidden;
}

.topic-content {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    min-width: 0;
}

.topic-title-compact {
    font-weight: normal;
    font-size: 13px;
    color: #18191c;
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.topic-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
}

.activity-badge-compact {
    background: #e7f3ff;
    color: #00aeec;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 11px;
    white-space: nowrap;
    flex-shrink: 0;
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
}

.play-count-compact {
    color: #909399;
    font-size: 10px;
    white-space: nowrap;
    flex-shrink: 0;
}

.topic-title {
    font-weight: normal;
    font-size: 14px;
    color: #18191c;
    line-height: 1.4;
}

.activity-badge {
    background: #e7f3ff;
    color: #00aeec;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 12px;
    white-space: nowrap;
    margin-left: 8px;
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
:deep(.topic-dialog .el-dialog__header) {
    padding: 16px 20px;
    border-bottom: 1px solid #e3e5e7;
}

:deep(.topic-dialog .el-dialog__title) {
    font-size: 16px;
    font-weight: 500;
    color: #18191c;
}

:deep(.topic-dialog .el-dialog__body) {
    padding: 20px;
}

:deep(.topic-dialog .el-dialog__footer) {
    padding: 16px 20px;
    border-top: 1px solid #e3e5e7;
}

/* 完整模式样式 */
.topic-view {
    height: 100vh;
    background: #f5f7fa;
}

.topic-container {
    height: 100%;
}

.topic-content {
    padding: 20px;
    overflow-y: auto;
}

.content-wrapper {
    max-width: 1200px;
    margin: 0 auto;
}

.topic-card {
    border-radius: 8px;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}

.topic-search-section {
    margin-bottom: 30px;
}

.topic-select {
    width: 100%;
    max-width: 600px;
}

:deep(.topic-select-dropdown) {
    max-height: 250px;
    overflow-y: auto;
}

:deep(.topic-option) {
    height: auto;
    min-height: 60px;
    padding: 8px 12px;
    line-height: 1.4;
}

.topic-option-content {
    width: 100%;
}

.topic-name {
    font-weight: 600;
    font-size: 14px;
    color: #303133;
    margin-bottom: 4px;
}

.topic-info {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 4px;
}

.activity-tag {
    background: #e6f7ff;
    color: #1890ff;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 12px;
}

.play-count {
    color: #909399;
    font-size: 12px;
}

.topic-desc {
    color: #606266;
    font-size: 12px;
    line-height: 1.3;
    max-height: 32px;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
}

.selected-topic-info {
    margin-top: 20px;
    padding: 20px;
    background: #f8f9fa;
    border-radius: 6px;
    border: 1px solid #e9ecef;
}

.selected-topic-info h3 {
    margin: 0 0 16px 0;
    color: #409eff;
    font-size: 16px;
}

.topic-details {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.detail-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
}

.detail-row.description {
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
}

.detail-row .label {
    font-weight: 600;
    color: #606266;
    min-width: 80px;
    flex-shrink: 0;
}

.detail-row .value {
    color: #303133;
    flex: 1;
}

.detail-row .value.activity-text {
    background: #e6f7ff;
    color: #1890ff;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 13px;
}

.topic-list-section {
    margin-top: 30px;
}

.topic-list-section h3 {
    margin: 0 0 20px 0;
    color: #303133;
    font-size: 18px;
}

.topic-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 16px;
}

.topic-card-item {
    background: white;
    border: 1px solid #e4e7ed;
    border-radius: 6px;
    padding: 16px;
    cursor: pointer;
    transition: all 0.3s ease;
}

.topic-card-item:hover {
    border-color: #409eff;
    box-shadow: 0 2px 8px rgba(64, 158, 255, 0.2);
    transform: translateY(-2px);
}

.topic-card-item.active {
    border-color: #409eff;
    background: #ecf5ff;
}

.topic-card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 12px;
    gap: 8px;
}

.topic-title {
    font-weight: 600;
    color: #303133;
    font-size: 14px;
    line-height: 1.4;
    flex: 1;
}

.activity-badge {
    background: #e6f7ff;
    color: #1890ff;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 12px;
    white-space: nowrap;
    flex-shrink: 0;
}

.topic-card-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.topic-stats {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: #909399;
}

.play-stat {
    display: flex;
    align-items: center;
    gap: 4px;
}

.mission-id {
    font-family: monospace;
}

.topic-description {
    color: #606266;
    font-size: 13px;
    line-height: 1.4;
    max-height: 40px;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
}

/* 移动端适配 */
@media (max-width: 768px) {
    .topic-content {
        padding: 10px;
    }

    .topic-grid {
        grid-template-columns: 1fr;
    }

    .topic-select {
        max-width: 100%;
    }

    .topic-grid-container {
        grid-template-columns: 1fr;
    }
}

@media (max-width: 480px) {
    .topic-grid-container {
        grid-template-columns: 1fr;
        gap: 6px;
    }

    .topic-item-compact {
        padding: 6px;
    }

    .topic-title-compact {
        font-size: 12px;
    }

    .activity-badge-compact {
        font-size: 10px;
        padding: 1px 4px;
    }
}
</style>
