<template>
    <div class="login-view">
        <div class="login-container">
            <div class="login-card">
                <div class="login-content">
                    <el-tabs v-model="activeTab" class="login-tabs">
                        <!-- 二维码登录 -->
                        <el-tab-pane label="二维码登录" name="qr">
                            <div class="qr-login">
                                <div v-if="qrCode" class="qr-container">
                                    <img :src="qrCode" alt="登录二维码" class="qr-image" />
                                    <p class="qr-tip">请使用哔哩哔哩APP扫描二维码登录</p>
                                </div>
                                <el-button
                                    v-else
                                    type="primary"
                                    @click="getQRCode"
                                    :loading="loading"
                                >
                                    获取二维码
                                </el-button>
                            </div>
                        </el-tab-pane>

                        <!-- 用户名密码登录 -->
                        <el-tab-pane label="密码登录" name="password">
                            <div class="password-login">
                                <el-form
                                    :model="passwordForm"
                                    ref="passwordFormRef"
                                    :rules="passwordRules"
                                >
                                    <el-form-item prop="username">
                                        <el-input
                                            v-model="passwordForm.username"
                                            placeholder="手机号/邮箱/用户名"
                                            prefix-icon="User"
                                        />
                                    </el-form-item>
                                    <el-form-item prop="password">
                                        <el-input
                                            v-model="passwordForm.password"
                                            type="password"
                                            placeholder="密码"
                                            prefix-icon="Lock"
                                            show-password
                                        />
                                    </el-form-item>
                                </el-form>
                                <el-button
                                    type="primary"
                                    @click="loginWithPassword"
                                    :loading="loading"
                                    class="login-btn"
                                >
                                    登录
                                </el-button>
                            </div>
                        </el-tab-pane>

                        <!-- 短信登录 -->
                        <el-tab-pane label="短信登录" name="sms">
                            <div class="sms-login">
                                <el-form :model="smsForm" ref="smsFormRef" :rules="smsRules">
                                    <el-form-item prop="phone">
                                        <div class="phone-input-container">
                                            <el-select
                                                v-model="smsForm.countryCode"
                                                placeholder="国家"
                                                class="country-select"
                                            >
                                                <el-option
                                                    v-for="country in countryList"
                                                    :key="country.code"
                                                    :label="`+${country.code} ${country.name}`"
                                                    :value="country.code"
                                                >
                                                    <span class="country-option">
                                                        {{ country.flag }} +{{ country.code }}
                                                        {{ country.name }}
                                                    </span>
                                                </el-option>
                                            </el-select>
                                            <el-input
                                                v-model="smsForm.phone"
                                                placeholder="请输入手机号"
                                                prefix-icon="Phone"
                                                class="phone-input"
                                            />
                                        </div>
                                    </el-form-item>
                                    <el-form-item prop="code">
                                        <div class="sms-code-container">
                                            <el-input
                                                v-model="smsForm.code"
                                                placeholder="请输入验证码"
                                                prefix-icon="Message"
                                                class="sms-code-input"
                                            />
                                            <el-button
                                                @click="sendSMSCode"
                                                :disabled="smsCountdown > 0"
                                                :loading="sendingCode"
                                                class="sms-send-btn"
                                            >
                                                {{
                                                    smsCountdown > 0
                                                        ? `${smsCountdown}s`
                                                        : '发送验证码'
                                                }}
                                            </el-button>
                                        </div>
                                    </el-form-item>
                                </el-form>
                                <el-button
                                    type="primary"
                                    @click="loginWithSMS"
                                    :loading="loading"
                                    class="login-btn"
                                >
                                    登录
                                </el-button>
                            </div>
                        </el-tab-pane>

                        <!-- Cookie登录 -->
                        <el-tab-pane label="Cookie登录" name="cookie">
                            <div class="cookie-login">
                                <el-input
                                    v-model="cookieValue"
                                    type="textarea"
                                    placeholder="请输入完整的Cookie..."
                                    :rows="6"
                                    class="cookie-input"
                                />
                                <el-button
                                    type="primary"
                                    @click="loginWithCookie"
                                    :loading="loading"
                                    class="login-btn"
                                >
                                    登录
                                </el-button>
                            </div>
                        </el-tab-pane>
                    </el-tabs>

                    <!-- 高级登录选项 -->
                    <div class="advanced-options">
                        <div
                            class="advanced-toggle"
                            @click="showAdvancedOptions = !showAdvancedOptions"
                        >
                            <span class="advanced-toggle-text">高级选项</span>
                            <el-icon
                                class="advanced-toggle-icon"
                                :class="{ 'is-expanded': showAdvancedOptions }"
                            >
                                <ArrowDown />
                            </el-icon>
                        </div>

                        <el-collapse-transition>
                            <div v-show="showAdvancedOptions" class="advanced-content">
                                <div class="proxy-settings">
                                    <h4 class="proxy-title">代理服务器设置</h4>
                                    <el-form :model="proxyForm" class="proxy-form">
                                        <el-form-item>
                                            <el-checkbox
                                                v-model="proxyForm.enabled"
                                                class="proxy-checkbox"
                                            >
                                                启用代理
                                            </el-checkbox>
                                        </el-form-item>

                                        <div v-show="proxyForm.enabled" class="proxy-config">
                                            <el-form-item label="代理类型">
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
                                                <el-checkbox
                                                    v-model="proxyForm.needAuth"
                                                    class="proxy-auth-checkbox"
                                                >
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
                                    </el-form>
                                </div>
                            </div>
                        </el-collapse-transition>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAuthStore } from '../stores/auth'
