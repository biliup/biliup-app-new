<template>
    <div>
        <!-- 创建模板对话框 -->
        <el-dialog v-model="showDialog" :title="t('新建模板')" width="500px">
            <el-form :model="newTemplateForm" label-width="80px">
                <el-form-item :label="t('选择用户')">
                    <el-select v-model="newTemplateForm.userUid" :placeholder="t('请选择用户')">
                        <el-option
                            v-for="user in loginUsers"
                            :key="user.uid"
                            :label="user.username"
                            :value="user.uid"
                        >
                            <div class="user-option">
                                <el-avatar
                                    :src="`data:image/jpeg;base64,${user.avatar}`"
                                    :size="20"
                                >
                                    {{ user.username.charAt(0) }}
                                </el-avatar>
                                <span class="user-option-name">{{ user.username }}</span>
                            </div>
                        </el-option>
                    </el-select>
                </el-form-item>

                <el-form-item :label="t('模板类型')">
                    <el-radio-group v-model="newTemplateForm.templateType">
                        <el-radio value="blank">{{ t('空白模板') }}</el-radio>
                        <el-radio value="bv">{{ t('BV/AV号') }}</el-radio>
                    </el-radio-group>
                </el-form-item>

                <el-form-item
                    :label="t('模板名称')"
                    required
                    v-if="newTemplateForm.templateType === 'blank'"
                >
                    <el-input
                        v-model="newTemplateForm.name"
                        :placeholder="t('请输入模板名称')"
                        maxlength="50"
                    />
                </el-form-item>

                <el-form-item
                    :label="t('BV/AV号')"
                    required
                    v-if="newTemplateForm.templateType === 'bv'"
                >
                    <el-input
                        v-model="newTemplateForm.bvNumber"
                        :placeholder="t('请输入BV号或AV号，如: BV1xx4y1z7xx 或 av12345')"
                        maxlength="20"
                    />
                </el-form-item>

                <el-form-item :label="t('操作类型')" v-if="newTemplateForm.templateType === 'bv'">
                    <el-radio-group v-model="newTemplateForm.actionType">
                        <el-radio value="edit">{{ t('编辑') }}</el-radio>
                        <el-radio value="copy">{{ t('复制') }}</el-radio>
                    </el-radio-group>
                    <div class="form-tip">
                        <div>{{ t('编辑：直接修改现有稿件') }}</div>
                        <div>{{ t('复制：基于现有稿件创建新模板') }}</div>
                    </div>
                </el-form-item>
                <el-form-item
                    v-if="
                        newTemplateForm.templateType === 'bv' &&
                        newTemplateForm.actionType === 'edit'
                    "
                >
                    <el-button
                        :disabled="!newTemplateForm.userUid"
                        @click="videoPickerVisible = true"
                    >
                        {{ t('选择已有稿件') }}
                    </el-button>
                </el-form-item>
                <el-form-item
                    :label="t('模板名称')"
                    required
                    v-if="
                        newTemplateForm.templateType === 'bv' &&
                        newTemplateForm.actionType === 'copy'
                    "
                >
                    <el-input
                        v-model="newTemplateForm.name"
                        :placeholder="t('请输入模板名称')"
                        maxlength="50"
                    />
                </el-form-item>
            </el-form>
            <template #footer>
                <span class="dialog-footer">
                    <el-button @click="closeDialog">{{ t('取消') }}</el-button>
                    <el-button type="primary" @click="createNewTemplate">{{ t('确定') }}</el-button>
                </span>
            </template>
        </el-dialog>
        <CreatorVideoPicker
            v-model="videoPickerVisible"
            :uid="newTemplateForm.userUid"
            :username="
                loginUsers.find(user => user.uid === newTemplateForm.userUid)?.username || ''
            "
            @select="newTemplateForm.bvNumber = $event"
        />
    </div>
</template>

<script setup lang="ts">
import { t } from '../i18n'
import { ref, computed, watch } from 'vue'
import { useAuthStore } from '../stores/auth'
import { useUserConfigStore } from '../stores/user_config'
import { useUtilsStore } from '../stores/utils'
import CreatorVideoPicker from './CreatorVideoPicker.vue'

// Props
const props = defineProps<{
    modelValue: boolean
}>()

// Emits
const emit = defineEmits<{
    'update:modelValue': [value: boolean]
    'template-created': [userUid: number, templateName: string]
}>()

// Stores
const authStore = useAuthStore()
const userConfigStore = useUserConfigStore()
const utilsStore = useUtilsStore()

// 计算属性
const loginUsers = computed(() => authStore.loginUsers)

// 响应式数据
const showDialog = computed({
    get: () => props.modelValue,
    set: value => emit('update:modelValue', value)
})

