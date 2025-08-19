<template>
    <el-dialog
        v-model="visible"
        title="全局设置"
        width="500px"
        :before-close="handleClose"
        @close="handleDialogClose"
    >
        <el-form :model="configForm" label-width="140px" v-loading="loading">
            <!-- 配置分类标签 -->
            <el-divider content-position="left">
                <el-text type="primary" size="large">全局设置</el-text>
            </el-divider>

            <!-- 最大并发任务数 -->
            <el-form-item label="最大并发任务数">
                <div class="slider-container">
                    <el-slider
                        v-model="configForm.max_curr"
                        :min="1"
                        :max="12"
                        :step="1"
                        show-stops
                        show-input
                        :input-size="'small'"
                    />
                </div>
                <div class="form-tip">控制同时进行的上传任务数量，建议根据网络状况调整</div>
                <div class="form-tip">已经开始的任务，即使暂停或失败同样占用一条并发数</div>
            </el-form-item>

            <!-- 自动添加到上传队列 -->
            <el-form-item label="自动添加到上传队列">
                <el-switch
                    v-model="configForm.auto_upload"
                    active-text="开启"
                    inactive-text="关闭"
                />
                <div class="form-tip">开启后，创建上传任务时会自动添加到上传队列</div>
            </el-form-item>

            <!-- 自动开始任务 -->
            <el-form-item label="自动开始任务">
                <el-switch
                    v-model="configForm.auto_start"
                    active-text="开启"
                    inactive-text="关闭"
                />
                <div class="form-tip">开启后，任务添加到队列后会自动开始上传</div>
            </el-form-item>

            <!-- 用户配置分类标签 -->
            <el-divider content-position="left">
                <el-text type="primary" size="large">用户配置</el-text>
            </el-divider>

            <!-- 用户选择下拉框 -->
            <el-form-item label="选择用户">
                <el-select
                    v-model="selectedUserUid"
                    placeholder="请选择要配置的用户"
                    @change="handleUserChange"
                    class="user-select"
                >
                    <el-option
                        v-for="user in loginUsers"
                        :key="user.uid"
                        :label="user.username"
                        :value="user.uid"
                    >
                        <div class="user-option">
                            <el-avatar :src="`data:image/jpeg;base64,${user.avatar}`" :size="20">
                                {{ user.username.charAt(0) }}
                            </el-avatar>
                            <span class="user-option-name">{{ user.username }}</span>
                            <span class="user-option-uid">UID: {{ user.uid }}</span>
                        </div>
                    </el-option>
                </el-select>
                <div class="form-tip">选择要修改配置的用户</div>
            </el-form-item>

            <!-- 用户配置内容 -->
            <div v-if="selectedUser" class="user-config-section">
                <el-form :model="userConfigForm" label-width="140px" v-loading="userConfigLoading">
                    <!-- 线路选择 -->
                    <el-form-item label="上传线路">
                        <el-select v-model="userConfigForm.line" placeholder="请选择上传线路">
                            <el-option label="自动选择" value="auto" />
                            <el-option label="BDA2" value="bda2" />
                            <el-option label="WS" value="ws" />
                            <el-option label="QN" value="qn" />
                            <el-option label="BLDSA" value="bldsa" />
                            <el-option label="TX" value="tx" />
                            <el-option label="TXA" value="txa" />
                            <el-option label="BDA" value="bda" />
                            <el-option label="ALIA" value="alia" />
                        </el-select>
                        <div class="form-tip">自动选择将根据网络环境自动选择最优线路</div>
                    </el-form-item>

                    <!-- 代理设置 -->
                    <el-form-item label="代理设置">
                        <el-checkbox v-model="userProxyForm.enabled" class="proxy-checkbox">
                            启用代理
                        </el-checkbox>

                        <div v-show="userProxyForm.enabled" class="proxy-config">
                            <el-form-item label="代理类型" style="margin-top: 10px">
                                <el-select
                                    v-model="userProxyForm.type"
                                    placeholder="选择代理类型"
                                    class="proxy-type-select"
                                >
                                    <el-option label="HTTP" value="http" />
                                    <el-option label="HTTPS" value="https" />
                                    <el-option label="SOCKS5" value="socks5" />
                                </el-select>
                            </el-form-item>

                            <el-form-item label="服务器地址">
                                <el-input
                                    v-model="userProxyForm.host"
                                    placeholder="127.0.0.1"
                                    class="proxy-input"
                                />
                            </el-form-item>

                            <el-form-item label="端口">
                                <el-input-number
                                    v-model="userProxyForm.port"
                                    :min="1"
                                    :max="65535"
                                    placeholder="8080"
                                    class="proxy-port"
                                />
                            </el-form-item>

                            <el-form-item label="用户名">
                                <el-input
                                    v-model="userProxyForm.username"
                                    placeholder="用户名（可选）"
                                    class="proxy-input"
                                />
                            </el-form-item>

                            <el-form-item label="密码">
                                <el-input
                                    v-model="userProxyForm.password"
                                    type="password"
                                    placeholder="密码（可选）"
                                    class="proxy-input"
                                    show-password
                                />
                            </el-form-item>
                        </div>
                        <div class="form-tip">配置代理服务器用于网络访问</div>
                    </el-form-item>

                    <!-- 限流设置 -->
                    <el-form-item label="单视频并发数">
                        <div class="rate-limit-container">
                            <el-slider
                                v-model="userConfigForm.limit"
                                :min="1"
                                :max="12"
                                :step="1"
                                show-stops
                                show-input
                                :input-size="'small'"
                            />
                        </div>
                    </el-form-item>

                    <!-- 水印设置 -->
                    <el-form-item label="默认开启水印">
                        <el-checkbox
                            v-model="userConfigForm.watermark"
                            :true-value="1"
                            :false-value="0"
                        >
                            开启
                        </el-checkbox>
                    </el-form-item>
                </el-form>
            </div>
        </el-form>

        <template #footer>
            <span class="dialog-footer">
                <el-button @click="handleCancel">取消</el-button>
                <el-button type="primary" @click="handleSave" :loading="saving"> 保存 </el-button>
            </span>
        </template>
    </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { ElMessageBox } from 'element-plus'
