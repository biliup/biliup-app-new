<template>
    <div class="tag-input" ref="tagContainerRef" 
         @focusin="addKeyboardListener" 
         @focusout="removeKeyboardListener"
         tabindex="0">
        <el-tag
            v-for="(tag, index) in modelValue"
            :key="`${tag}-${index}`"
            closable
            @close="removeTag(tag)"
            @click.stop="selectTag(index)"
            @keydown="handleTagKeydown($event, index)"
            class="tag-item"
            :class="{
                'tag-selected': selectedTagIndex === index,
                'tag-editing': editingTagIndex === index
            }"
            tabindex="0"
            v-show="editingTagIndex !== index"
        >
            {{ tag }}
        </el-tag>

        <!-- 编辑中的标签输入框 -->
        <el-input
            v-if="editingTagIndex >= 0"
            v-show="editingTagIndex >= 0"
            ref="editingTagInputRef"
            v-model="editingTagValue"
            size="small"
            placeholder="修改标签"
            @blur="handleEditFinish"
            @keydown.enter="handleEditFinish"
            @keydown.esc="cancelEditTag"
            class="tag-input-field tag-editing-field"
        />

        <el-input
            v-if="inputVisible"
            ref="tagInputRef"
            v-model="newTag"
            size="small"
            placeholder="按回车键添加"
            @keyup.enter="addTag(true)"
            @blur="addTag(false)"
            @keydown="handleNewTagKeydown"
            @keydown.esc="cancelEditTag"
            class="tag-input-field"
        />
        <el-button v-else size="small" @click.stop="showTagInput" class="add-tag-btn">
            + 添加标签
        </el-button>
    </div>
</template>

<script setup lang="ts">
import { ref, nextTick, watch } from 'vue'
import { useUtilsStore } from '../stores/utils'

const utilsStore = useUtilsStore()

// Props
interface Props {
    modelValue: string[]
}

const props = defineProps<Props>()

// Emits
const emit = defineEmits<{
    'update:modelValue': [tags: string[]]
}>()

// 响应式数据
const inputVisible = ref(false)
const newTag = ref('')
const tagInputRef = ref()
const editingTagInputRef = ref()
const tagContainerRef = ref()

// 标签编辑和选中状态
const selectedTagIndex = ref<number>(-1)
const editingTagIndex = ref<number>(-1)
const editingTagValue = ref<string>('')
const hasShownDuplicateWarning = ref<boolean>(false)

// 方法
const showTagInput = () => {
    // 如果正在编辑标签，先保存
    if (editingTagIndex.value >= 0) {
        finishEditTag()
    }

    inputVisible.value = true
    selectedTagIndex.value = -1 // 取消选中状态
    hasShownDuplicateWarning.value = false // 重置警告标志
    nextTick(() => {
        tagInputRef.value?.focus()
    })
}

const addTag = (keepInput = false) => {
    const tag = newTag.value.trim()

    if (!tag) {
        // 如果没有输入内容
        if (!keepInput) {
            inputVisible.value = false
        }
        return true // 空内容视为成功
    }

    if (props.modelValue.includes(tag)) {
        // 标签已存在，显示错误提示
        // 只在还没显示过警告时显示
        if (!hasShownDuplicateWarning.value) {
            utilsStore.showMessage(`标签 "${tag}" 已存在`, 'warning')
            hasShownDuplicateWarning.value = true
        }

        // 选中输入框内容，方便用户重新输入
        nextTick(() => {
            tagInputRef.value?.select()
        })
        return false
    }

    // 添加新标签
    const newTags = [...props.modelValue, tag]
    emit('update:modelValue', newTags)
    newTag.value = ''

    // 只有在keepInput为true时才保持输入状态和焦点
    if (keepInput && inputVisible.value) {
        nextTick(() => {
            tagInputRef.value?.focus()
        })
    } else {
        inputVisible.value = false
    }

    return true
}

const removeTag = (tag: string) => {
    const index = props.modelValue.indexOf(tag)
    if (index > -1) {
        const newTags = [...props.modelValue]
        newTags.splice(index, 1)
        emit('update:modelValue', newTags)

        // 更新选中状态：这个逻辑由 handleTagKeydown 处理，这里只处理通过关闭按钮删除的情况
        if (selectedTagIndex.value === index) {
            // 如果删除的是选中的标签，选择上一个标签或清除选中状态
            if (newTags.length > 0) {
                selectedTagIndex.value = index > 0 ? index - 1 : 0
            } else {
                selectedTagIndex.value = -1
            }
        } else if (selectedTagIndex.value > index) {
            // 如果删除的标签在选中标签之前，选中索引需要减1
            selectedTagIndex.value -= 1
        }
    }
}

