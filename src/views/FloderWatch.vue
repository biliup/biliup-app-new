<template>
    <el-dialog
        v-model="visible"
        title="文件夹监控"
        width="600px"
        :close-on-click-modal="!monitoring"
        :close-on-press-escape="!monitoring"
        :show-close="!monitoring"
        center
    >
        <div class="folder-watch-content">
            <!-- 功能说明 -->
            <el-alert title="功能说明" type="info" show-icon :closable="false" class="info-alert">
                <div class="info-text">
                    <p>📁 <strong>文件夹监控功能：</strong></p>
                    <ul>
                        <li>选择监控文件夹，每分钟自动检测新增视频文件</li>
                        <li>文件需连续3次检测大小无变化才会被添加（确保文件完整）</li>
                        <li>自动将大于1KB且稳定的视频文件添加到当前模板</li>
                        <li>
                            连续{{
                                settings.maxEmptyChecks
                            }}次检测无小文件（≤1KB），且无大小持续改变的文件后自动提交稿件
                        </li>
                    </ul>
                </div>
            </el-alert>

            <!-- 设置区域 -->
            <div v-if="!monitoring" class="settings-section">
                <el-form :model="settings" label-width="140px" label-position="left">
                    <el-form-item label="监控文件夹：" required>
                        <el-input
                            v-model="settings.folderPath"
                            placeholder="请选择要监控的文件夹"
                            readonly
                        >
                            <template #append>
                                <el-button type="primary" @click="selectFolder">
                                    <el-icon><folder-opened /></el-icon>
                                    选择文件夹
                                </el-button>
                            </template>
                        </el-input>
                    </el-form-item>

                    <el-form-item label="检测次数设置：">
                        <el-input-number
                            v-model="settings.maxEmptyChecks"
                            :min="1"
                            :max="20"
                            :step="1"
                            controls-position="right"
                            style="width: 200px"
                        />
                        <span class="setting-description">
                            连续检测此次数无小文件后自动提交稿件
                        </span>
                    </el-form-item>
                </el-form>
            </div>

            <!-- 监控状态区域 -->
            <div v-if="monitoring" class="monitoring-section">
                <div class="status-card">
                    <div class="status-header">
                        <el-icon class="status-icon rotating"><loading /></el-icon>
                        <h3>正在监控中...</h3>
                    </div>

                    <div class="status-info">
                        <p><strong>监控路径：</strong>{{ settings.folderPath }}</p>
                        <p>
                            <strong>检测轮数：</strong>{{ currentCheckRound }} /
                            {{ settings.maxEmptyChecks }}
                        </p>
                        <p><strong>下次检测：</strong>{{ nextCheckTime }}</p>
                        <p><strong>已添加文件：</strong>{{ addedFilesCount }} 个</p>
                    </div>

                    <div v-if="lastCheckResult" class="last-check">
                        <h4>最近检测结果：</h4>
                        <ul>
                            <li v-if="lastCheckResult.newFiles.length > 0">
                                已添加稳定文件：{{ lastCheckResult.stableFiles.join(', ') }}
                            </li>
                            <li
                                v-if="
                                    lastCheckResult.stableFiles.length === 0 &&
                                    lastCheckResult.newFiles.length === 0
                                "
                            >
                                本次检测无新增稳定文件
                            </li>
                        </ul>
                    </div>
                </div>
            </div>
        </div>

        <!-- 按钮区域 -->
        <template #footer>
            <div class="dialog-footer">
                <template v-if="!monitoring">
                    <el-button @click="closeDialog">取消</el-button>
                    <el-button
                        type="primary"
                        :disabled="!settings.folderPath"
                        @click="startMonitoring"
                    >
                        开始监控
                    </el-button>
                </template>
                <template v-else>
                    <el-button type="danger" @click="stopMonitoring">停止监控</el-button>
                </template>
            </div>
        </template>
    </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, onUnmounted, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { FolderOpened, Loading } from '@element-plus/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import { readDir, stat } from '@tauri-apps/plugin-fs'

// Props and Emits
interface Props {
    modelValue: boolean
    currentVideos: any[]
}

