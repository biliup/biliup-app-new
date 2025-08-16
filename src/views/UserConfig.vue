<template>
    <el-dialog
        v-model="visible"
        :title="`${user?.username} 的配置`"
        width="500px"
        :before-close="handleClose"
        @close="handleDialogClose"
    >
        <el-form :model="configForm" label-width="120px" v-loading="loading">
            <!-- 线路选择 -->
            <el-form-item label="上传线路">
                <el-select v-model="configForm.line" placeholder="请选择上传线路">
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
                <el-checkbox v-model="proxyForm.enabled" class="proxy-checkbox">
                    启用代理
                </el-checkbox>

                <div v-show="proxyForm.enabled" class="proxy-config">
                    <el-form-item label="代理类型" style="margin-top: 10px">
                        <el-select
                            v-model="proxyForm.type"
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
                            v-model="proxyForm.host"
                            placeholder="127.0.0.1"
                            class="proxy-input"
                        />
                    </el-form-item>

                    <el-form-item label="端口">
                        <el-input
                            v-model="proxyForm.port"
                            type="number"
                            placeholder="1080"
                            class="proxy-input"
                        />
                    </el-form-item>

                    <el-form-item>
                        <el-checkbox v-model="proxyForm.needAuth" class="proxy-auth-checkbox">
                            需要身份验证
                        </el-checkbox>
                    </el-form-item>

                    <div v-show="proxyForm.needAuth" class="proxy-auth">
                        <el-form-item label="用户名">
                            <el-input
                                v-model="proxyForm.username"
                                placeholder="用户名"
                                class="proxy-input"
                            />
                        </el-form-item>

                        <el-form-item label="密码">
                            <el-input
                                v-model="proxyForm.password"
                                type="password"
                                placeholder="密码"
                                show-password
                                class="proxy-input"
                            />
                        </el-form-item>
                    </div>
                </div>

                <div class="form-tip">修改代理设置需要重启应用后生效</div>
            </el-form-item>

            <!-- 并发数设置 -->
            <el-form-item label="单视频并发数">
                <div class="slider-container">
                    <el-slider
                        v-model="configForm.limit"
                        :min="1"
                        :max="12"
                        :step="1"
                        show-stops
                        show-input
                        :input-size="'small'"
                    />
                </div>
            </el-form-item>
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
import { ref, watch, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useUserConfigStore } from '../stores/user_config'

// 接口定义
interface User {
    uid: number
    username: string
    avatar: string
}

interface UserConfigForm {
    line: string
    proxy: string
    limit: number
}

// 代理表单接口
interface ProxyForm {
    enabled: boolean
    type: string
    host: string
    port: string
    needAuth: boolean
    username: string
    password: string
}

// Props
const props = defineProps<{
    modelValue: boolean
    user: User | null
}>()

// Emits
const emit = defineEmits<{
    'update:modelValue': [value: boolean]
    'config-updated': []
}>()

// Store
const userConfigStore = useUserConfigStore()

// 响应式数据
const visible = ref(false)
const loading = ref(false)
const saving = ref(false)
const originalProxy = ref('')

const configForm = ref<UserConfigForm>({
    line: 'auto',
    proxy: '',
    limit: 3
})

// 代理设置表单
const proxyForm = ref<ProxyForm>({
    enabled: false,
    type: 'http',
    host: '',
    port: '',
    needAuth: false,
    username: '',
    password: ''
})

// 监听 modelValue 变化
watch(
    () => props.modelValue,
    newValue => {
        visible.value = newValue
        if (newValue && props.user) {
            loadUserConfig()
        }
    },
    { immediate: true }
)

// 监听 visible 变化
watch(visible, newValue => {
    emit('update:modelValue', newValue)
})

// 构建代理 URL
const buildProxyUrl = () => {
    if (!proxyForm.value.enabled || !proxyForm.value.host || !proxyForm.value.port) {
        return ''
    }

    let proxyUrl = `${proxyForm.value.type}://`

    // 如果需要身份验证且有用户名密码
    if (proxyForm.value.needAuth && proxyForm.value.username && proxyForm.value.password) {
        if (proxyForm.value.username) {
            proxyUrl += `${encodeURIComponent(proxyForm.value.username)}`
            if (proxyForm.value.password) {
                proxyUrl += `:${encodeURIComponent(proxyForm.value.password)}@`
            } else {
                proxyUrl += '@'
            }
        }
    }

    proxyUrl += `${proxyForm.value.host}:${proxyForm.value.port}`

    return proxyUrl
}

