<template>
    <el-dialog
        :model-value="modelValue"
        :title="t('稿件处理进度')"
        width="900px"
        style="max-width: calc(100vw - 32px)"
        :close-on-click-modal="false"
        @update:model-value="emit('update:modelValue', $event)"
    >
        <template #header>
            <div class="status-dialog-title">
                <span>{{ t('稿件处理进度') }}</span>
                <el-button
                    link
                    :icon="Refresh"
                    :loading="loading || transcodeLoading"
                    :disabled="!uid || !aid || loading || transcodeLoading"
                    :title="t('刷新')"
                    :aria-label="t('刷新')"
                    @click="loadPage(page)"
                />
            </div>
        </template>
        <el-alert v-if="error" :title="error" type="error" :closable="false" show-icon />
        <div v-loading="loading" class="processing-content">
            <template v-if="processing">
                <div class="section-heading">
                    <strong>{{ t('分P稿件') }}（{{ processing.page.total }}）</strong>
                    <el-pagination
                        :current-page="page"
                        :page-size="20"
                        :total="processing.page.total"
                        :disabled="loading"
                        layout="prev, pager, next"
                        @current-change="loadPage"
                    />
                </div>
                <div class="part-list">
                    <el-button
                        v-for="video in processing.videos"
                        :key="video.cid"
                        :type="selectedCid === video.cid ? 'primary' : 'default'"
                        :plain="selectedCid !== video.cid"
                        :disabled="loading"
                        :title="xcodeText(video.xcode_state)"
                        @click="loadTranscode(video)"
                        >P{{ video.index }}</el-button
                    >
                </div>
                <el-empty v-if="!processing.videos.length" :description="t('暂无稿件')" />
                <template v-if="selectedCid">
                    <section>
                        <div class="section-heading">
                            <strong>{{ t('稿件上传') }}</strong>
                            <el-tag type="success">{{ t('上传完成') }}</el-tag>
                        </div>
                        <el-progress :percentage="100" :show-text="false" />
                    </section>
                    <section v-loading="transcodeLoading">
                        <div class="section-heading">
                            <strong>{{ t('稿件转码') }}</strong>
                            <el-tag v-if="transcode">{{ xcodeText(transcode.xcode_state) }}</el-tag>
                        </div>
                        <el-alert
                            v-if="transcodeError"
                            :title="transcodeError"
                            type="error"
                            :closable="false"
                        />
                        <template v-if="transcode">
                            <p v-if="transcode.xcode_tip">{{ transcode.xcode_tip }}</p>
                            <el-alert
                                v-if="transcode.fail_tip"
                                :title="transcode.fail_tip"
                                type="error"
                                :closable="false"
                            />
                            <div class="resolution-list">
                                <div
                                    v-for="resolution in transcode.transcode_list || []"
                                    :key="resolution.resolution"
                                >
                                    <div class="resolution-heading">
                                        <span>{{ t(resolution.resolution) }}</span>
                                        <span>{{ resolutionText(resolution) }}</span>
                                    </div>
                                    <el-progress
                                        :percentage="resolutionProgress(resolution)"
                                        :status="
                                            resolution.status === 'failed' ? 'exception' : undefined
                                        "
                                        :show-text="false"
                                    />
                                </div>
                            </div>
                        </template>
                    </section>
                </template>
                <section>
                    <div class="section-heading">
                        <strong>{{ t('稿件审核') }}</strong>
                        <el-tag>{{ auditText(processing.audit.aegis_state) }}</el-tag>
                    </div>
                    <div v-if="processing.audit.audit_process" class="audit-steps">
                        <span
                            >{{ t('内容审核') }}：{{
                                stepText(processing.audit.audit_process.archive_audit)
                            }}</span
                        >
                        <span
                            >{{ t('业务审核') }}：{{
                                stepText(processing.audit.audit_process.business_audit)
                            }}</span
                        >
                    </div>
                </section>
            </template>
        </div>
        <template #footer>
            <el-button @click="emit('update:modelValue', false)">{{ t('关闭') }}</el-button>
            <el-button
                type="primary"
                :loading="loading || transcodeLoading"
                @click="loadPage(page)"
                >{{ t('刷新') }}</el-button
            >
        </template>
    </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, onBeforeUnmount } from 'vue'
import { Refresh } from '@element-plus/icons-vue'
import { t } from '../i18n'
import { useUtilsStore } from '../stores/utils'
import type {
    ArchiveProcessing,
    ProcessingVideo,
    TranscodeDetail,
    TranscodeResolution
} from '../types/video_status'

const props = defineProps<{ modelValue: boolean; uid?: number; aid?: number }>()
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()
const utilsStore = useUtilsStore()
const processing = ref<ArchiveProcessing | null>(null)
const transcode = ref<TranscodeDetail | null>(null)
const loading = ref(false)
const transcodeLoading = ref(false)
const error = ref('')
const transcodeError = ref('')
const selectedCid = ref<number | null>(null)
const page = ref(1)
let listRequest = 0
let transcodeRequest = 0

