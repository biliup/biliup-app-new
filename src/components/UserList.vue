<template>
    <div class="user-list-dropdown">
        <el-dropdown
            trigger="click"
            placement="bottom-end"
            @command="handleCommand"
            popper-class="user-list-popper"
        >
            <div class="user-trigger">
                <el-icon class="user-icon"><user /></el-icon>
                <span class="user-count" v-if="loginUsers.length > 0">
                    {{ loginUsers.length }} 个用户已登录
                </span>
                <span class="user-count" v-else> 点击添加用户 </span>
                <el-icon class="dropdown-arrow"><arrow-down /></el-icon>
            </div>

            <template #dropdown>
                <el-dropdown-menu class="user-dropdown-menu">
                    <div class="user-list-content">
                        <div v-for="user in loginUsers" :key="user.uid" class="user-item">
                            <div class="user-info">
                                <el-avatar
                                    :src="`data:image/jpeg;base64,${user.avatar}`"
                                    :size="32"
                                    class="user-avatar"
                                >
                                    {{ user.username.charAt(0) }}
                                </el-avatar>
                                <div class="user-details">
                                    <div class="username">{{ user.username }}</div>
                                    <div class="user-uid">UID: {{ user.uid }}</div>
                                </div>
                            </div>

                            <el-tooltip
                                :content="
                                    isUserHasUploadTasks(user.uid)
                                        ? '请先删除上传队列中属于该用户的任务'
                                        : '登出用户'
                                "
                                placement="top"
                            >
                                <el-button
                                    :disabled="isUserHasUploadTasks(user.uid)"
                                    @click="handleLogout(user)"
                                    class="logout-button"
                                    size="small"
                                    type="danger"
                                    plain
                                >
                                    <div class="exit-icon">
                                        <div class="exit-figure"></div>
                                        <div class="exit-door"></div>
                                        <div class="exit-arrow"></div>
                                    </div>
                                    登出
                                </el-button>
                            </el-tooltip>
                        </div>

                        <!-- 空状态 -->
                        <div v-if="loginUsers.length === 0" class="empty-state">
                            <el-empty description="暂无登录用户" :image-size="60">
                                <el-button type="primary" @click="$emit('show-login')">
                                    立即登录
                                </el-button>
                            </el-empty>
                        </div>
                    </div>

                    <div class="user-list-footer">
                        <el-button
                            type="primary"
                            size="small"
                            @click="$emit('show-login')"
                            class="add-user-btn"
                        >
                            <el-icon><plus /></el-icon>
                            添加用户
                        </el-button>
                    </div>
                </el-dropdown-menu>
            </template>
        </el-dropdown>
    </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAuthStore } from '../stores/auth'
import { useUploadStore } from '../stores/upload'
import { useUtilsStore } from '../stores/utils'
import { ElMessageBox } from 'element-plus'
import { User, ArrowDown, Plus } from '@element-plus/icons-vue'

// Props & Emits
const emit = defineEmits<{
    'show-login': []
    'user-logout': [uid: number]
}>()

// Stores
const authStore = useAuthStore()
const uploadStore = useUploadStore()
const utilsStore = useUtilsStore()

// 计算属性
const loginUsers = computed(() => {
    const users = authStore.loginUsers
    return users
})

// 检查用户是否有上传任务
const isUserHasUploadTasks = (uid: number): boolean => {
    return uploadStore.uploadQueue.some(task => task.user?.uid === uid)
}

// 处理下拉菜单命令
const handleCommand = (command: string) => {
    console.log('Dropdown command:', command)
}

// 处理登出操作
const handleLogout = async (user: any) => {
    try {
        // 检查用户是否有未完成的上传任务
        if (isUserHasUploadTasks(user.uid)) {
            utilsStore.showMessage('请先删除上传队列中属于该用户的任务', 'warning')
            return
        }

        // 确认对话框
        await ElMessageBox.confirm(
            `确定要退出用户 "${user.username}" 吗？退出后会删除所有本地数据。`,
            '确认登出',
            {
                confirmButtonText: '确定登出',
                cancelButtonText: '取消',
                type: 'warning',
                confirmButtonClass: 'el-button--danger'
            }
        )

        // 执行登出操作
        emit('user-logout', user.uid)
    } catch (error) {
        if (error !== 'cancel') {
            console.error('登出用户失败:', error)
            utilsStore.showMessage(`登出用户失败: ${error}`, 'error')
        }
    }
}
</script>