// 解析代理 URL
const parseProxyUrl = (proxyUrl: string) => {
    if (!proxyUrl) {
        proxyForm.value = {
            enabled: false,
            type: 'http',
            host: '',
            port: '',
            needAuth: false,
            username: '',
            password: ''
        }
        return
    }

    try {
        const url = new URL(proxyUrl)
        proxyForm.value = {
            enabled: true,
            type: url.protocol.replace(':', ''),
            host: url.hostname,
            port: url.port || (url.protocol === 'https:' ? '443' : '80'),
            needAuth: !!url.username && !!url.password,
            username: decodeURIComponent(url.username || ''),
            password: decodeURIComponent(url.password || '')
        }
    } catch (error) {
        // 如果解析失败，使用默认值
        proxyForm.value = {
            enabled: false,
            type: 'http',
            host: '',
            port: '',
            needAuth: false,
            username: '',
            password: ''
        }
    }
}

// 加载用户配置
const loadUserConfig = async () => {
    if (!props.user) return

    loading.value = true
    try {
        // 确保配置已加载
        if (!userConfigStore.configRoot) {
            await userConfigStore.loadConfig()
        }

        const userConfig = userConfigStore.configRoot?.config[props.user.uid]
        if (userConfig) {
            configForm.value = {
                line: userConfig.line || 'auto',
                proxy: userConfig.proxy || '',
                limit: userConfig.limit || 3
            }
            originalProxy.value = userConfig.proxy || ''

            // 解析代理设置
            parseProxyUrl(userConfig.proxy || '')
        } else {
            // 如果用户配置不存在，使用默认值
            configForm.value = {
                line: 'auto',
                proxy: '',
                limit: 3
            }
            originalProxy.value = ''
            parseProxyUrl('')
        }
    } catch (error) {
        console.error('加载用户配置失败:', error)
        ElMessage.error(`加载用户配置失败: ${error}`)
    } finally {
        loading.value = false
    }
}

// 保存配置
const handleSave = async () => {
    if (!props.user) return

    saving.value = true
    try {
        // 构建代理 URL
        const proxyUrl = buildProxyUrl()

        // 检查代理设置是否发生变化
        const proxyChanged = originalProxy.value !== proxyUrl

        // 确保配置根对象存在
        if (!userConfigStore.configRoot) {
            await userConfigStore.loadConfig()
        }

        if (!userConfigStore.configRoot) {
            throw new Error('无法加载配置')
        }

        // 更新用户配置
        const userConfig = userConfigStore.configRoot.config[props.user.uid]
        if (!userConfig) {
            throw new Error('用户配置不存在')
        }

        // 处理线路设置，如果是 auto 则设为 null
        userConfig.line = configForm.value.line === 'auto' ? undefined : configForm.value.line
        userConfig.proxy = proxyUrl || undefined
        userConfig.limit = configForm.value.limit

        // 保存配置
        await userConfigStore.updateUserConfig(props.user.uid, userConfig)

        ElMessage.success('配置保存成功')

        // 如果代理设置有变化，提示重启
        if (proxyChanged) {
            await nextTick()
            ElMessageBox.alert('代理设置已更改，需要重启应用后生效', '重启提示', {
                confirmButtonText: '知道了',
                type: 'info'
            })
        }

        emit('config-updated')
        visible.value = false
    } catch (error) {
        console.error('保存配置失败:', error)
        ElMessage.error(`保存配置失败: ${error}`)
    } finally {
        saving.value = false
    }
}

// 取消操作
const handleCancel = () => {
    visible.value = false
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
    // 重置表单
    configForm.value = {
        line: 'auto',
        proxy: '',
        limit: 3
    }
    proxyForm.value = {
        enabled: false,
        type: 'http',
        host: '',
        port: '',
        needAuth: false,
        username: '',
        password: ''
    }
    originalProxy.value = ''
}

// 检查是否有未保存的更改
const hasUnsavedChanges = (): boolean => {
    if (!props.user || !userConfigStore.configRoot) return false

    const userConfig = userConfigStore.configRoot.config[props.user.uid]
    if (!userConfig) return true

    const currentLine = userConfig.line || 'auto'
    const currentProxy = userConfig.proxy || ''
    const currentLimit = userConfig.limit || 3

    return (
        configForm.value.line !== currentLine ||
        configForm.value.proxy !== currentProxy ||
        configForm.value.limit !== currentLimit
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
}

.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
}

.el-select {
    width: 100%;
}

:deep(.el-input-group__prepend) {
    background-color: #fafafa;
    border-color: #dcdfe6;
}

.proxy-checkbox {
    margin-bottom: 10px;
}

.proxy-config {
    margin-left: 20px;
    padding: 10px;
    border-left: 2px solid #e4e7ed;
}

.proxy-type-select {
    width: 100%;
}

.proxy-input {
    width: 100%;
}

.proxy-auth-checkbox {
    margin: 10px 0;
}

.proxy-auth {
    margin-left: 20px;
    padding: 10px;
    border-left: 2px solid #f0f0f0;
}
</style>
