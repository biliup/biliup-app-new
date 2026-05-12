<template>
    <el-dialog
        v-model="visible"
        width="580px"
        :close-on-click-modal="!monitoring"
        :close-on-press-escape="!monitoring"
        :show-close="!monitoring"
        draggable
        center
    >
        <div class="folder-watch-content">
            <!-- 功能说明 -->
            <el-alert
                type="info"
                show-icon
                :closable="false"
                class="info-alert"
                v-if="!monitoring && !waitingForStart"
            >
                <div class="info-text">
                    <p>📁 <strong>文件夹监控功能：</strong></p>
                    <ul>
                        <li>选择监控文件夹，按设定间隔自动检测新增视频文件</li>
                        <li>可设置文件大小稳定检测次数，确保文件完整后再添加</li>
                        <li>自动将符合大小要求且稳定的视频文件添加到当前模板</li>
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
                <el-form :model="settings" label-width="110px" label-position="right" size="small">
                    <el-form-item label="监控文件夹" required>
                        <div class="folder-selection">
                            <div v-if="settings.folderPaths.length === 0" class="no-folders">
                                <el-text type="info">尚未选择监控文件夹</el-text>
                            </div>
                            <div v-else class="selected-folders">
                                <el-tag
                                    v-for="(folder, index) in settings.folderPaths"
                                    :key="index"
                                    closable
                                    @close="removeFolder(index)"
                                    class="folder-tag"
                                    type="primary"
                                >
                                    <el-tooltip :content="folder" placement="top">
                                        <span class="folder-path">{{
                                            getFolderDisplayName(folder)
                                        }}</span>
                                    </el-tooltip>
                                </el-tag>
                            </div>
                            <div class="folder-buttons">
                                <el-button type="primary" @click="selectFolder">
                                    <el-icon><folder-opened /></el-icon>
                                    添加文件夹
                                </el-button>
                                <el-button
                                    v-if="settings.folderPaths.length > 0"
                                    type="danger"
                                    @click="clearAllFolders"
                                    plain
                                >
                                    <el-icon><delete /></el-icon>
                                    清空全部
                                </el-button>
                            </div>
                        </div>
                    </el-form-item>

                    <el-form-item label="检测次数设置">
                        <el-input-number
                            v-model="settings.maxEmptyChecks"
                            :min="1"
                            :max="20"
                            :step="1"
                            controls-position="right"
                            size="small"
                            style="width: 180px"
                        />
                        <span class="setting-description">
                            连续检测此次数后自动提交稿件<br />（每轮检测如果有小文件或有不稳定的文件会重置计数器）
                        </span>
                    </el-form-item>

                    <el-form-item label="检测间隔时间">
                        <el-input-number
                            v-model="settings.checkInterval"
                            :min="5"
                            :max="3600"
                            :step="1"
                            controls-position="right"
                            size="small"
                            style="width: 180px"
                        />
                        <span class="setting-description">
                            检测间隔时间（秒），范围：5秒-3600秒（1小时）
                        </span>
                    </el-form-item>

                    <el-form-item label="稳定检测次数">
                        <el-input-number
                            v-model="settings.stableCheckCount"
                            :min="0"
                            :max="5"
                            :step="1"
                            controls-position="right"
                            size="small"
                            style="width: 180px"
                        />
                        <span class="setting-description">
                            文件大小连续相同次数后才添加，0表示不检测直接添加
                        </span>
                    </el-form-item>

                    <el-form-item label="监控范围">
                        <el-checkbox v-model="settings.includeSubfolders">
                            包含子文件夹
                        </el-checkbox>
                        <span class="setting-description">
                            勾选后将递归监控所有子文件夹中的视频文件（最大深度20）
                        </span>
                    </el-form-item>

                    <el-form-item label="最小文件大小">
                        <el-input-number
                            v-model="settings.minFileSize"
                            :min="0"
                            :max="999999"
                            :step="1"
                            controls-position="right"
                            size="small"
                            style="width: 180px"
                        />
                        <span class="setting-description">
                            过滤小于此大小的文件（MB），0为不过滤
                        </span>
                    </el-form-item>

                    <el-form-item label="文件名过滤">
                        <el-checkbox v-model="settings.enableFilenameFilter">
                            启用正则表达式过滤
                        </el-checkbox>
                        <span class="setting-description">
                            启用后，忽略符合正则表达式的文件名
                        </span>
                    </el-form-item>

                    <el-form-item v-if="settings.enableFilenameFilter" label="正则表达式">
                        <el-input
                            v-model="settings.filenameRegex"
                            placeholder="例如：^.*_temp\.|\.tmp$"
                            size="small"
                            style="width: 280px"
                            :class="{ 'regex-error': regexError }"
                        />
                        <span class="setting-description">
                            匹配要忽略的文件名的正则表达式，支持 JavaScript 正则语法
                        </span>
                        <div v-if="regexError" class="regex-error-msg">
                            {{ regexError }}
                        </div>
                        <div
                            v-if="settings.filenameRegex && !regexError"
                            class="regex-test-section"
                        >
                            <div class="test-input-group">
                                <el-input
                                    v-model="testFilename"
                                    placeholder="输入文件名测试忽略结果，例如：video_temp.mp4"
                                    size="small"
                                    style="width: 200px; margin-right: 8px"
                                    clearable
                                />
                                <span class="test-result" :class="getTestResultClass()">
                                    {{ getTestResult() }}
                                </span>
                            </div>
                        </div>
                    </el-form-item>

                    <el-form-item label="自动提交">
                        <el-checkbox v-model="settings.autoSubmit"> 启用 </el-checkbox>
                        <span class="setting-description">
                            启用后，连续{{ settings.maxEmptyChecks }}次检测，无变化后将自动提交到"{{
                                templateTitle || '当前模板'
                            }}"
                        </span>
                    </el-form-item>

                    <el-form-item v-if="settings.autoSubmit" label="提交模式">
                        <el-radio-group v-model="settings.autoSubmitMode" size="small">
                            <el-radio-button label="single" value="single"
                                >单稿件模式</el-radio-button
                            >
                            <el-radio-button label="multi" value="multi"
                                >多稿件模式</el-radio-button
                            >
                        </el-radio-group>
                        <span class="setting-description">
                            单稿件：按当前模板一次提交；多稿件：每个视频单独投稿一条稿件
                        </span>
                    </el-form-item>

                    <el-form-item label="定时开始">
                        <el-checkbox v-model="settings.delayedStart"> 启用定时开始 </el-checkbox>
                        <span class="setting-description"> 启用后，将在指定时间开始监控 </span>
                    </el-form-item>

                    <el-form-item v-if="settings.delayedStart" label="开始时间">
                        <el-date-picker
                            v-model="settings.startTime"
                            type="datetime"
                            placeholder="选择开始时间"
                            format="YYYY-MM-DD HH:mm:ss"
                            value-format="YYYY-MM-DD HH:mm:ss"
                            size="small"
                            :disabled-date="
                                (date: Date) => {
                                    const now = new Date()
                                    return date < now
                                }
                            "
                            style="width: 280px"
                        />
                    </el-form-item>
                </el-form>
            </div>

            <!-- 监控状态区域 -->
            <div v-if="monitoring || waitingForStart" class="monitoring-section">
                <div class="status-card">
                    <!-- 等待定时开始状态 -->
                    <div v-if="waitingForStart && !monitoring" class="status-header">
                        <el-icon class="status-icon waiting"><clock /></el-icon>
                        <h3>等待定时开始...</h3>
                    </div>

                    <!-- 正在监控状态 -->
                    <div v-else-if="monitoring" class="status-header">
                        <el-icon class="status-icon rotating"><loading /></el-icon>
                        <h3>正在监控中...</h3>
                    </div>

                    <div class="status-info">
                        <div><strong>监控路径：</strong></div>
                        <ul class="folder-list">
                            <li
                                v-for="folder in settings.folderPaths"
                                :key="folder"
                                class="folder-item"
                            >
                                {{ folder }}
                            </li>
                        </ul>
                        <p>
                            <strong>监控配置：</strong>
                            {{ settings.includeSubfolders ? '包含子文件夹' : '仅当前文件夹' }}，
                            最小文件大小 {{ settings.minFileSize }}MB
                            <span v-if="settings.enableFilenameFilter">
                                ，正则忽略：{{ settings.filenameRegex || '未设置' }}
                            </span>
                        </p>
                        <p v-if="settings.autoSubmit" class="auto-submit-info">
                            <strong>自动提交：</strong>连续
                            {{ settings.maxEmptyChecks }} 次检测，无变化后将自动提交到"{{
                                templateTitle || '当前模板'
                            }}" （{{
                                settings.autoSubmitMode === 'multi' ? '多稿件模式' : '单稿件模式'
                            }}）
                        </p>

                        <!-- 等待定时开始时显示倒计时 -->
                        <p v-if="waitingForStart && !monitoring">
                            <strong>预定开始时间：</strong>{{ settings.startTime }}
                        </p>
                        <p
                            v-if="waitingForStart && !monitoring && timeUntilStart"
                            class="countdown-info"
                        >
                            <strong>距离开始还有：</strong>{{ timeUntilStart }}
                        </p>

                        <!-- 监控进行中时显示检测信息 -->
                        <template v-if="monitoring">
                            <p>
                                <strong>检测轮数：</strong>{{ currentCheckRound }} /
                                {{ settings.maxEmptyChecks }}
                            </p>
                            <p><strong>下次检测：</strong>{{ nextCheckTime }}</p>
                            <p><strong>已添加文件：</strong>{{ addedFilesCount }} 个</p>
                        </template>
                    </div>

                    <div v-if="lastCheckResult && monitoring" class="last-check">
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
                <template v-if="!monitoring && !waitingForStart">
                    <el-button @click="closeDialog">取消</el-button>
                    <el-button
                        type="primary"
                        :disabled="
                            !settings.folderPaths.length ||
                            (settings.delayedStart && !settings.startTime)
                        "
                        @click="startMonitoring"
                    >
                        {{ settings.delayedStart ? '设置定时开始' : '开始监控' }}
                    </el-button>
                </template>
                <template v-else-if="waitingForStart && !monitoring">
                    <el-button type="warning" @click="cancelDelayedStart">取消定时</el-button>
                    <el-button type="primary" @click="startMonitoringNow">立即开始</el-button>
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
import { FolderOpened, Loading, Clock, Delete } from '@element-plus/icons-vue'
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
    (e: 'submit-videos', mode: 'single' | 'multi'): void
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
    folderPaths: [] as string[], // 改为多个文件夹路径的数组
    maxEmptyChecks: 5,
    checkInterval: 60, // 检测间隔时间（秒），默认60秒
    includeSubfolders: false, // 是否包含子文件夹
    minFileSize: 0, // 最小文件大小（MB），默认1MB
    autoSubmit: true, // 是否自动提交稿件
    delayedStart: false, // 是否启用定时开始
    startTime: null as string | null, // 开始时间
    stableCheckCount: 3, // 文件大小稳定检测次数，默认3次
    enableFilenameFilter: false, // 是否启用文件名正则过滤
    filenameRegex: '', // 文件名过滤正则表达式
    autoSubmitMode: 'single' as 'single' | 'multi' // 自动提交模式
})

