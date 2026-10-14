<template>
    <el-dialog
        :model-value="modelValue"
        :title="t('提交统计')"
        width="80%"
        top="6vh"
        @close="handleClose"
    >
        <div class="submit-stats-overview">
            <el-tag type="info">{{ t('启动时间：') }}{{ stats.startedAt }}</el-tag>
            <el-tag type="success">{{ t('成功：') }}{{ stats.successCount }}</el-tag>
            <el-tag type="danger">{{ t('失败：') }}{{ stats.failCount }}</el-tag>
            <el-tag type="warning">{{ t('总计：') }}{{ stats.totalCount }}</el-tag>
        </div>

        <div class="submit-stats-actions">
            <el-button type="danger" plain @click="emit('clear')">{{ t('清空统计') }}</el-button>
        </div>

        <el-table :data="stats.records" height="420" stripe :empty-text="t('暂无提交统计')">
            <el-table-column prop="time" :label="t('时间')" width="180" />
            <el-table-column prop="user" :label="t('用户')" width="220" />
            <el-table-column prop="mode" :label="t('模式')" width="110">
                <template #default="{ row }">{{ t(row.mode) }}</template>
            </el-table-column>
            <el-table-column prop="templateName" :label="t('模板')" min-width="180" />
            <el-table-column prop="statusText" :label="t('结果')" width="90">
                <template #default="{ row }">{{ t(row.statusText) }}</template>
            </el-table-column>
            <el-table-column prop="bvid" :label="t('BV号')" width="170" />
            <el-table-column prop="videoName" :label="t('失败视频')" min-width="220" />
            <el-table-column
                prop="error"
                :label="t('错误信息')"
                min-width="260"
                show-overflow-tooltip
            />
        </el-table>
    </el-dialog>
</template>

<script setup lang="ts">
import { t } from '../i18n'
interface SubmitStatsRecord {
    time: string
    user: string
    mode: '单稿件' | '多稿件'
    templateName: string
    status: 'success' | 'failed'
    statusText: '成功' | '失败'
    bvid: string
    videoName: string
    error: string
}

defineProps<{
    modelValue: boolean
    stats: {
        startedAt: string
        totalCount: number
        successCount: number
        failCount: number
        records: SubmitStatsRecord[]
    }
}>()

const emit = defineEmits<{
    'update:modelValue': [value: boolean]
    clear: []
}>()

const handleClose = () => {
    emit('update:modelValue', false)
}
</script>

<style scoped>
.submit-stats-overview {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 12px;
}

.submit-stats-actions {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 12px;
}
</style>