<style scoped>
.user-list-dropdown {
    display: inline-block;
}

.user-trigger {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.3s;
    background: #ffffff;
    color: #374151;
    border: 1px solid #d1d5db;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.user-trigger:hover {
    background: #f9fafb;
    border-color: #9ca3af;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
}

.user-icon {
    font-size: 16px;
    color: #6b7280;
}

.user-count {
    font-size: 14px;
    font-weight: 500;
    color: #374151;
}

.dropdown-arrow {
    font-size: 12px;
    color: #9ca3af;
    transition: transform 0.3s;
}

.user-trigger:hover .dropdown-arrow {
    transform: rotate(180deg);
}

/* 下拉菜单样式 */
:deep(.user-list-popper) {
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12);
    border: 1px solid #e1e6ea;
    overflow: hidden;
}

.user-dropdown-menu {
    min-width: 280px;
    max-width: 320px;
    padding: 0;
    border-radius: 8px;
}

.user-list-content {
    max-height: 300px;
    overflow-y: auto;
}

.user-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid #f1f5f9;
    transition: all 0.2s;
}

.user-item:hover {
    background-color: #f8fafc;
}

.user-item:last-child {
    border-bottom: none;
}

.user-info {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
}

.user-avatar {
    flex-shrink: 0;
}

.user-details {
    flex: 1;
    min-width: 0;
}

.username {
    font-size: 14px;
    font-weight: 500;
    color: #1e293b;
    margin-bottom: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.user-uid {
    font-size: 12px;
    color: #64748b;
}

.logout-button {
    flex-shrink: 0;
    min-width: auto;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 12px;
    transition: all 0.2s;
}

.logout-button:not(:disabled) {
    border-color: #ef4444;
    color: #ef4444;
}

.logout-button:not(:disabled):hover {
    background-color: #ef4444;
    color: white;
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(239, 68, 68, 0.3);
}

.logout-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

/* 安全出口样式图标 */
.exit-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 12px;
    position: relative;
    margin-right: 4px;
}

.exit-figure {
    position: absolute;
    left: 2px;
    top: 1px;
    width: 3px;
    height: 6px;
    background: currentColor;
    border-radius: 1px 1px 0 0;
}

.exit-figure::before {
    content: '';
    position: absolute;
    top: -2px;
    left: 0.5px;
    width: 2px;
    height: 2px;
    background: currentColor;
    border-radius: 50%;
}

.exit-door {
    position: absolute;
    right: 3px;
    top: 2px;
    width: 6px;
    height: 8px;
    border: 1px solid currentColor;
    border-bottom: none;
    border-radius: 1px 1px 0 0;
}

.exit-door::before {
    content: '';
    position: absolute;
    right: 1px;
    top: 2px;
    width: 1px;
    height: 1px;
    background: currentColor;
    border-radius: 50%;
}

.exit-arrow {
    position: absolute;
    left: 6px;
    top: 4px;
    width: 4px;
    height: 1px;
    background: currentColor;
}

.exit-arrow::after {
    content: '';
    position: absolute;
    right: -1px;
    top: -1px;
    width: 0;
    height: 0;
    border: 1.5px solid transparent;
    border-left-color: currentColor;
}

.logout-icon {
    font-size: 12px;
    margin-right: 4px;
}

.empty-state {
    padding: 20px;
    text-align: center;
}

.user-list-footer {
    padding: 12px 16px;
    background: #f8fafc;
    border-top: 1px solid #e1e6ea;
}

.add-user-btn {
    width: 100%;
    border-radius: 6px;
}

/* 滚动条样式 */
.user-list-content::-webkit-scrollbar {
    width: 4px;
}

.user-list-content::-webkit-scrollbar-track {
    background: #f1f5f9;
}

.user-list-content::-webkit-scrollbar-thumb {
    background: #cbd5e1;
    border-radius: 2px;
}

.user-list-content::-webkit-scrollbar-thumb:hover {
    background: #94a3b8;
}

/* 响应式设计 */
@media (max-width: 768px) {
    .user-dropdown-menu {
        min-width: 260px;
    }

    .user-item {
        padding: 10px 12px;
    }

    .username {
        font-size: 13px;
    }

    .user-uid {
        font-size: 11px;
    }

    .logout-button {
        padding: 4px 8px;
        font-size: 11px;
    }
}
</style>