// 监控状态
const monitoring = ref(false)
const waitingForStart = ref(false) // 等待定时开始状态
const currentCheckRound = ref(0)
const nextCheckTime = ref('')
const addedFilesCount = ref(0)
const lastCheckResult = ref<{
    newFiles: string[]
    resetCounter: boolean
    stableFiles: string[]
} | null>(null)

// 定时开始相关状态
const timeUntilStart = ref('')
const startCountdownTimer = ref<number | null>(null)

// 文件大小跟踪：存储每个文件最近3次的大小记录
const fileSizeHistory = ref<Map<string, number[]>>(new Map())

// 正则表达式相关
const regexError = ref('')
const compiledRegex = ref<RegExp | null>(null)
const testFilename = ref('')

// 定时器
let monitorTimer: number | null = null

// 监听窗口打开状态，每次打开时清空文件夹路径
watch(visible, (newValue, oldValue) => {
    if (newValue && !oldValue) {
        // 窗口从关闭变为打开状态，清空文件夹路径
        settings.value.folderPaths = []
    }
})

// 监听正则表达式变化，验证正则表达式
watch(
    () => settings.value.filenameRegex,
    newRegex => {
        regexError.value = ''
        compiledRegex.value = null

        if (newRegex.trim()) {
            try {
                compiledRegex.value = new RegExp(newRegex, 'i') // 不区分大小写
                regexError.value = ''
            } catch (error) {
                regexError.value = '正则表达式语法错误'
                compiledRegex.value = null
            }
        } else {
            // 正则表达式被清空时，也清空测试文件名
            testFilename.value = ''
        }
    },
    { immediate: true }
)

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
            multiple: true, // 允许多选
            title: '选择要监控的文件夹'
        })

        if (selected) {
            if (Array.isArray(selected)) {
                // 多选情况
                for (const folder of selected) {
                    if (!settings.value.folderPaths.includes(folder)) {
                        settings.value.folderPaths.push(folder)
                    }
                }
            } else if (typeof selected === 'string') {
                // 单选情况
                if (!settings.value.folderPaths.includes(selected)) {
                    settings.value.folderPaths.push(selected)
                }
            }
        }
    } catch (error) {
        console.error('选择文件夹失败:', error)
        utilsStore.showMessage('选择文件夹失败', 'error')
    }
}