import { ElMessage } from 'element-plus'
import { ArrowDown } from '@element-plus/icons-vue'

// 定义emit事件
const emit = defineEmits<{
    'login-success': []
    'loading-change': [loading: boolean]
}>()

const authStore = useAuthStore()

const activeTab = ref('qr')
const qrCode = ref('')
const cookieValue = ref('')
const loading = ref(false)

// 高级选项
const showAdvancedOptions = ref(false)

// 代理设置表单
const proxyForm = ref({
    enabled: false,
    type: 'http',
    host: '',
    port: '',
    needAuth: false,
    username: '',
    password: ''
})

// 密码登录表单
const passwordForm = ref({
    username: '',
    password: ''
})
const passwordFormRef = ref()

// 短信登录表单
const smsForm = ref({
    phone: '',
    countryCode: '86',
    code: ''
})
const smsFormRef = ref()
const smsCountdown = ref(0)
const sendingCode = ref(false)

// 国家代码列表
const countryList = ref([
    { code: '86', name: '中国', flag: '🇨🇳' },
    { code: '1', name: '美国', flag: '🇺🇸' },
    { code: '44', name: '英国', flag: '🇬🇧' },
    { code: '81', name: '日本', flag: '🇯🇵' },
    { code: '82', name: '韩国', flag: '🇰🇷' },
    { code: '65', name: '新加坡', flag: '🇸🇬' },
    { code: '852', name: '香港', flag: '🇭🇰' },
    { code: '853', name: '澳门', flag: '🇲🇴' },
    { code: '886', name: '台湾', flag: '🇹🇼' },
    { code: '61', name: '澳大利亚', flag: '🇦🇺' },
    { code: '33', name: '法国', flag: '🇫🇷' },
    { code: '49', name: '德国', flag: '🇩🇪' },
    { code: '39', name: '意大利', flag: '🇮🇹' },
    { code: '7', name: '俄罗斯', flag: '🇷🇺' },
    { code: '91', name: '印度', flag: '🇮🇳' }
])

// 表单验证规则
const passwordRules = {
    username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
    password: [{ required: true, message: '请输入密码', trigger: 'blur' }]
}

const smsRules = {
    phone: [{ required: true, message: '请输入手机号', trigger: 'blur' }],
    code: [{ required: true, message: '请输入验证码', trigger: 'blur' }]
}