interface Emits {
    (e: 'update:modelValue', value: boolean): void
    (e: 'add-videos', files: any[]): void
    (e: 'submit-videos'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// 显示状态
const visible = computed({
    get: () => props.modelValue,
    set: value => emit('update:modelValue', value)
})

// 设置
const settings = ref({
    folderPath: '',
    maxEmptyChecks: 5
})

// 监控状态
const monitoring = ref(false)
const currentCheckRound = ref(0)
const nextCheckTime = ref('')
const addedFilesCount = ref(0)
const lastCheckResult = ref<{
    newFiles: string[]
    resetCounter: boolean
    stableFiles: string[]
} | null>(null)

// 文件大小跟踪：存储每个文件最近3次的大小记录
const fileSizeHistory = ref<Map<string, number[]>>(new Map())

// 定时器
let monitorTimer: number | null = null

// 监听窗口打开状态，每次打开时清空文件夹路径
watch(visible, (newValue, oldValue) => {
    if (newValue && !oldValue) {
        // 窗口从关闭变为打开状态，清空文件夹路径
        settings.value.folderPath = ''
    }
})

// 支持的视频格式
const supportedFormats = [
    '.mp4',
    '.flv',
    '.avi',
    '.wmv',
    '.mov',
    '.webm',
    '.mpeg4',
    '.ts',
    '.mpg',
    '.rm',
    '.rmvb',
    '.mkv',
    '.m4v'
]

// 选择文件夹
const selectFolder = async () => {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: '选择要监控的文件夹'
        })

        if (selected && typeof selected === 'string') {
            settings.value.folderPath = selected
        }
    } catch (error) {
        console.error('选择文件夹失败:', error)
        ElMessage.error('选择文件夹失败')
    }
}

// 检查文件是否为支持的视频格式
const isSupportedVideoFormat = (filename: string): boolean => {
    const ext = filename.toLowerCase().substring(filename.lastIndexOf('.'))
    return supportedFormats.includes(ext)
}

// 获取当前视频文件名列表
const getCurrentVideoNames = (): string[] => {
    return props.currentVideos.map(video => video.filename || '').filter(Boolean)
}

const getCurrentVideoTitles = (): string[] => {
    return props.currentVideos.map(video => video.title || '').filter(Boolean)
}

// 检查文件大小是否稳定（连续3次大小相同）
const isFileSizeStable = (filename: string, currentSize: number): boolean => {
    const history = fileSizeHistory.value.get(filename) || []

    // 更新文件大小历史记录
    history.push(currentSize)

    // 只保留最近3次记录
    if (history.length > 3) {
        history.shift()
    }

    fileSizeHistory.value.set(filename, history)

    // 检查是否有连续3次相同的大小记录
    if (history.length >= 3) {
        const allSame = history.every(size => size === history[0])
        return allSame
    }

    return false
}

// 执行一次文件夹检测
const performCheck = async (): Promise<{
    newFiles: string[]
    resetCounter: boolean
    stableFiles: string[]
}> => {
    try {
        const entries = await readDir(settings.value.folderPath)
        const currentVideoNames = getCurrentVideoNames()
        const currentVideoTitles = getCurrentVideoTitles()
        const newFiles: string[] = []
        const stableFiles: string[] = []
        let resetCounter = false

        // 按文件名排序
        const sortedEntries = entries
            .filter(entry => !entry.isDirectory) // 只处理文件，不处理文件夹
            .sort((a, b) => (a.name || '').localeCompare(b.name || ''))

        for (const entry of sortedEntries) {
            if (!entry.name) continue

            // 根据操作系统选择正确的路径分隔符
            const separator = navigator.platform.toLowerCase().includes('win') ? '\\' : '/'
            const filePath = `${settings.value.folderPath}${separator}${entry.name}`

            try {
                // 获取文件状态信息
                const fileStats = await stat(filePath)
                const fileSize = fileStats.size || 0

                const isVideoFile = isSupportedVideoFormat(entry.name)

                if (isVideoFile) {
                    // 检查文件大小
                    if (fileSize <= 1024) {
                        resetCounter = true
                    } else {
                        // 检查文件是否已在当前视频列表中
                        const fileAlreadyExists =
                            currentVideoNames.includes(entry.name) ||
                            currentVideoTitles.includes(entry.name.replace(/\.[^/.]+$/, ''))

                        if (!fileAlreadyExists) {
                            // 检查文件大小是否稳定
                            const isStable = isFileSizeStable(entry.name, fileSize)

                            if (isStable) {
                                // 文件大小稳定，可以添加
                                newFiles.push(filePath)
                                stableFiles.push(entry.name)
                            } else {
                                resetCounter = true
                            }
                        }
                    }
                }
            } catch (statError) {
                console.warn(`获取文件 ${entry.name} 状态失败:`, statError)
                // 如果无法获取文件状态，跳过该文件
                continue
            }
        }

        return { newFiles, resetCounter, stableFiles }
    } catch (error) {
        console.error('检测文件夹失败:', error)
        throw error
    }
}