// 移除文件夹
const removeFolder = (index: number) => {
    settings.value.folderPaths.splice(index, 1)
}

// 清空所有文件夹
const clearAllFolders = () => {
    settings.value.folderPaths = []
}

// 获取文件夹显示名称（截取路径末尾部分）
const getFolderDisplayName = (folderPath: string): string => {
    const maxLength = 30
    if (folderPath.length <= maxLength) {
        return folderPath
    }
    return '...' + folderPath.slice(-maxLength + 3)
}

// 检查文件是否为支持的视频格式
const isSupportedVideoFormat = (filename: string): boolean => {
    const ext = filename.toLowerCase().substring(filename.lastIndexOf('.'))
    return supportedFormats.includes(ext)
}

// 获取测试文件名的匹配结果
const getTestResult = (): string => {
    if (!testFilename.value.trim()) {
        return '请输入文件名进行测试'
    }

    if (!compiledRegex.value) {
        return '正则表达式无效'
    }

    const isMatch = compiledRegex.value.test(testFilename.value.trim())
    return isMatch ? '✓ 将被忽略' : '✗ 将被监控'
}

// 获取测试结果的CSS类
const getTestResultClass = (): string => {
    if (!testFilename.value.trim() || !compiledRegex.value) {
        return 'test-neutral'
    }

    const isMatch = compiledRegex.value.test(testFilename.value.trim())
    // 逻辑反转：匹配的文件会被忽略（红色），不匹配的文件会被监控（绿色）
    return isMatch ? 'test-no-match' : 'test-match'
}