// 选中标签
const selectTag = (index: number) => {
    // 如果正在输入新标签，先保存新标签
    if (inputVisible.value && newTag.value.trim()) {
        if (props.modelValue.includes(newTag.value.trim())) {
            nextTick(() => {
                tagInputRef.value?.select()
            })
            return
        }
    }

    // 如果正在编辑标签
    if (editingTagIndex.value >= 0) {
        if (editingTagIndex.value === index) {
            // 如果点击的是正在编辑的标签，不做任何操作
            return
        } else {
            // 如果点击的是其他标签，先保存编辑内容
            const editSuccess = finishEditTag()
            if (!editSuccess) {
                // 如果编辑失败（比如标签重复），不进行后续操作
                return
            }
        }
    }

    if (selectedTagIndex.value === index) {
        // 如果点击的是已选中的标签，进入编辑模式
        startEditTag(index)
    } else {
        selectedTagIndex.value = index
    }
}

// 开始编辑标签
const startEditTag = (index: number) => {
    editingTagIndex.value = index
    editingTagValue.value = props.modelValue[index]
    selectedTagIndex.value = -1
    hasShownDuplicateWarning.value = false // 重置警告标志
    nextTick(() => {
        editingTagInputRef.value?.focus()
        editingTagInputRef.value?.select()
    })
}

// 完成编辑标签
const finishEditTag = () => {
    if (editingTagIndex.value >= 0) {
        const newValue = editingTagValue.value.trim()

        // 如果没有输入内容，直接取消编辑
        if (!newValue) {
            cancelEditTag()
            return true
        }

        // 检查标签是否已存在（排除当前正在编辑的标签）
        const existingIndex = props.modelValue.indexOf(newValue)
        if (existingIndex >= 0 && existingIndex !== editingTagIndex.value) {
            // 只在还没显示过警告时显示
            if (!hasShownDuplicateWarning.value) {
                utilsStore.showMessage(`标签 "${newValue}" 已存在`, 'warning')
                hasShownDuplicateWarning.value = true
            }

            // 选中输入框内容，方便用户重新输入
            nextTick(() => {
                editingTagInputRef.value?.select()
            })
            return false // 返回失败状态
        }

        // 更新标签值
        const newTags = [...props.modelValue]
        newTags[editingTagIndex.value] = newValue
        emit('update:modelValue', newTags)
        cancelEditTag()
        return true // 返回成功状态
    }
    return true // 如果没有正在编辑的标签，返回成功
}

// 处理编辑输入框的回车和失去焦点事件（不需要返回值检查）
const handleEditFinish = () => {
    finishEditTag()
}

const cancelSelect = () => {
    selectedTagIndex.value = -1
    const activeElement = document.activeElement as HTMLElement
    if (activeElement && activeElement.classList.contains('tag-item')) {
        activeElement.blur()
    }
}

// 取消编辑标签
const cancelEditTag = () => {
    inputVisible.value = false
    newTag.value = ''
    editingTagIndex.value = -1
    editingTagValue.value = ''
    hasShownDuplicateWarning.value = false
}

// 处理标签的键盘事件
const handleTagKeydown = (event: KeyboardEvent, index: number) => {
    if (event.key === 'Backspace' || event.key === 'Delete') {
        hasShownDuplicateWarning.value = false
        event.preventDefault()
        const currentTag = props.modelValue[index]

        // 删除标签
        removeTag(currentTag)

        // 删除后自动选择上一个标签
        nextTick(() => {
            if (props.modelValue.length > 0) {
                // 如果删除的不是第一个标签，选择上一个标签
                if (index > 0) {
                    selectedTagIndex.value = index - 1
                } else {
                    // 如果删除的是第一个标签，选择新的第一个标签
                    selectedTagIndex.value = 0
                }

                // 聚焦到选中的标签
                nextTick(() => {
                    const tagElements = document.querySelectorAll('.tag-item')
                    const targetElement = tagElements[selectedTagIndex.value] as HTMLElement
                    targetElement?.focus()
                })
            } else {
                // 如果没有标签了，清除选中状态
                selectedTagIndex.value = -1
            }
        })
    }
}

// 处理新标签输入框的键盘事件
const handleNewTagKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Backspace' && newTag.value === '' && props.modelValue.length > 0) {
        // 如果输入框为空且按下退格键，选中最后一个标签
        event.preventDefault()
        inputVisible.value = false
        selectedTagIndex.value = props.modelValue.length - 1
        nextTick(() => {
            // 找到对应的标签元素并聚焦
            const tagElements = document.querySelectorAll('.tag-item')
            const lastTagElement = tagElements[props.modelValue.length - 1] as HTMLElement
            lastTagElement?.focus()
        })
    }
}

// 键盘监听器管理
let focusTimeout: number | null = null