import { useUserConfigStore } from '../stores/user_config'
import { useAuthStore } from '../stores/auth'
import { useUtilsStore } from '../stores/utils'

// 全局配置表单接口
interface GlobalConfigForm {
    max_curr: number
    auto_upload: boolean
    auto_start: boolean
}

// Props
const props = defineProps<{
    modelValue: boolean
}>()

// Emits
const emit = defineEmits<{
    'update:modelValue': [value: boolean]
    'config-updated': []
}>()

// Store
const userConfigStore = useUserConfigStore()
const authStore = useAuthStore()
const utilsStore = useUtilsStore()

// 响应式数据
const visible = ref(false)
const loading = ref(false)
const saving = ref(false)

// 用户配置相关
const selectedUserUid = ref<number | null>(null)
const userConfigLoading = ref(false)

const userConfigForm = ref({
    line: 'auto',
    limit: 0,
    watermark: 0
})

const userProxyForm = ref({
    enabled: false,
    type: 'http',
    host: '',
    port: 8080,
    username: '',
    password: ''
})

// 计算属性
const loginUsers = computed(() => authStore.loginUsers)
const selectedUser = computed(
    () => loginUsers.value.find(user => user.uid === selectedUserUid.value) || null
)

const configForm = ref<GlobalConfigForm>({
    max_curr: 1,
    auto_upload: true,
    auto_start: true
})

// 保存原始配置用于检查变化
const originalConfig = ref<GlobalConfigForm>({
    max_curr: 1,
    auto_upload: true,
    auto_start: true
})