// 获取当前视频文件名列表
const getCurrentVideoNames = (): string[] => {
    return props.currentVideos.map(video => video.filename || '').filter(Boolean)
}

const getCurrentVideoTitles = (): string[] => {
    return props.currentVideos.map(video => video.title || '').filter(Boolean)
}

// 检查文件大小是否稳定（连续N次大小相同）
const isFileSizeStable = (filename: string, currentSize: number): boolean => {
    // 如果设置为0，直接返回true（不检测稳定性，直接添加）
    if (settings.value.stableCheckCount === 0) {
        return true
    }

    const history = fileSizeHistory.value.get(filename) || []

    // 更新文件大小历史记录
    history.push(currentSize)

    // 只保留最近N次记录
    if (history.length > settings.value.stableCheckCount) {
        history.shift()
    }

    fileSizeHistory.value.set(filename, history)

    // 检查是否有连续N次相同的大小记录
    if (history.length >= settings.value.stableCheckCount) {
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
        // 收集所有文件夹的文件
        const allEntries: Array<{ name: string; path: string; isDirectory: boolean }> = []

        for (const folderPath of settings.value.folderPaths) {
            try {
                const entries = await utilsStore
                    .readDirRecursive(folderPath, settings.value.includeSubfolders, 20)
                    .then(files =>
                        files.map(file => ({
                            name: file.name,
                            path: file.path,
                            isDirectory: file.is_directory
                        }))
                    )
                allEntries.push(...entries)
            } catch (error) {
                console.error(`读取文件夹 ${folderPath} 失败:`, error)
                // 继续处理其他文件夹，不因单个文件夹失败而中断
            }
        }

        const currentVideoNames = getCurrentVideoNames()
        const currentVideoTitles = getCurrentVideoTitles()
        const newFiles: string[] = []
        const stableFiles: string[] = []
        let resetCounter = false

        // 按文件名排序，去重
        const uniqueEntries = Array.from(
            new Map(allEntries.map(entry => [entry.path, entry])).values()
        )
        const sortedEntries = uniqueEntries
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
                const shouldIgnoreByRegex = settings.value.enableFilenameFilter
                    ? compiledRegex.value
                        ? compiledRegex.value.test(entry.name)
                        : false
                    : false

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
                                if (shouldIgnoreByRegex) {
                                    // 过滤项仍参与监控，但不加入待添加列表
                                    console.log(`文件 ${entry.name} 命中过滤规则，仅监控不添加`)
                                } else {
                                    // 文件大小稳定，可以添加
                                    newFiles.push(filePath)
                                    stableFiles.push(entry.name)
                                    console.log(
                                        `添加文件: ${entry.name} (${fileSizeMB.toFixed(2)}MB)`
                                    )
                                }
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
                emit('submit-videos', settings.value.autoSubmitMode)
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
    if (!settings.value.folderPaths || settings.value.folderPaths.length === 0) {
        utilsStore.showMessage('请先选择监控文件夹', 'error')
        return
    }

    // 如果启用了定时开始
    if (settings.value.delayedStart && settings.value.startTime) {
        const startTime = new Date(settings.value.startTime)
        const now = new Date()

        if (startTime <= now) {
            utilsStore.showMessage('开始时间不能早于当前时间', 'error')
            return
        }

        // 设置等待状态
        waitingForStart.value = true

        // 开始倒计时
        startCountdown()

        // 设置定时器，在指定时间开始监控
        const delay = startTime.getTime() - now.getTime()
        setTimeout(() => {
            if (waitingForStart.value) {
                startMonitoringNow()
            }
        }, delay)

        utilsStore.showMessage(
            `已设置定时开始，将在 ${settings.value.startTime} 开始监控`,
            'success'
        )
        return
    }

    // 立即开始监控
    startMonitoringNow()
}

// 立即开始监控（实际的监控逻辑）
const startMonitoringNow = async () => {
    monitoring.value = true
    waitingForStart.value = false
    currentCheckRound.value = 0
    addedFilesCount.value = 0

    // 清除倒计时定时器
    if (startCountdownTimer.value) {
        clearInterval(startCountdownTimer.value)
        startCountdownTimer.value = null
    }

    // 清空文件大小历史记录
    fileSizeHistory.value.clear()

    const folderMsg = settings.value.includeSubfolders
        ? `开始监控文件夹 (${settings.value.folderPaths.length}个，包含子文件夹): ${settings.value.folderPaths.join(', ')}`
        : `开始监控文件夹 (${settings.value.folderPaths.length}个): ${settings.value.folderPaths.join(', ')}`

    let configMsg = `最小文件大小: ${settings.value.minFileSize}MB`
    if (settings.value.enableFilenameFilter && settings.value.filenameRegex) {
        configMsg += `，正则忽略: ${settings.value.filenameRegex}`
    }
    console.log(`${folderMsg}，${configMsg}`)

    // 立即执行第一次检测
    await performMonitoringCycle()

    // 设置定时器，按配置的间隔检测
    monitorTimer = setInterval(performMonitoringCycle, settings.value.checkInterval * 1000)

    let successMsg = settings.value.includeSubfolders
        ? `开始监控文件夹（包含子文件夹，最小${settings.value.minFileSize}MB`
        : `开始监控文件夹（最小${settings.value.minFileSize}MB`

    if (settings.value.enableFilenameFilter && settings.value.filenameRegex) {
        successMsg += `，正则忽略）`
    } else {
        successMsg += `）`
    }

    utilsStore.showMessage(successMsg, 'success')
}

// 取消定时开始
const cancelDelayedStart = () => {
    waitingForStart.value = false

    if (startCountdownTimer.value) {
        clearInterval(startCountdownTimer.value)
        startCountdownTimer.value = null
    }

    utilsStore.showMessage('已取消定时开始', 'info')
}

// 开始倒计时显示
const startCountdown = () => {
    if (!settings.value.startTime) return

    const updateCountdown = () => {
        const startTime = new Date(settings.value.startTime!)
        const now = new Date()
        const diff = startTime.getTime() - now.getTime()

        if (diff <= 0) {
            timeUntilStart.value = '即将开始...'
            if (startCountdownTimer.value) {
                clearInterval(startCountdownTimer.value)
                startCountdownTimer.value = null
            }
            return
        }

        const days = Math.floor(diff / (1000 * 60 * 60 * 24))
        const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60))
        const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60))
        const seconds = Math.floor((diff % (1000 * 60)) / 1000)

        let countdownText = ''
        if (days > 0) {
            countdownText += `${days}天 `
        }
        if (hours > 0 || days > 0) {
            countdownText += `${hours}小时 `
        }
        if (minutes > 0 || hours > 0 || days > 0) {
            countdownText += `${minutes}分钟 `
        }
        countdownText += `${seconds}秒`

        timeUntilStart.value = countdownText
    }

    // 立即更新一次
    updateCountdown()

    // 每秒更新倒计时
    startCountdownTimer.value = setInterval(updateCountdown, 1000)
}

