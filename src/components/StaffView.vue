<template>
    <div class="staff-view">
        <div class="staff-display" v-if="staffList.length">
            <div
                v-for="member in staffList"
                :key="`${member.mid}-${member.title}-${member.is_del}`"
                class="staff-item"
            >
                <el-tag
                    class="staff-tag"
                    size="small"
                    :type="member.is_del === 1 ? 'danger' : 'info'"
                    :effect="member.is_del === 1 ? 'dark' : 'plain'"
                    :closable="!disabled && member.is_del !== 1 && !isEditMode"
                    @close="removeStaff(member)"
                >
                    <!-- 暂不支持编辑稿件时删除联合投稿成员的功能 -->
                    <span :class="{ 'staff-deleted': member.is_del === 1 }">
                        {{ member.title || '成员' }} -- UID: {{ member.mid }}
                    </span>
                </el-tag>
                <el-button
                    v-if="member.is_del === 1"
                    class="staff-undo-btn"
                    type="primary"
                    link
                    size="small"
                    :disabled="disabled"
                    @click="undoRemove(member)"
                >
                    撤销
                </el-button>
            </div>
        </div>
        <span v-else class="staff-empty">无联合投稿成员</span>

        <el-button
            class="staff-add-btn"
            type="primary"
            text
            circle
            :disabled="disabled || isEditMode"
            @click="openDialog"
            :title="isEditMode ? '编辑稿件不允许新增联合投稿成员' : '新增联合投稿成员'"
        >
            <el-icon><Plus /></el-icon>
        </el-button>

        <el-dialog
            v-model="dialogVisible"
            title="新增联合投稿成员"
            width="420px"
            :close-on-click-modal="false"
        >
            <el-form label-width="80px">
                <el-form-item label="职能" required>
                    <el-select v-model="form.title" placeholder="请选择职能" style="width: 100%">
                        <el-option
                            v-for="role in roleOptions"
                            :key="role"
                            :label="role"
                            :value="role"
                        />
                    </el-select>
                </el-form-item>
                <el-form-item label="UID/昵称" required>
                    <MentionView
                        v-model="form.mid"
                        :user-uid="userUid"
                        placeholder="请输入 UID 或昵称"
                        :disabled="disabled"
                        @select="onMentionSelect"
                    />
                </el-form-item>
            </el-form>
            <template #footer>
                <el-button @click="dialogVisible = false">取消</el-button>
                <el-button type="primary" @click="addStaff">添加</el-button>
            </template>
        </el-dialog>
    </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import MentionView from './MentionView.vue'

interface StaffMember {
    title: string
    mid: number
    is_del: number
}

const props = defineProps<{
    modelValue?: StaffMember[]
    roleOptions?: string[]
    maxStaff?: number
    isEditMode?: boolean
    disabled?: boolean
    userUid?: number
}>()

const emit = defineEmits<{
    (e: 'update:modelValue', value: StaffMember[] | undefined): void
}>()

const roleOptions = computed(() => props.roleOptions || [])

const dialogVisible = ref(false)
const form = ref<{ title: string; mid: string }>({
    title: '',
    mid: ''
})

const isEditMode = computed(() => Boolean(props.isEditMode))

const staffList = computed(() =>
    (props.modelValue || []).map(member => ({
        ...member,
        is_del: member.is_del === 1 ? 1 : 0
    }))
)

const userUid = computed(() => Number(props.userUid || 0))

const getMaxStaff = () => {
    return props.maxStaff && props.maxStaff > 0 ? props.maxStaff : 10
}

const openDialog = () => {
    if (roleOptions.value.length === 0) {
        ElMessage.warning('当前活动未配置可选职能')
        return
    }

    form.value = {
        title: roleOptions.value[0],
        mid: ''
    }
    dialogVisible.value = true
}

const onMentionSelect = (item: { uid: string }) => {
    form.value.mid = String(item.uid || '')
}

const addStaff = () => {
    if (isEditMode.value) {
        ElMessage.warning('编辑稿件模板不允许新增联合投稿成员')
        return
    }

    const title = form.value.title
    const mid = Number.parseInt((form.value.mid || '').trim(), 10)

    if (!title) {
        ElMessage.warning('请选择职能')
        return
    }

    if (!Number.isInteger(mid) || mid <= 0) {
        ElMessage.warning('请输入有效 UID，或从昵称搜索结果中选择')
        return
    }

    const duplicateUid = staffList.value.some(member => member.mid === mid && member.is_del !== 1)
    if (duplicateUid) {
        ElMessage.warning('该 UID 已存在，请勿重复添加')
        return
    }

    const maxStaff = getMaxStaff()
    const activeCount = staffList.value.filter(member => member.is_del !== 1).length
    if (activeCount >= maxStaff) {
        ElMessage.warning(`联合投稿成员最多 ${maxStaff} 人`)
        return
    }

    const next = [...staffList.value, { title, mid, is_del: 0 }]
    emit('update:modelValue', next)
    dialogVisible.value = false
}

const removeStaff = (target: StaffMember) => {
    if (isEditMode.value) {
        const next = staffList.value.map(member => {
            if (member.mid === target.mid && member.title === target.title) {
                return { ...member, is_del: 1 }
            }
            return member
        })
        emit('update:modelValue', next)
        return
    }

    const next = staffList.value.filter(
        member => !(member.mid === target.mid && member.title === target.title)
    )
    emit('update:modelValue', next.length > 0 ? next : undefined)
}

const undoRemove = (target: StaffMember) => {
    const maxStaff = getMaxStaff()
    const activeCount = staffList.value.filter(member => member.is_del !== 1).length
    if (activeCount >= maxStaff) {
        ElMessage.warning(`联合投稿成员最多 ${maxStaff} 人`)
        return
    }

    const next = staffList.value.map(member => {
        if (member.mid === target.mid && member.title === target.title) {
            return { ...member, is_del: 0 }
        }
        return member
    })
    emit('update:modelValue', next)
}
</script>

<style scoped>
.staff-view {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    width: 100%;
}

.staff-display {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    flex: 1;
}

.staff-item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
}

.staff-tag {
    margin: 0;
}

.staff-deleted {
    text-decoration: line-through;
    opacity: 0.9;
}

.staff-undo-btn {
    padding: 0;
}

.staff-empty {
    color: #909399;
    font-size: 13px;
    flex: 1;
}

.staff-add-btn {
    flex-shrink: 0;
}
</style>
