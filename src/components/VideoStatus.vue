<template>
    <el-dialog
        v-model="dialogVisible"
        title="视频转码状态"
        width="600px"
        :close-on-click-modal="false"
    >
        <div class="video-status-container">
            <div v-if="completedVideos.length === 0" class="empty-state">
                <el-empty description="暂无已完成上传的视频文件" />
            </div>
            <div v-else>
                <el-table
                    :data="videoStatusList"
                    stripe
                    style="width: 100%"
                    :row-key="(row: VideoStatusItem) => row.id"
                    :expand-row-keys="Array.from(expandedErrorRows)"
                    :row-class-name="getRowClassName"
                    @row-click="handleRowClick"
                >
                    <!-- 展开列 -->
                    <el-table-column type="expand" width="30">
                        <template #default="{ row }">
                            <div
                                v-if="isFailedStatus(row.statusCode)"
                                class="expanded-error-detail"
                            >
                                <div class="error-message-full">
                                    <strong>错误详情：</strong>{{ getErrorDetail(row.statusDesc) }}
                                </div>
                            </div>
                        </template>
                    </el-table-column>

                    <el-table-column prop="filename" label="文件名" min-width="250">
                        <template #default="{ row }">
                            <div class="filename-cell">
                                <el-icon class="file-icon"><video-play /></el-icon>
                                <span class="filename" :title="row.filename">{{
                                    row.filename
                                }}</span>
                            </div>
                        </template>
                    </el-table-column>
                    <el-table-column prop="status" label="转码状态" width="180" align="center">
                        <template #default="{ row }">
                            <div class="status-content">
                                <el-tag
                                    :type="getStatusType(row.statusCode)"
                                    effect="light"
                                    size="small"
                                >
                                    <el-icon class="status-icon">
                                        <check v-if="row.statusCode === ENCODING_STATUS.SUCCESS" />
                                        <loading
                                            v-else-if="
                                                row.statusCode === ENCODING_STATUS.PROCESSING
                                            "
                                        />
                                        <warning v-else />
                                    </el-icon>
                                    {{ row.statusText }}
                                </el-tag>

                                <!-- 点击提示图标 -->
                                <el-icon
                                    v-if="isFailedStatus(row.statusCode)"
                                    class="click-hint-icon"
                                >
                                    <arrow-down v-if="!expandedErrorRows.has(row.id)" />
                                    <arrow-up v-else />
                                </el-icon>
                            </div>
                        </template>
                    </el-table-column>
                </el-table>
            </div>
        </div>

        <template #footer>
            <div class="dialog-footer">
                <el-button @click="dialogVisible = false">关闭</el-button>
                <el-button type="primary" @click="refreshAllStatus" :loading="refreshingAll">
                    刷新全部
                </el-button>
            </div>
        </template>
    </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Check, Loading, Warning, VideoPlay, ArrowDown, ArrowUp } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'

// 转码状态常量
const ENCODING_STATUS = {
    SUCCESS: 0,
    PROCESSING: -30
} as const

interface VideoFile {
    id: string
    path: string
    title?: string
    filename?: string
    complete?: boolean
    status?: number
    status_desc?: string
    [key: string]: any
}

interface VideoStatusItem {
    id: string
    filename: string
    statusCode: number
    statusText: string
    statusDesc?: string
}

const props = defineProps<{
    modelValue: boolean
    videos: VideoFile[]
    user?: any
    templateAid?: number
}>()

const emit = defineEmits<{
    'update:modelValue': [value: boolean]
    'reload-template': []
}>()

const dialogVisible = computed({
    get: () => props.modelValue,
    set: value => emit('update:modelValue', value)
})

const refreshingAll = ref(false)
const expandedErrorRows = ref<Set<string>>(new Set())

// 获取已完成上传的视频
const completedVideos = computed(() => {
    return props.videos.filter(video => video.complete)
})

// 获取文件名
const getFilename = (video: VideoFile): string => {
    if (video.title) return video.title
    if (video.filename) return video.filename
    if (video.path) {
        return video.path.split(/[/\\]/).pop() || video.path
    }
    return `视频文件 ${video.id}`
}