// 停止监控
const stopMonitoring = () => {
    monitoring.value = false
    waitingForStart.value = false

    if (monitorTimer) {
        clearInterval(monitorTimer)
        monitorTimer = null
    }

    if (startCountdownTimer.value) {
        clearInterval(startCountdownTimer.value)
        startCountdownTimer.value = null
    }

    utilsStore.showMessage('已停止文件夹监控', 'info')
}

// 关闭对话框
const closeDialog = () => {
    if (monitoring.value || waitingForStart.value) {
        stopMonitoring()
    }
    visible.value = false
}

// 清理定时器
onUnmounted(() => {
    if (monitorTimer) {
        clearInterval(monitorTimer)
    }
    if (startCountdownTimer.value) {
        clearInterval(startCountdownTimer.value)
    }
})
</script>

<style scoped>
.folder-watch-content {
    padding: 5px 0;
}

.info-alert {
    margin-bottom: 15px;
}

.info-alert :deep(.el-alert__content) {
    padding: 8px 0;
}

.info-text {
    line-height: 1.4;
}

.info-text p {
    margin: 0 0 6px 0;
    font-weight: 500;
    font-size: 13px;
}

.info-text ul {
    margin: 6px 0 0 0;
    padding-left: 18px;
}

.info-text li {
    margin: 3px 0;
    font-size: 12px;
}