// 获取二维码
const getQRCode = async () => {
    loading.value = true
    emit('loading-change', true)
    try {
        const proxyUrl = buildProxyUrl()
        const response = (await authStore.getLoginQR(proxyUrl)) as any
        qrCode.value = response

        // 检查登录状态
        checkQRLogin()
    } catch (error) {
        ElMessage.error('获取二维码失败')
    } finally {
        loading.value = false
        emit('loading-change', false)
    }
}

// 检查二维码登录状态
const checkQRLogin = async () => {
    if (!qrCode.value) return
    try {
        const response = await authStore.checkQRLogin()
        if (response.success) {
            ElMessage.success('登录成功！')
            emit('login-success')
        } else {
            ElMessage.error(response.message || '登录失败')
        }
    } catch (error) {
        console.error('检查登录状态失败:', error)
    }
}

// Cookie登录
const loginWithCookie = async () => {
    if (!cookieValue.value.trim()) {
        ElMessage.warning('请输入Cookie')
        return
    }

    loading.value = true
    emit('loading-change', true)
    try {
        const proxyUrl = buildProxyUrl()
        const response = await authStore.loginWithCookie(cookieValue.value, proxyUrl)
        if (response.success) {
            ElMessage.success('登录成功！')
            emit('login-success')
        } else {
            ElMessage.error(response.message || '登录失败')
        }
    } catch (error) {
        ElMessage.error('登录失败')
    } finally {
        loading.value = false
        emit('loading-change', false)
    }
}

// 密码登录
const loginWithPassword = async () => {
    try {
        await passwordFormRef.value.validate()
    } catch {
        return
    }

    loading.value = true
    emit('loading-change', true)
    try {
        const proxyUrl = buildProxyUrl()
        const response = await authStore.loginWithPassword(
            passwordForm.value.username,
            passwordForm.value.password,
            proxyUrl
        )
        if (response.success) {
            ElMessage.success('登录成功！')
            emit('login-success')
        } else {
            ElMessage.error(response.message || '登录失败')
        }
    } catch (error) {
        ElMessage.error('登录失败')
    } finally {
        loading.value = false
        emit('loading-change', false)
    }
}

// 发送短信验证码
const sendSMSCode = async () => {
    try {
        await smsFormRef.value.validateField('phone')
    } catch {
        return
    }

    sendingCode.value = true
    emit('loading-change', true)
    try {
        const proxyUrl = buildProxyUrl()
        const response = (await authStore.sendSMSCode(
            smsForm.value.phone,
            smsForm.value.countryCode,
            proxyUrl
        )) as any
        if (response.success) {
            ElMessage.success('验证码已发送')
            // 开始倒计时
            smsCountdown.value = 60
            const timer = setInterval(() => {
                smsCountdown.value--
                if (smsCountdown.value <= 0) {
                    clearInterval(timer)
                }
            }, 1000)
        } else {
            ElMessage.error(response.message || '发送失败')
        }
    } catch (error) {
        ElMessage.error('发送验证码失败')
    } finally {
        sendingCode.value = false
        emit('loading-change', false)
    }
}

// 短信登录
const loginWithSMS = async () => {
    try {
        await smsFormRef.value.validate()
    } catch {
        return
    }

    loading.value = true
    emit('loading-change', true)
    try {
        const proxyUrl = buildProxyUrl()
        const response = await authStore.loginWithSMS(
            smsForm.value.phone,
            smsForm.value.countryCode,
            smsForm.value.code,
            proxyUrl
        )
        if (response.success) {
            ElMessage.success('登录成功！')
            emit('login-success')
        } else {
            ElMessage.error(response.message || '登录失败')
        }
    } catch (error) {
        ElMessage.error('登录失败')
    } finally {
        loading.value = false
        emit('loading-change', false)
    }
}