// 监听 modelValue 变化
watch(
    () => props.modelValue,
    newValue => {
        visible.value = newValue
        if (newValue) {
            loadGlobalConfig()
            // 当对话框打开时，如果有用户且没有选择用户，默认选择第一个
            if (loginUsers.value.length > 0 && !selectedUserUid.value) {
                selectedUserUid.value = loginUsers.value[0].uid
                loadUserConfig(loginUsers.value[0].uid)
            }
        }
    },
    { immediate: true }
)

// 监听 visible 变化
watch(visible, newValue => {
    emit('update:modelValue', newValue)
})

// 监听登录用户变化，自动选择第一个用户
watch(
    () => loginUsers.value,
    newUsers => {
        // 如果当前没有选择用户，或者选择的用户不在新的用户列表中，选择第一个用户
        if (visible.value && newUsers.length > 0) {
            const currentUserExists = newUsers.some(user => user.uid === selectedUserUid.value)
            if (!selectedUserUid.value || !currentUserExists) {
                selectedUserUid.value = newUsers[0].uid
                loadUserConfig(newUsers[0].uid)
            }
        } else if (newUsers.length === 0) {
            // 如果没有用户了，清空选择
            selectedUserUid.value = null
        }
    },
    { immediate: true, deep: true }
)

// 加载全局配置
const loadGlobalConfig = async () => {
    loading.value = true
    try {
        // 确保配置已加载
        if (!userConfigStore.configRoot) {
            await userConfigStore.loadConfig()
        }

        if (userConfigStore.configRoot) {
            const config = userConfigStore.configRoot
            configForm.value = {
                max_curr: config.max_curr || 2,
                auto_upload: config.auto_upload ?? true,
                auto_start: config.auto_start ?? true
            }

            // 保存原始配置
            originalConfig.value = { ...configForm.value }
        }
    } catch (error) {
        console.error('加载全局配置失败:', error)
        utilsStore.showMessage(`加载全局配置失败: ${error}`, 'error')
    } finally {
        loading.value = false
    }
}

// 保存配置
const handleSave = async () => {
    saving.value = true
    try {
        // 确保配置根对象存在
        if (!userConfigStore.configRoot) {
            await userConfigStore.loadConfig()
        }

        if (!userConfigStore.configRoot) {
            throw new Error('无法加载配置')
        }

        // 更新全局配置
        await userConfigStore.updateGlobalConfig({
            max_curr: configForm.value.max_curr,
            auto_upload: configForm.value.auto_upload,
            auto_start: configForm.value.auto_start
        })

        // 如果选择了用户，保存用户配置
        if (selectedUserUid.value) {
            // 构建代理URL
            let proxyUrl = ''
            if (userProxyForm.value.enabled && userProxyForm.value.host) {
                const auth =
                    userProxyForm.value.username && userProxyForm.value.password
                        ? `${userProxyForm.value.username}:${userProxyForm.value.password}@`
                        : ''
                proxyUrl = `${userProxyForm.value.type}://${auth}${userProxyForm.value.host}:${userProxyForm.value.port}`
            }

            await userConfigStore.updateUserConfig(selectedUserUid.value, {
                line: userConfigForm.value.line === 'auto' ? undefined : userConfigForm.value.line,
                proxy: proxyUrl || undefined,
                limit: userConfigForm.value.limit,
                watermark: userConfigForm.value.watermark
            })
        }

        utilsStore.showMessage('配置保存成功', 'success')
        emit('config-updated')
        visible.value = false
    } catch (error) {
        console.error('保存配置失败:', error)
        utilsStore.showMessage(`保存配置失败: ${error}`, 'error')
    } finally {
        saving.value = false
    }
}

// 取消操作
const handleCancel = () => {
    visible.value = false
}

// 用户选择变化处理
const handleUserChange = async (uid: number) => {
    selectedUserUid.value = uid
    await loadUserConfig(uid)
}