const xcodeText = (state: number) =>
    t(
        ({ 0: '未开始', 1: '转码中', 2: '转码完成', 3: '转码失败' } as Record<number, string>)[
            state
        ] || '状态未知'
    )
const auditText = (state: number) =>
    t(
        (
            {
                0: '未开始',
                1: '审核中',
                2: '已完成',
                3: '已退回',
                4: '已锁定',
                5: '受限'
            } as Record<number, string>
        )[state] || '状态未知'
    )
const stepText = (state: number) =>
    t(({ 0: '未开始', 1: '进行中', 2: '已完成' } as Record<number, string>)[state] || '状态未知')

// 与官网一致，处理中按服务端时间估算，成功前最多显示 99%。
const resolutionProgress = (item: TranscodeResolution) => {
    if (item.status === 'success') return 100
    if (item.status !== 'processing') return 0
    const duration = item.estimated_time - item.start_time
    const estimate =
        item.estimated_time > 0 && duration > 0
            ? Math.round(((item.time_now - item.start_time) * 100) / duration)
            : 0
    return Math.min(99, Math.max(0, estimate, item.progress || 0))
}
const resolutionText = (item: TranscodeResolution) => {
    if (item.status === 'success') return t('已完成')
    if (item.status === 'failed') return item.failure_reason || t('转码失败')
    if (item.status === 'processing')
        return item.estimated_time > 0
            ? t('预计 {0}%', { 0: resolutionProgress(item) })
            : t('未开始')
    return t('状态未知')
}

const loadTranscode = async (video: ProcessingVideo) => {
    const uid = props.uid
    const bvid = processing.value?.bvid
    if (
        !props.modelValue ||
        !uid ||
        !bvid ||
        !processing.value?.videos.some(item => item.cid === video.cid)
    )
        return
    const request = ++transcodeRequest
    selectedCid.value = video.cid
    transcode.value = null
    transcodeError.value = ''
    transcodeLoading.value = true
    try {
        const result = await utilsStore.getVideoTranscode(uid, bvid, video.cid)
        if (request !== transcodeRequest || !props.modelValue || props.uid !== uid) return
        transcode.value = result
        video.xcode_state = result.xcode_state
    } catch (cause) {
        if (request === transcodeRequest)
            transcodeError.value = t('获取转码进度失败: {0}', { 0: String(cause) })
    } finally {
        if (request === transcodeRequest) transcodeLoading.value = false
    }
}

const loadPage = async (nextPage: number) => {
    const uid = props.uid
    const aid = props.aid
    if (!props.modelValue || !uid || !aid) return
    const request = ++listRequest
    transcodeRequest += 1
    const previousCid = selectedCid.value
    processing.value = null
    transcode.value = null
    selectedCid.value = null
    error.value = ''
    transcodeError.value = ''
    transcodeLoading.value = false
    loading.value = true
    page.value = nextPage
    try {
        const result = await utilsStore.getArchiveProcessing(uid, aid, nextPage)
        if (request !== listRequest || !props.modelValue || props.uid !== uid || props.aid !== aid)
            return
        processing.value = result
        const video =
            result.videos.find(item => item.cid === previousCid) ||
            result.videos.find(item => item.xcode_state === 1) ||
            result.videos[0]
        if (video) void loadTranscode(video)
    } catch (cause) {
        if (request === listRequest)
            error.value = t('获取稿件处理进度失败: {0}', { 0: String(cause) })
    } finally {
        if (request === listRequest) loading.value = false
    }
}

watch(
    () => [props.modelValue, props.uid, props.aid] as const,
    ([visible, uid, aid]) => {
        listRequest += 1
        transcodeRequest += 1
        processing.value = null
        transcode.value = null
        selectedCid.value = null
        loading.value = false
        transcodeLoading.value = false
        error.value = ''
        transcodeError.value = ''
        page.value = 1
        if (visible && uid && aid) void loadPage(1)
    },
    { immediate: true }
)
onBeforeUnmount(() => {
    listRequest += 1
    transcodeRequest += 1
})
</script>

<style scoped>
.status-dialog-title {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: var(--el-dialog-title-font-size);
    color: var(--el-text-color-primary);
}
.processing-content {
    min-height: 180px;
}
.section-heading,
.resolution-heading,
.audit-steps {
    display: flex;
    align-items: center;
    gap: 12px;
}
.section-heading {
    margin-bottom: 16px;
}
.section-heading .el-pagination {
    margin-left: auto;
}
.resolution-heading {
    justify-content: space-between;
    margin-bottom: 10px;
}
.part-list {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(65px, 1fr));
    gap: 10px;
}
.part-list .el-button {
    margin: 0;
    height: 48px;
}
.resolution-list {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 24px 32px;
}
section {
    margin-top: 28px;
}
@media (max-width: 600px) {
    .resolution-list {
        grid-template-columns: 1fr;
    }
    .section-heading,
    .audit-steps {
        flex-wrap: wrap;
    }
}
</style>
