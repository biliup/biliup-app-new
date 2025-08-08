import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface User {
    uid: number
    username: string
    avatar: string
}

interface LoginResponse {
    success: boolean
    message: string
}

export const useAuthStore = defineStore('auth', () => {
    const loginUsers = ref<User[]>([])
    const isLoggedIn = computed(() => loginUsers.value.length > 0)

    // 获取登录二维码
    const getLoginQR = async (proxy?: string) => {
        try {
            return await invoke('get_login_qr', { proxy })
        } catch (error) {
            console.error('获取二维码失败:', error)
            throw error
        }
    }

    // 检查二维码登录状态
    const checkQRLogin = async () => {
        try {
            const response: LoginResponse = await invoke('check_qr_login', {})
            if (response.success) {
                // 重新获取所有登录用户
                await getLoginUsers()
            }
            return response
        } catch (error) {
            console.error('检查二维码登录失败:', error)
            throw error
        }
    }

    // Cookie 登录
    const loginWithCookie = async (cookie: string, proxy?: string) => {
        try {
            const response: LoginResponse = await invoke('login_with_cookie', { cookie, proxy })
            if (response.success) {
                // 重新获取所有登录用户
                await getLoginUsers()
            }
            return response
        } catch (error) {
            console.error('Cookie登录失败:', error)
            throw error
        }
    }

    // 密码登录
    const loginWithPassword = async (username: string, password: string, proxy?: string) => {
        try {
            const response: LoginResponse = await invoke('login_with_password', {
                username,
                password,
                proxy
            })
            if (response.success) {
                // 重新获取所有登录用户
                await getLoginUsers()
            }
            return response
        } catch (error) {
            console.error('密码登录失败:', error)
            throw error
        }
    }

    // 发送短信验证码
    const sendSMSCode = async (phone: string, countryCode: string = '86', proxy?: string) => {
        try {
            const response = await invoke('send_sms_code', {
                phone,
                country_code: countryCode,
                proxy
            })
            return response
        } catch (error) {
            console.error('发送短信验证码失败:', error)
            throw error
        }
    }

    // 短信登录
    const loginWithSMS = async (
        phone: string,
        countryCode: string,
        code: string,
        proxy?: string
    ) => {
        try {
            const response: LoginResponse = await invoke('login_with_sms', {
                phone,
                country_code: countryCode,
                code,
                proxy
            })
            if (response.success) {
                // 重新获取所有登录用户
                await getLoginUsers()
            }
            return response
        } catch (error) {
            console.error('短信登录失败:', error)
            throw error
        }
    }

    // 获取所有已登录的用户
    const getLoginUsers = async () => {
        try {
            const users: User[] = await invoke('get_login_users')
            loginUsers.value = users
            return users
        } catch (error) {
            console.error('获取登录用户失败:', error)
            throw error
        }
    }

    const logoutUser = async (uid: number) => {
        try {
            const success = await invoke('logout_user', { uid })
            return success
        } catch (error) {
            console.error('登出用户失败:', error)
            throw error
        }
    }

    return {
        loginUsers,
        isLoggedIn,
        getLoginQR,
        checkQRLogin,
        loginWithCookie,
        loginWithPassword,
        sendSMSCode,
        loginWithSMS,
        getLoginUsers,
        logoutUser
    }
})
