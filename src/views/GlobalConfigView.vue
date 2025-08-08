<template>
    <el-dialog
        v-model="visible"
        title="全局设置"
        width="500px"
        :before-close="handleClose"
        @close="handleDialogClose"
    >
        <el-form :model="configForm" label-width="140px" v-loading="loading">
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
import { ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useUserConfigStore } from '../stores/user_config'

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

// 响应式数据
const visible = ref(false)
const loading = ref(false)
const saving = ref(false)

const configForm = ref<GlobalConfigForm>({
    max_curr: 2,
    auto_upload: true,
    auto_start: true
})

// 保存原始配置用于检查变化
const originalConfig = ref<GlobalConfigForm>({
    max_curr: 2,
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
        }
    },
    { immediate: true }
)

// 监听 visible 变化
watch(visible, newValue => {
    emit('update:modelValue', newValue)
})

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
        ElMessage.error(`加载全局配置失败: ${error}`)
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

        ElMessage.success('配置保存成功')
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
    // 重置表单为默认值
    configForm.value = {
        max_curr: 2,
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

.dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
}

.el-switch {
    margin-right: 10px;
}

:deep(.el-form-item__label) {
    font-weight: 500;
}

:deep(.el-slider) {
    margin: 0 12px;
}
</style>