.settings-section {
    margin-bottom: 15px;
}

.settings-section :deep(.el-form-item) {
    margin-bottom: 12px;
}

.settings-section :deep(.el-form-item__label) {
    font-size: 13px;
    line-height: 1.2;
    padding-bottom: 4px;
}

.settings-section :deep(.el-form-item__content) {
    line-height: 1.2;
}

/* 多文件夹选择相关样式 */
.folder-selection {
    width: 100%;
}

.no-folders {
    padding: 12px;
    text-align: center;
    background: #f5f7fa;
    border: 1px dashed #dcdfe6;
    border-radius: 4px;
    margin-bottom: 12px;
}

.selected-folders {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 12px;
    min-height: 32px;
    padding: 8px;
    background: #f5f7fa;
    border: 1px solid #dcdfe6;
    border-radius: 4px;
}

.folder-tag {
    max-width: 200px;
    display: flex;
    align-items: center;
}

.folder-path {
    display: inline-block;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.folder-buttons {
    display: flex;
    gap: 8px;
    justify-content: flex-start;
}

.folder-list {
    margin: 8px 0;
    padding-left: 20px;
    list-style-type: disc;
}

.folder-item {
    margin: 4px 0;
    color: #606266;
    font-size: 12px;
    word-break: break-all;
}

.setting-description {
    margin-left: 8px;
    font-size: 11px;
    color: #909399;
    line-height: 1.3;
}

.monitoring-section {
    padding: 15px 0;
}

.status-card {
    background: #f5f7fa;
    border-radius: 6px;
    padding: 15px;
    border: 1px solid #e4e7ed;
}

.status-header {
    display: flex;
    align-items: center;
    margin-bottom: 12px;
}

.status-icon {
    font-size: 18px;
    color: #409eff;
    margin-right: 6px;
}

.status-icon.waiting {
    color: #e6a23c;
    animation: pulse 2s ease-in-out infinite;
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

@keyframes pulse {
    0%,
    100% {
        opacity: 1;
    }
    50% {
        opacity: 0.5;
    }
}

.status-header h3 {
    margin: 0;
    color: #303133;
    font-size: 15px;
}

.status-info {
    margin-bottom: 12px;
}

.status-info p {
    margin: 6px 0;
    color: #606266;
    font-size: 13px;
}

.auto-submit-info {
    color: #67c23a !important;
    font-weight: 500;
}

.countdown-info {
    color: #e6a23c !important;
    font-weight: 500;
    font-size: 15px;
}

.last-check h4 {
    margin: 0 0 6px 0;
    color: #303133;
    font-size: 13px;
}

.last-check ul {
    margin: 0;
    padding-left: 16px;
}

.last-check li {
    margin: 3px 0;
    color: #606266;
    font-size: 12px;
}

.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
}

/* 正则表达式相关样式 */
.regex-error {
    border-color: #f56c6c !important;
}

.regex-error-msg {
    color: #f56c6c;
    font-size: 12px;
    margin-top: 4px;
    line-height: 1.2;
}

/* 正则表达式测试相关样式 */
.regex-test-section {
    margin-top: 8px;
}

.test-input-group {
    display: flex;
    align-items: center;
}

.test-result {
    font-size: 12px;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 4px;
    white-space: nowrap;
}

.test-neutral {
    color: #909399;
    background: #f4f4f5;
}

.test-no-match {
    color: #67c23a;
    background: #f0f9ff;
}

.test-match {
    color: #f56c6c;
    background: #fef0f0;
}
</style>