// 构建代理URL
const buildProxyUrl = () => {
    if (!proxyForm.value.enabled || !proxyForm.value.host || !proxyForm.value.port) {
        return undefined
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

onMounted(() => {})
</script>

<style scoped>
.login-view {
    min-height: 100vh;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
}

.login-container {
    width: 100%;
    max-width: 480px;
}

.login-card {
    background: rgba(255, 255, 255, 0.95);
    backdrop-filter: blur(20px);
    border-radius: 24px;
    padding: 40px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
}

.card-header {
    text-align: center;
    margin-bottom: 40px;
}

.login-content {
    width: 100%;
}

.login-tabs {
    width: 100%;
}

.login-tabs :deep(.el-tabs__header) {
    margin: 0 0 30px 0;
}

.login-tabs :deep(.el-tabs__nav-wrap) {
    background: #f8fafc;
    border-radius: 12px;
    padding: 4px;
}

.login-tabs :deep(.el-tabs__active-bar) {
    display: none;
}

.login-tabs :deep(.el-tabs__item) {
    border-radius: 8px;
    transition: all 0.3s;
    color: #64748b;
    font-weight: 500;
    border: none;
}

.login-tabs :deep(.el-tabs__item.is-active) {
    background: white;
    color: #667eea;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.tab-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;
}

.login-form {
    width: 100%;
}

.form-input {
    margin-bottom: 16px;
}

.form-input :deep(.el-input__wrapper) {
    border-radius: 12px;
    padding: 0 16px;
    border: 2px solid #e2e8f0;
    transition: all 0.3s;
}

.form-input :deep(.el-input__wrapper:hover),
.form-input :deep(.el-input__wrapper.is-focus) {
    border-color: #667eea;
    box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
}

.remember-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px !important;
}

.remember-checkbox {
    color: #64748b;
}

.forgot-link {
    text-decoration: none;
    font-size: 14px;
}

.login-btn {
    width: 100%;
    height: 48px;
    border-radius: 12px;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    border: none;
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 24px;
}

.login-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 25px rgba(102, 126, 234, 0.3);
}

/* 二维码样式 */
.qr-login {
    text-align: center;
    padding: 20px 0;
}

.qr-container {
    display: flex;
    flex-direction: column;
    align-items: center;
}

.qr-wrapper {
    background: white;
    padding: 20px;
    border-radius: 16px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
    margin-bottom: 16px;
}

.qr-image {
    width: 180px;
    height: 180px;
    border-radius: 8px;
}

.qr-tip {
    color: #64748b;
    font-size: 14px;
    margin-bottom: 12px;
}

.refresh-qr {
    color: #667eea;
    font-size: 14px;
}

.qr-placeholder {
    padding: 60px 20px;
    color: #94a3b8;
}

.qr-icon {
    font-size: 48px;
    margin-bottom: 16px;
}

.qr-placeholder-text {
    margin-bottom: 20px;
    font-size: 16px;
}

.get-qr-btn {
    padding: 12px 24px;
}

/* 短信登录样式 */
.sms-login {
    padding: 20px 0;
}

.phone-input-container {
    display: flex;
    gap: 12px;
}

.country-select {
    width: 120px;
}

.country-select :deep(.el-select__wrapper) {
    border-radius: 12px;
    border: 2px solid #e2e8f0;
}

.phone-input {
    flex: 1;
}

.country-option {
    display: flex;
    align-items: center;
    gap: 8px;
}

.sms-code-container {
    display: flex;
    gap: 12px;
}

.sms-code-input {
    flex: 1;
}

.sms-send-btn {
    min-width: 120px;
    height: 48px;
    border-radius: 12px;
    border: 2px solid #667eea;
    color: #667eea;
    background: white;
}

.sms-send-btn:hover {
    background: #667eea;
    color: white;
}

/* Cookie登录样式 */
.cookie-login {
    padding: 20px 0;
}

.cookie-input {
    margin-bottom: 24px;
}

.cookie-input :deep(.el-textarea__inner) {
    border-radius: 12px;
    border: 2px solid #e2e8f0;
    padding: 16px;
}