const newTemplateForm = ref({
    userUid: null as number | null,
    name: '',
    templateType: 'blank', // 'blank' | 'bv'
    bvNumber: '',
    actionType: 'copy' // 'edit' | 'copy'
})
const videoPickerVisible = ref(false)

watch(
    () => [
        props.modelValue,
        newTemplateForm.value.userUid,
        newTemplateForm.value.templateType,
        newTemplateForm.value.actionType
    ],
    () => {
        videoPickerVisible.value = false
    }
)

// localStorage key for storing user preferences
const STORAGE_KEY = 'new-template-preferences'

// 加载用户上次的选择
const loadUserPreferences = () => {
    try {
        const saved = localStorage.getItem(STORAGE_KEY)
        if (saved) {
            const preferences = JSON.parse(saved)
            // 只加载有效的用户UID（确保用户仍然登录）
            if (
                preferences.userUid &&
                loginUsers.value.some(user => user.uid === preferences.userUid)
            ) {
                newTemplateForm.value.userUid = preferences.userUid
            }
            // 加载其他设置
            if (preferences.templateType) {
                newTemplateForm.value.templateType = preferences.templateType
            }
            if (preferences.actionType) {
                newTemplateForm.value.actionType = preferences.actionType
            }
        }
    } catch (error) {
        console.error(t('加载用户偏好设置失败:'), error)
    }
}

// 保存用户选择
const saveUserPreferences = () => {
    try {
        const preferences = {
            userUid: newTemplateForm.value.userUid,
            templateType: newTemplateForm.value.templateType,
            actionType: newTemplateForm.value.actionType
        }
        localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences))
    } catch (error) {
        console.error(t('保存用户偏好设置失败:'), error)
    }
}

// 监听表单变化，自动保存用户偏好
watch(() => newTemplateForm.value.userUid, saveUserPreferences)
watch(() => newTemplateForm.value.templateType, saveUserPreferences)
watch(() => newTemplateForm.value.actionType, saveUserPreferences)

// 当对话框打开时加载用户偏好
watch(
    () => props.modelValue,
    newValue => {
        if (newValue) {
            loadUserPreferences()
        }
    }
)

// 方法
const closeDialog = () => {
    showDialog.value = false
    resetForm()
}

const resetForm = () => {
    // 只重置输入字段，保留用户偏好设置
    newTemplateForm.value.name = ''
    newTemplateForm.value.bvNumber = ''
    // 不重置 userUid, templateType, actionType，保持用户上次的选择
}

// 创建新模板
const createNewTemplate = async () => {
    const targetUserUid = newTemplateForm.value.userUid
    if (!targetUserUid) {
        utilsStore.showMessage(t('请选择用户'), 'error')
        return
    }

    try {
        if (newTemplateForm.value.templateType === 'blank') {
            // 空白模板
            if (!newTemplateForm.value.name.trim()) {
                utilsStore.showMessage(t('请输入模板名称'), 'error')
                return
            }

            const templateName = newTemplateForm.value.name.trim()
            await userConfigStore.addUserTemplate(targetUserUid, templateName)

            utilsStore.showMessage(t('空白模板创建成功'), 'success')
            emit('template-created', targetUserUid, templateName)
        } else if (newTemplateForm.value.templateType === 'bv') {
            // BV/AV号模板
            if (!newTemplateForm.value.bvNumber.trim()) {
                utilsStore.showMessage(t('请输入BV号或AV号'), 'error')
                return
            }

            const bvNumber = newTemplateForm.value.bvNumber.trim()
            const actionType = newTemplateForm.value.actionType
            const templateName = newTemplateForm.value.name.trim() || `${bvNumber}`

            const isEdit = actionType === 'edit'
            await userConfigStore.createTemplateFromVideo(
                targetUserUid,
                bvNumber,
                templateName,
                isEdit
            )

            utilsStore.showMessage(t('基于稿件创建模板成功'), 'success')
            emit('template-created', targetUserUid, templateName)
        }

        closeDialog()
    } catch (error) {
        console.error(t('创建模板失败:'), error)
        // 提供更详细的错误信息
        let errorMessage = t('创建模板失败')
        if (error instanceof Error) {
            errorMessage = t('创建模板失败: {0}', { 0: String(error.message) })
        } else if (typeof error === 'string') {
            errorMessage = t('创建模板失败: {0}', { 0: String(error) })
        }
        utilsStore.showMessage(errorMessage, 'error')
    }
}
</script>

<style scoped>
.user-option {
    display: flex;
    align-items: center;
    gap: 8px;
}

.user-option-name {
    font-size: 14px;
}

.form-tip {
    margin-top: 8px;
    font-size: 12px;
    color: #666;
    line-height: 1.4;
}

.form-tip > div {
    margin-bottom: 2px;
}

.dialog-footer {
    display: flex;
    gap: 10px;
}
</style>
