<template>
    <el-dialog
        v-model="visible"
        title="文件夹监控"
        width="600px"
        :close-on-click-modal="!monitoring"
        :close-on-press-escape="!monitoring"
        :show-close="!monitoring"
        draggable
        center
    >
        <div class="folder-watch-content">
            <!-- 功能说明 -->
            <el-alert type="info" show-icon :closable="false" class="info-alert" v-if="!monitoring">
                <div class="info-text">
                    <p>📁 <strong>文件夹监控功能：</strong></p>
                    <ul>
                        <li>选择监控文件夹，按设定间隔自动检测新增视频文件</li>
                        <li>文件需连续3次检测大小无变化才会被添加（确保文件完整）</li>
                        <li>自动将符合大小要求且稳定的视频文件添加到当前模板</li>
                        <li>支持设置最小文件大小过滤，跳过过小的文件</li>
                        <li v-if="settings.autoSubmit">
                            启用自动提交后，连续{{
                                settings.maxEmptyChecks
                            }}次检测，无小于1KB且无大小持续改变的文件后自动提交稿件
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
                        <span class="setting-description"> 连续检测此次数后自动提交稿件 </span>
                    </el-form-item>

                    <el-form-item label="检测间隔时间：">
                        <el-input-number
                            v-model="settings.checkInterval"
                            :min="5"
                            :max="3600"
                            :step="1"
                            controls-position="right"
                            style="width: 200px"
                        />
                        <span class="setting-description">
                            检测间隔时间（秒），范围：5秒-3600秒（1小时）
                        </span>
                    </el-form-item>

                    <el-form-item label="监控范围：">
                        <el-checkbox v-model="settings.includeSubfolders">
                            包含子文件夹
                        </el-checkbox>
                        <span class="setting-description">
                            勾选后将递归监控所有子文件夹中的视频文件（最大深度20）
                        </span>
                    </el-form-item>

                    <el-form-item label="最小文件大小：">
                        <el-input-number
                            v-model="settings.minFileSize"
                            :min="0"
                            :max="999999"
                            :step="1"
                            controls-position="right"
                            style="width: 200px"
                        />
                        <span class="setting-description">
                            过滤小于此大小的文件（MB），0为不过滤
                        </span>
                    </el-form-item>

                    <el-form-item label="自动提交：">
                        <el-checkbox v-model="settings.autoSubmit"> 启用 </el-checkbox>
                        <span class="setting-description">
                            启用后，连续{{ settings.maxEmptyChecks }}次检测，无变化后将自动提交到"{{
                                templateTitle || '当前模板'
                            }}"
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
                            <strong>监控配置：</strong>
                            {{ settings.includeSubfolders ? '包含子文件夹' : '仅当前文件夹' }}，
                            最小文件大小 {{ settings.minFileSize }}MB
                        </p>
                        <p v-if="settings.autoSubmit" class="auto-submit-info">
                            <strong>自动提交：</strong>连续
                            {{ settings.maxEmptyChecks }} 次检测，无变化后将自动提交到"{{
                                templateTitle || '当前模板'
                            }}"
                        </p>
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
import { FolderOpened, Loading } from '@element-plus/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useUtilsStore } from '../stores/utils'

// Props and Emits
interface Props {
    modelValue: boolean
    currentVideos: any[]
    templateTitle?: string
}