.cookie-input :deep(.el-textarea__inner:hover),
.cookie-input :deep(.el-textarea__inner:focus) {
    border-color: #667eea;
}

/* 其他登录方式 */
.other-login {
    margin-top: 30px;
}

.divider {
    text-align: center;
    position: relative;
    margin: 20px 0;
}

.divider::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 0;
    right: 0;
    height: 1px;
    background: #e2e8f0;
}

.divider span {
    background: rgba(255, 255, 255, 0.95);
    padding: 0 16px;
    color: #64748b;
    font-size: 14px;
}

.quick-login-buttons {
    display: flex;
    gap: 12px;
    justify-content: center;
}

.quick-btn {
    flex: 1;
    height: 40px;
    border-radius: 8px;
    border: 1px solid #e2e8f0;
}

/* 用户切换样式 */
.user-switcher {
    margin-top: 30px;
}

.saved-users {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    justify-content: center;
}

.user-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 16px 12px;
    border: 2px solid #e2e8f0;
    border-radius: 12px;
    cursor: pointer;
    transition: all 0.3s;
    background: white;
    min-width: 80px;
}

.user-item:hover {
    border-color: #667eea;
    transform: translateY(-2px);
    box-shadow: 0 8px 25px rgba(102, 126, 234, 0.15);
}

.user-item .username {
    margin-top: 8px;
    font-size: 12px;
    color: #64748b;
    text-align: center;
}

/* 响应式设计 */
@media (max-width: 640px) {
    .login-card {
        padding: 30px 24px;
        margin: 20px;
        border-radius: 20px;
    }

    .app-title {
        font-size: 28px;
    }

    .quick-login-buttons {
        flex-direction: column;
    }

    .phone-input-container {
        flex-direction: column;
        gap: 16px;
    }

    .country-select {
        width: 100%;
    }
}

/* 高级选项样式 */
.advanced-options {
    margin-top: 20px;
    margin-bottom: 20px;
}

.advanced-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px;
    cursor: pointer;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    background: #f8fafc;
    transition: all 0.3s;
    user-select: none;
}

.advanced-toggle:hover {
    background: #f1f5f9;
    border-color: #cbd5e1;
}

.advanced-toggle-text {
    color: #64748b;
    font-size: 14px;
    font-weight: 500;
}

.advanced-toggle-icon {
    color: #64748b;
    transition: transform 0.3s;
}

.advanced-toggle-icon.is-expanded {
    transform: rotate(180deg);
}

.advanced-content {
    padding: 20px 0;
}

.proxy-settings {
    background: #f8fafc;
    border-radius: 12px;
    padding: 20px;
    border: 1px solid #e2e8f0;
}

.proxy-title {
    margin: 0 0 16px 0;
    color: #334155;
    font-size: 16px;
    font-weight: 600;
}

.proxy-form :deep(.el-form-item) {
    margin-bottom: 16px;
}

.proxy-form :deep(.el-form-item__label) {
    color: #475569;
    font-weight: 500;
    font-size: 14px;
}

.proxy-checkbox {
    color: #64748b;
    font-weight: 500;
}

.proxy-config {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid #e2e8f0;
}

.proxy-type-select,
.proxy-input {
    width: 100%;
}

.proxy-type-select :deep(.el-select__wrapper),
.proxy-input :deep(.el-input__wrapper) {
    border-radius: 8px;
    border: 1px solid #d1d5db;
    transition: all 0.3s;
}

.proxy-type-select :deep(.el-select__wrapper:hover),
.proxy-type-select :deep(.el-select__wrapper.is-focus),
.proxy-input :deep(.el-input__wrapper:hover),
.proxy-input :deep(.el-input__wrapper.is-focus) {
    border-color: #667eea;
    box-shadow: 0 0 0 2px rgba(102, 126, 234, 0.1);
}

.proxy-auth-checkbox {
    color: #64748b;
    font-weight: 500;
    margin-top: 8px;
}

.proxy-auth {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid #e2e8f0;
}
</style>
