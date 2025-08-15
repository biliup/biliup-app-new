<template>
    <div>
        <!-- 创建模板对话框 -->
        <el-dialog v-model="showDialog" title="新建模板" width="500px">
            <el-form :model="newTemplateForm" label-width="80px">
                <el-form-item label="选择用户">
                    <el-select v-model="newTemplateForm.userUid" placeholder="请选择用户">
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

                <el-form-item label="模板类型">
                    <el-radio-group v-model="newTemplateForm.templateType">
                        <el-radio value="blank">空白模板</el-radio>
                        <el-radio value="bv">BV/AV号</el-radio>
                    </el-radio-group>
                </el-form-item>

                <el-form-item
                    label="模板名称"
                    required
                    v-if="newTemplateForm.templateType === 'blank'"
                >
                    <el-input
                        v-model="newTemplateForm.name"
                        placeholder="请输入模板名称"
                        maxlength="50"
                    />
                </el-form-item>

                <el-form-item label="BV/AV号" required v-if="newTemplateForm.templateType === 'bv'">
                    <el-input
                        v-model="newTemplateForm.bvNumber"
                        placeholder="请输入BV号或AV号，如: BV1xx4y1z7xx 或 av12345"
                        maxlength="20"
                    />
                </el-form-item>

                <el-form-item label="操作类型" v-if="newTemplateForm.templateType === 'bv'">
                    <el-radio-group v-model="newTemplateForm.actionType">
                        <el-radio value="edit">编辑</el-radio>
                        <el-radio value="copy">复制</el-radio>
                    </el-radio-group>
                    <div class="form-tip">
                        <div>编辑：直接修改现有稿件</div>
                        <div>复制：基于现有稿件创建新模板</div>
                    </div>
                </el-form-item>
                <el-form-item
                    label="模板名称"
                    required
                    v-if="
                        newTemplateForm.templateType === 'bv' &&
                        newTemplateForm.actionType === 'copy'
                    "
                >
                    <el-input
                        v-model="newTemplateForm.name"
                        placeholder="请输入模板名称"
                        maxlength="50"
                    />
                </el-form-item>
            </el-form>
            <template #footer>
                <span class="dialog-footer">
                    <el-button @click="closeDialog">取消</el-button>
                    <el-button type="primary" @click="createNewTemplate">确定</el-button>
                </span>
            </template>
        </el-dialog>
    </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAuthStore } from '../stores/auth'
import { uploadForm, useUserConfigStore } from '../stores/user_config'
import { useUtilsStore } from '../stores/utils'
import { ElMessage } from 'element-plus'

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
    userUid: null,
    name: '',
    templateType: 'blank', // 'blank' | 'bv'
    bvNumber: '',
    actionType: 'copy' // 'edit' | 'copy'
})

// 方法
const closeDialog = () => {
    showDialog.value = false
    resetForm()
}

const resetForm = () => {
    newTemplateForm.value = {
        userUid: null,
        name: '',
        templateType: 'blank',
        bvNumber: '',
        actionType: 'copy'
    }
}

// 创建新模板
const createNewTemplate = async () => {
    const targetUserUid = newTemplateForm.value.userUid
    if (!targetUserUid) {
        ElMessage.error('请选择用户')
        return
    }

    try {
        if (newTemplateForm.value.templateType === 'blank') {
            // 空白模板
            if (!newTemplateForm.value.name.trim()) {
                ElMessage.error('请输入模板名称')
                return
            }

            const templateName = newTemplateForm.value.name.trim()
            await userConfigStore.addUserTemplate(targetUserUid, templateName)

            ElMessage.success('空白模板创建成功')
            emit('template-created', targetUserUid, templateName)
        } else if (newTemplateForm.value.templateType === 'bv') {
            // BV/AV号模板
            if (!newTemplateForm.value.bvNumber.trim()) {
                ElMessage.error('请输入BV号或AV号')
                return
            }

            const bvNumber = newTemplateForm.value.bvNumber.trim()
            const actionType = newTemplateForm.value.actionType
            const templateName = newTemplateForm.value.name.trim() || `编辑_${bvNumber}`

            const isEdit = actionType === 'edit'
            await createTemplateFromBV(targetUserUid, bvNumber, templateName, isEdit)

            ElMessage.success('基于稿件创建模板成功')
            emit('template-created', targetUserUid, templateName)
        }

        closeDialog()
    } catch (error) {
        console.error('创建模板失败:', error)
        // 提供更详细的错误信息
        let errorMessage = '创建模板失败'
        if (error instanceof Error) {
            errorMessage = `创建模板失败: ${error.message}`
        } else if (typeof error === 'string') {
            errorMessage = `创建模板失败: ${error}`
        }
        ElMessage.error(errorMessage)
    }
}

// 根据BV号创建模板
const createTemplateFromBV = async (
    userUid: number,
    bvNumber: string,
    templateName: string,
    isEdit: boolean
) => {
    try {
        console.log(`从BV号 ${bvNumber} 创建模板: ${templateName}`)

        const newForm = (await utilsStore.getVideoDetail(userUid, bvNumber)) as uploadForm

        for (const video of newForm.videos) {
            video.id = video.filename
            video.path = ''
        }

        const newTemplate = userConfigStore.buildTemplateFromUploadForm(newForm)

        if (!isEdit) {
            newTemplate.aid = undefined
        }

        if (newForm.aid && (await utilsStore.getSeasonList(userUid))) {
            const season_id = await utilsStore.getVideoSeason(userUid, newForm.aid)

            if (season_id !== 0) {
                const section_id = await utilsStore.seasonlist.find(
                    item => item.season_id === season_id
                )?.section_id
                newTemplate.season_id = season_id
                newTemplate.section_id = section_id
            }
        }

        await userConfigStore.addUserTemplate(userUid, templateName, newTemplate)
    } catch (error) {
        console.error('从BV号创建模板失败: ', error)
        ElMessage.error(`从BV号创建模板失败: ${error}`)
        throw error
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