// 获取状态文本
const getStatusText = (statusCode: number): string => {
    switch (statusCode) {
        case ENCODING_STATUS.SUCCESS:
            return '转码成功'
        case ENCODING_STATUS.PROCESSING:
            return '转码中'
        default:
            return '转码失败'
    }
}

// 计算视频状态列表
const videoStatusList = computed<VideoStatusItem[]>(() => {
    return completedVideos.value.map(video => ({
        id: video.id,
        filename: getFilename(video),
        statusCode: video.encoding_status ?? ENCODING_STATUS.SUCCESS,
        statusText: getStatusText(video.encoding_status ?? ENCODING_STATUS.SUCCESS),
        statusDesc: video.status_desc
    }))
})

// 获取状态标签类型
const getStatusType = (statusCode: number): string => {
    switch (statusCode) {
        case ENCODING_STATUS.SUCCESS:
            return 'success'
        case ENCODING_STATUS.PROCESSING:
            return 'warning'
        default:
            return 'danger'
    }
}

// 切换错误详情展开状态
const toggleErrorDetail = (videoId: string) => {
    if (expandedErrorRows.value.has(videoId)) {
        expandedErrorRows.value.delete(videoId)
    } else {
        expandedErrorRows.value.add(videoId)
    }
}

// 处理行点击事件
const handleRowClick = (row: VideoStatusItem) => {
    if (isFailedStatus(row.statusCode)) {
        toggleErrorDetail(row.id)
    }
}

// 获取行类名
const getRowClassName = ({ row }: { row: VideoStatusItem }): string => {
    return isFailedStatus(row.statusCode) ? 'clickable-row' : ''
}

// 获取错误详情
const getErrorDetail = (statusDesc?: string): string => {
    return statusDesc || '未知错误'
}

// 检查是否为失败状态
const isFailedStatus = (statusCode: number): boolean => {
    return statusCode !== ENCODING_STATUS.SUCCESS && statusCode !== ENCODING_STATUS.PROCESSING
}

// 刷新全部状态
const refreshAllStatus = async () => {
    refreshingAll.value = true
    try {
        // 触发模板重新加载
        emit('reload-template')

        // 模拟刷新延迟
        await new Promise(resolve => setTimeout(resolve, 1500))

        ElMessage.success('视频状态已刷新')
    } catch (error) {
        ElMessage.error('刷新状态失败')
    } finally {
        refreshingAll.value = false
    }
}
</script>

<style scoped>
.video-status-container {
    min-height: 200px;
}

.empty-state {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 200px;
}

.filename-cell {
    display: flex;
    align-items: center;
    gap: 8px;
}

.file-icon {
    color: #409eff;
    font-size: 16px;
    flex-shrink: 0;
}

.filename {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.status-icon {
    margin-right: 4px;
}

.status-content {
    display: flex;
    align-items: center;
    gap: 8px;
}

.click-hint-icon {
    color: #909399;
    font-size: 14px;
    transition: transform 0.2s ease;
}

.expanded-error-detail {
    padding: 12px 20px;
    background-color: #fafafa;
    border-radius: 4px;
    margin: 0 -20px;
}

.error-message-full {
    color: #909399;
    font-size: 12px;
    line-height: 1.4;
}

.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
}

/* 表格样式优化 */
:deep(.el-table) {
    border-radius: 6px;
    overflow: hidden;
}

:deep(.el-table th) {
    background-color: #f5f7fa;
    color: #303133;
    font-weight: 600;
}

:deep(.el-table td) {
    padding: 12px 0;
}

:deep(.el-table .el-table__row:hover > td) {
    background-color: #f0f9ff;
}

/* 失败状态行可点击样式 */
:deep(.el-table .el-table__row) {
    transition: background-color 0.2s ease;
}

:deep(.el-table .el-table__row.clickable-row) {
    cursor: pointer;
}

:deep(.el-table .el-table__row.clickable-row:hover > td) {
    background-color: #f0f9ff;
}

/* 状态标签样式 */
:deep(.el-tag) {
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 80px;
}
</style>