// 加载用户配置
const loadUserConfig = async (uid: number) => {
    userConfigLoading.value = true
    try {
        // 确保配置已加载
        if (!userConfigStore.configRoot) {
            await userConfigStore.loadConfig()
        }

        const userConfig = userConfigStore.configRoot?.config[uid]
        if (userConfig) {
            userConfigForm.value = {
                line: userConfig.line || 'auto',
                limit: userConfig.limit || 0,
                watermark: userConfig.watermark || 0
            }

            // 解析代理设置
            const proxyUrl = userConfig.proxy || ''
            if (proxyUrl) {
                try {
                    const url = new URL(proxyUrl)
                    userProxyForm.value = {
                        enabled: true,
                        type: url.protocol.replace(':', ''),
                        host: url.hostname,
                        port: parseInt(url.port) || 8080,
                        username: url.username || '',
                        password: url.password || ''
                    }
                } catch {
                    // 解析失败，使用默认值
                    userProxyForm.value = {
                        enabled: false,
                        type: 'http',
                        host: '',
                        port: 8080,
                        username: '',
                        password: ''
                    }
                }
            } else {
                userProxyForm.value = {
                    enabled: false,
                    type: 'http',
                    host: '',
                    port: 8080,
                    username: '',
                    password: ''
                }
            }
        } else {
            // 用户配置不存在时，使用默认值
            userConfigForm.value = {
                line: 'auto',
                limit: 0,
                watermark: 0
            }

            userProxyForm.value = {
                enabled: false,
                type: 'http',
                host: '',
                port: 8080,
                username: '',
                password: ''
            }
        }
    } catch (error) {
        console.error('加载用户配置失败:', error)
        utilsStore.showMessage(`加载用户配置失败: ${error}`, 'error')
    } finally {
        userConfigLoading.value = false
    }
}

// 对话框关闭处理
const handleClose = (done: () => void) => {
    // 检查是否有未保存的更改
    if (hasUnsavedChanges()) {
        ElMessageBox.confirm('有未保存的更改，确定要关闭吗？', '确认', {
            confirmButtonText: '确定',
            cancelButtonText: '取消',
            type: 'warning'
        })
            .then(() => {
                done()
            })
            .catch(() => {
                // 用户取消关闭
            })
    } else {
        done()
    }
}

// 对话框关闭后的处理
const handleDialogClose = () => {
    // 重置表单为默认值
    configForm.value = {
        max_curr: 1,
        auto_upload: true,
        auto_start: true
    }
    originalConfig.value = { ...configForm.value }
}

// 检查是否有未保存的更改
const hasUnsavedChanges = (): boolean => {
    return (
        configForm.value.max_curr !== originalConfig.value.max_curr ||
        configForm.value.auto_upload !== originalConfig.value.auto_upload ||
        configForm.value.auto_start !== originalConfig.value.auto_start
    )
}
</script>

<style scoped>
.form-tip {
    font-size: 12px;
    color: #909399;
    margin-top: 5px;
    line-height: 1.4;
}

.slider-container {
    width: 100%;
    margin-bottom: 10px;
}

.rate-limit-container {
    width: 100%;
    margin-bottom: 10px;
}

.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
}

.el-switch {
    margin-right: 10px;
}

/* 用户配置相关样式 */
.user-select {
    width: 100%;
}

.user-option {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
}

.user-option-name {
    flex: 1;
    font-size: 14px;
}

.user-option-uid {
    font-size: 12px;
    color: #909399;
}

.user-config-section {
    border-top: 1px solid #e1e6ea;
    margin-top: 20px;
    padding-top: 20px;
}

.proxy-checkbox {
    margin-bottom: 10px;
}

.proxy-config {
    background: #f8f9fa;
    border-radius: 6px;
    padding: 15px;
    margin-top: 10px;
}

.proxy-type-select {
    width: 100%;
}

.proxy-input {
    width: 100%;
}

.proxy-port {
    width: 100%;
}

:deep(.el-form-item__label) {
    font-weight: 500;
}

:deep(.el-slider) {
    margin: 0 12px;
}

:deep(.el-divider__text) {
    font-weight: 600;
}

:deep(.el-divider__text .el-text) {
    font-size: 16px;
}
</style>