interface Emits {
    (e: 'update:modelValue', value: boolean): void
    (e: 'add-videos', files: any[]): void
    (e: 'submit-videos'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()
const utilsStore = useUtilsStore()

// 模板标题
const templateTitle = computed(() => props.templateTitle)

// 显示状态
const visible = computed({
    get: () => props.modelValue,
    set: value => emit('update:modelValue', value)
})

// 设置
const settings = ref({
    folderPath: '',
    maxEmptyChecks: 5,
    checkInterval: 60, // 检测间隔时间（秒），默认60秒
    includeSubfolders: false, // 是否包含子文件夹
    minFileSize: 0, // 最小文件大小（MB），默认1MB
    autoSubmit: true // 是否自动提交稿件
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
        utilsStore.showMessage('选择文件夹失败', 'error')
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
        // 根据设置决定是否递归读取目录
        const entries = await utilsStore
            .readDirRecursive(settings.value.folderPath, settings.value.includeSubfolders, 20)
            .then(files =>
                files.map(file => ({
                    name: file.name,
                    path: file.path,
                    isDirectory: file.is_directory
                }))
            )

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

            const filePath = entry.path

            try {
                // 获取文件大小
                const fileSize = await utilsStore.getFileSize(filePath)
                const fileSizeMB = fileSize / (1024 * 1024) // 转换为MB

                const isVideoFile = isSupportedVideoFormat(entry.name)

                if (isVideoFile) {
                    // 检查文件大小是否符合最小要求
                    if (fileSizeMB < settings.value.minFileSize) {
                        console.log(
                            `文件 ${entry.name} 大小 ${fileSizeMB.toFixed(2)}MB 小于最小要求 ${settings.value.minFileSize}MB，跳过`
                        )
                        continue
                    }

                    // 检查文件大小（原有的小文件检查逻辑保留用于重置计数器）
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
                                console.log(`添加文件: ${entry.name} (${fileSizeMB.toFixed(2)}MB)`)
                            } else {
                                resetCounter = true
                            }
                        }
                    }
                }
            } catch (statError) {
                // 更详细的错误日志
                const errorMsg = statError instanceof Error ? statError.message : String(statError)
                if (
                    errorMsg.includes('forbidden') ||
                    errorMsg.includes('permission') ||
                    errorMsg.includes('access')
                ) {
                    console.warn(`权限不足，跳过文件: ${entry.name} (${filePath})`)
                } else {
                    console.warn(`获取文件 ${entry.name} 状态失败:`, statError)
                }
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
        utilsStore.showMessage(`已添加 ${filenames.length} 个视频文件`, 'success')
    }
}

// 更新下次检测时间显示
const updateNextCheckTime = () => {
    const next = new Date(Date.now() + settings.value.checkInterval * 1000)
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

        // 检查是否达到结束条件
        if (currentCheckRound.value >= settings.value.maxEmptyChecks) {
            if (settings.value.autoSubmit) {
                utilsStore.showMessage(
                    `连续 ${settings.value.maxEmptyChecks} 次检测，自动提交稿件到"${templateTitle.value || '当前模板'}"`,
                    'success'
                )
                emit('submit-videos')
            } else {
                utilsStore.showMessage(
                    `连续 ${settings.value.maxEmptyChecks} 次检测，文件夹监控结束}"`,
                    'success'
                )
            }
            closeDialog()
        }

        // 更新下次检测时间
        updateNextCheckTime()
    } catch (error) {
        console.error('监控检测失败:', error)
        utilsStore.showMessage('监控检测失败，请检查文件夹路径', 'error')
        stopMonitoring()
    }
}

// 开始监控
const startMonitoring = async () => {
    if (!settings.value.folderPath) {
        utilsStore.showMessage('请先选择监控文件夹', 'error')
        return
    }

    monitoring.value = true
    currentCheckRound.value = 0
    addedFilesCount.value = 0

    // 清空文件大小历史记录
    fileSizeHistory.value.clear()

    const folderMsg = settings.value.includeSubfolders
        ? `开始监控文件夹: ${settings.value.folderPath} (包含子文件夹)`
        : `开始监控文件夹: ${settings.value.folderPath}`
    console.log(`${folderMsg}，最小文件大小: ${settings.value.minFileSize}MB`)

    // 立即执行第一次检测
    await performMonitoringCycle()

    // 设置定时器，按配置的间隔检测
    monitorTimer = setInterval(performMonitoringCycle, settings.value.checkInterval * 1000)

    const successMsg = settings.value.includeSubfolders
        ? `开始监控文件夹（包含子文件夹，最小${settings.value.minFileSize}MB）`
        : `开始监控文件夹（最小${settings.value.minFileSize}MB）`
    utilsStore.showMessage(successMsg, 'success')
}

// 停止监控
const stopMonitoring = () => {
    monitoring.value = false

    if (monitorTimer) {
        clearInterval(monitorTimer)
        monitorTimer = null
    }

    utilsStore.showMessage('已停止文件夹监控', 'info')
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

.auto-submit-info {
    color: #67c23a !important;
    font-weight: 500;
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