// 添加新文件到视频列表
const addNewFiles = async (filenames: string[]) => {
    if (filenames.length > 0) {
        emit('add-videos', filenames)
        addedFilesCount.value += filenames.length
        ElMessage.success(`已添加 ${filenames.length} 个视频文件`)
    }
}

// 更新下次检测时间显示
const updateNextCheckTime = () => {
    const next = new Date(Date.now() + 60000) // 1分钟后
    nextCheckTime.value = next.toLocaleTimeString()
}

// 执行监控循环
const performMonitoringCycle = async () => {
    try {
        const result = await performCheck()
        lastCheckResult.value = result

        // 添加新文件
        if (result.newFiles.length > 0) {
            await addNewFiles(result.newFiles)
        }

        if (result.resetCounter) {
            currentCheckRound.value = 0
        } else {
            currentCheckRound.value++
        }

        // 检查是否达到提交条件
        if (currentCheckRound.value >= settings.value.maxEmptyChecks) {
            ElMessage.success(`连续 ${settings.value.maxEmptyChecks} 次检测无小文件，自动提交稿件`)
            emit('submit-videos')
            closeDialog()
            return
        }

        // 更新下次检测时间
        updateNextCheckTime()
    } catch (error) {
        console.error('监控检测失败:', error)
        ElMessage.error('监控检测失败，请检查文件夹路径')
        stopMonitoring()
    }
}

// 开始监控
const startMonitoring = async () => {
    if (!settings.value.folderPath) {
        ElMessage.error('请先选择监控文件夹')
        return
    }

    monitoring.value = true
    currentCheckRound.value = 0
    addedFilesCount.value = 0

    // 清空文件大小历史记录
    fileSizeHistory.value.clear()

    // 立即执行第一次检测
    await performMonitoringCycle()

    // 设置定时器，每分钟检测一次
    monitorTimer = setInterval(performMonitoringCycle, 60000)

    ElMessage.success('开始监控文件夹')
}

// 停止监控
const stopMonitoring = () => {
    monitoring.value = false

    if (monitorTimer) {
        clearInterval(monitorTimer)
        monitorTimer = null
    }

    ElMessage.info('已停止文件夹监控')
}

// 关闭对话框
const closeDialog = () => {
    if (monitoring.value) {
        stopMonitoring()
    }
    visible.value = false
}

// 清理定时器
onUnmounted(() => {
    if (monitorTimer) {
        clearInterval(monitorTimer)
    }
})
</script>

<style scoped>
.folder-watch-content {
    padding: 10px 0;
}

.info-alert {
    margin-bottom: 20px;
}

.info-text {
    line-height: 1.6;
}

.info-text p {
    margin: 0 0 8px 0;
    font-weight: 500;
}

.info-text ul {
    margin: 8px 0 0 0;
    padding-left: 20px;
}

.info-text li {
    margin: 4px 0;
}

.settings-section {
    margin-bottom: 20px;
}

.setting-description {
    margin-left: 12px;
    font-size: 12px;
    color: #909399;
}

.monitoring-section {
    padding: 20px 0;
}

.status-card {
    background: #f5f7fa;
    border-radius: 8px;
    padding: 20px;
    border: 1px solid #e4e7ed;
}

.status-header {
    display: flex;
    align-items: center;
    margin-bottom: 16px;
}

.status-icon {
    font-size: 20px;
    color: #409eff;
    margin-right: 8px;
}

.rotating {
    animation: rotate 2s linear infinite;
}

@keyframes rotate {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

.status-header h3 {
    margin: 0;
    color: #303133;
}

.status-info {
    margin-bottom: 16px;
}

.status-info p {
    margin: 8px 0;
    color: #606266;
}

.last-check h4 {
    margin: 0 0 8px 0;
    color: #303133;
    font-size: 14px;
}

.last-check ul {
    margin: 0;
    padding-left: 20px;
}

.last-check li {
    margin: 4px 0;
    color: #606266;
    font-size: 13px;
}

.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
}
</style>