const addKeyboardListener = () => {
    // 清除可能存在的延时器
    if (focusTimeout) {
        clearTimeout(focusTimeout)
        focusTimeout = null
    }
    document.addEventListener('keydown', handleKeyboardNavigation)
}

const removeKeyboardListener = () => {
    // 使用延时来检查焦点是否真的离开了整个组件
    focusTimeout = setTimeout(() => {
        // 检查当前聚焦的元素是否还在标签组件内
        const activeElement = document.activeElement
        const isStillInTagContainer = tagContainerRef.value?.contains(activeElement)
        
        if (!isStillInTagContainer) {
            // 焦点确实离开了整个标签组件
            document.removeEventListener('keydown', handleKeyboardNavigation)
            cancelSelect()
        }
        focusTimeout = null
    }, 0)
}

// 处理键盘导航
const handleKeyboardNavigation = (event: KeyboardEvent) => {
    // 只在有标签且没有输入或编辑状态时处理方向键
    if (props.modelValue.length === 0 || inputVisible.value || editingTagIndex.value >= 0) {
        return
    }

    if (event.key === 'ArrowLeft') {
        event.preventDefault()
        if (selectedTagIndex.value <= 0) {
            // 如果当前没有选中或选中第一个，选中最后一个
            selectedTagIndex.value = props.modelValue.length - 1
        } else {
            // 选中前一个
            selectedTagIndex.value -= 1
        }

        // 聚焦到选中的标签
        focusSelectedTag()
    } else if (event.key === 'ArrowRight') {
        event.preventDefault()
        if (selectedTagIndex.value >= props.modelValue.length - 1 || selectedTagIndex.value < 0) {
            // 如果当前没有选中或选中最后一个，选中第一个
            selectedTagIndex.value = 0
        } else {
            // 选中下一个
            selectedTagIndex.value += 1
        }

        // 聚焦到选中的标签
        focusSelectedTag()
    } else if (event.key === 'Enter') {
        if (selectedTagIndex.value >= 0) {
            // 如果有选中标签，进入编辑模式
            event.preventDefault()
            event.stopPropagation()
            // 使用 nextTick 确保事件处理完成后再进入编辑状态
            nextTick(() => {
                startEditTag(selectedTagIndex.value)
            })
        }
    } else if (event.key === 'Escape') {
        event.preventDefault()
        if (selectedTagIndex.value >= 0) {
            selectedTagIndex.value = -1
            // 让当前聚焦的元素失去焦点
            const activeElement = document.activeElement as HTMLElement
            if (activeElement && activeElement.classList.contains('tag-item')) {
                activeElement.blur()
            }
        }
    } else if (event.key === 'Tab') {
        if (selectedTagIndex.value >= 0) {
            // 如果有选中标签，进入编辑模式
            event.preventDefault()
            event.stopPropagation()

            cancelSelect()
            nextTick(() => {
                inputVisible.value = true
                nextTick(() => {
                    tagInputRef.value?.focus()
                })
            })
        } else if (inputVisible.value) {
            // 如果输入框可见，按 Tab 键时不做任何操作
            event.preventDefault()
        }
    }
}

// 聚焦到选中的标签
const focusSelectedTag = () => {
    nextTick(() => {
        const tagElements = document.querySelectorAll('.tag-item')
        if (selectedTagIndex.value >= 0 && tagElements[selectedTagIndex.value]) {
            const targetElement = tagElements[selectedTagIndex.value] as HTMLElement
            targetElement?.focus()
        }
    })
}

// 清空标签功能（供外部调用）
const clearTags = () => {
    emit('update:modelValue', [])
    selectedTagIndex.value = -1
    cancelEditTag()
    inputVisible.value = false
}

// 监听编辑内容变化，重置警告标志
watch(editingTagValue, () => {
    hasShownDuplicateWarning.value = false
})

// 监听新标签输入内容变化，重置警告标志
watch(newTag, () => {
    hasShownDuplicateWarning.value = false
})

// 暴露方法供父组件使用
defineExpose({
    clearTags
})
</script>

<style scoped>
.tag-input {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
}

.tag-item {
    margin: 0;
    cursor: pointer;
    transition: all 0.2s ease;
    outline: none;
}

.tag-item:hover {
    background-color: var(--el-color-primary-light-8);
}

.tag-item:focus {
    background-color: var(--el-color-primary-light-7);
    border-color: var(--el-color-primary);
    box-shadow: 0 0 0 2px var(--el-color-primary-light-8);
}

.tag-item.tag-selected {
    background-color: var(--el-color-primary-light-6);
    border-color: var(--el-color-primary);
}

.tag-item.tag-editing {
    display: none;
}

.tag-editing-field {
    width: 120px;
    margin-right: 8px;
}

.tag-input-field {
    width: 100px;
}

.add-tag-btn {
    height: 24px;
}
</style>
