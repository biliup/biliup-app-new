<template>
    <div class="season-selector">
        <div class="season-select-row">
            <el-select
                v-model="selectedSeasonId"
                placeholder="请选择合集"
                clearable
                filterable
                :loading="isSearching"
                :disabled="disabled"
                class="season-select season-main-select"
                @change="handleSeasonChange"
                @clear="clearSelection"
            >
                <el-option
                    v-for="season in allSeasons"
                    :key="season.season_id"
                    :label="season.title"
                    :value="season.season_id"
                    class="season-option"
                >
                    <div class="season-option-content">
                        <span class="season-option-title">{{ season.title }}</span>
                        <span class="season-option-id">ID: {{ season.season_id }}</span>
                    </div>
                </el-option>
            </el-select>

            <el-select
                v-if="showSectionSelect"
                v-model="selectedSectionId"
                placeholder="请选择小节"
                filterable
                :disabled="disabled"
                class="season-select season-section-select"
                @change="handleSectionChange"
            >
                <el-option
                    v-for="section in selectedSeasonSections"
                    :key="section.section_id"
                    :label="section.title"
                    :value="section.section_id"
                    class="season-option"
                >
                    <div class="season-option-content">
                        <span class="season-option-title">{{ section.title }}</span>
                        <span class="season-option-id">ID: {{ section.section_id }}</span>
                    </div>
                </el-option>
            </el-select>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useUtilsStore } from '../stores/utils'

interface Props {
    modelValue?: number
    sectionId?: number
    userUid?: number
    disabled?: boolean
}

interface SeasonSection {
    section_id: number
    title: string
}

interface SeasonOption {
    season_id: number
    title: string
    sections: SeasonSection[]
}

const props = withDefaults(defineProps<Props>(), {
    disabled: false
})

const emit = defineEmits<{
    'update:modelValue': [value: number | undefined]
    'update:sectionId': [value: number | undefined]
}>()

const utilsStore = useUtilsStore()

const allSeasons = ref<SeasonOption[]>([])
const selectedSeasonId = ref<number | undefined>(props.modelValue)
const selectedSectionId = ref<number | undefined>(props.sectionId)
const isSearching = ref(false)

const selectedSeasonSections = computed(() => {
    if (selectedSeasonId.value === undefined) {
        return []
    }

    return allSeasons.value.find(item => item.season_id === selectedSeasonId.value)?.sections || []
})

const showSectionSelect = computed(() => {
    return selectedSeasonId.value !== undefined && selectedSeasonSections.value.length > 0
})

const uniqueSections = (sections: SeasonSection[]) => {
    return sections.filter(
        (section, index, list) => index === list.findIndex(item => item.section_id === section.section_id)
    )
}

const normalizeSeason = (season: any): SeasonOption | null => {
    const seasonId = Number(season?.season_id)
    if (!Number.isFinite(seasonId) || seasonId <= 0) {
        return null
    }

    const seasonTitle = String(season?.title || `合集 ${seasonId}`)

    let sections: SeasonSection[] = []
    if (Array.isArray(season?.sections)) {
        sections = season.sections
            .map((section: any) => {
                const sectionId = Number(section?.section_id)
                if (!Number.isFinite(sectionId) || sectionId <= 0) {
                    return null
                }

                return {
                    section_id: sectionId,
                    title: String(section?.title || `小节 ${sectionId}`)
                }
            })
            .filter(Boolean) as SeasonSection[]
    }

    const defaultSectionId = Number(season?.section_id)
    if (sections.length === 0 && Number.isFinite(defaultSectionId) && defaultSectionId > 0) {
        sections = [
            {
                section_id: defaultSectionId,
                title: '默认小节'
            }
        ]
    }

    return {
        season_id: seasonId,
        title: seasonTitle,
        sections: uniqueSections(sections)
    }
}

const syncSelectionWithCurrentSeason = () => {
    if (selectedSeasonId.value === undefined) {
        selectedSectionId.value = undefined
        return
    }

    const season = allSeasons.value.find(item => item.season_id === selectedSeasonId.value)
    if (!season) {
        selectedSeasonId.value = undefined
        selectedSectionId.value = undefined
        return
    }

    if (season.sections.length === 0) {
        selectedSectionId.value = undefined
        return
    }

    const hasSelectedSection = season.sections.some(
        section => section.section_id === selectedSectionId.value
    )

    if (!hasSelectedSection) {
        selectedSectionId.value = season.sections[0].section_id
    }
}

const clearSelection = () => {
    selectedSeasonId.value = undefined
    selectedSectionId.value = undefined
}

const handleSeasonChange = (value: number | undefined) => {
    if (value === undefined || value === null) {
        clearSelection()
        return
    }

    selectedSeasonId.value = value
    const season = allSeasons.value.find(item => item.season_id === value)
    selectedSectionId.value = season?.sections[0]?.section_id
}

const handleSectionChange = (value: number | undefined) => {
    selectedSectionId.value = value
}

const loadSeasons = async () => {
    if (!props.userUid) {
        allSeasons.value = []
        clearSelection()
        return
    }

    try {
        isSearching.value = true
        await utilsStore.getSeasonList(props.userUid)

        const seasonMap = new Map<number, SeasonOption>()

        for (const rawSeason of utilsStore.seasonlist || []) {
            const season = normalizeSeason(rawSeason)
            if (!season) {
                continue
            }

            const current = seasonMap.get(season.season_id)
            if (!current) {
                seasonMap.set(season.season_id, season)
                continue
            }

            current.sections = uniqueSections([...current.sections, ...season.sections])
            if (!current.title && season.title) {
                current.title = season.title
            }
        }

        allSeasons.value = Array.from(seasonMap.values())
        syncSelectionWithCurrentSeason()
    } catch (error) {
        console.error('加载合集列表失败:', error)
        utilsStore.showMessage(`加载合集列表失败: ${error}`, 'error')
    } finally {
        isSearching.value = false
    }
}

watch(
    () => props.modelValue,
    newValue => {
        selectedSeasonId.value = newValue
        syncSelectionWithCurrentSeason()
    }
)

watch(
    () => props.sectionId,
    newValue => {
        selectedSectionId.value = newValue
        syncSelectionWithCurrentSeason()
    }
)

watch(
    () => props.userUid,
    () => {
        loadSeasons()
    },
    { immediate: true }
)

watch(selectedSeasonId, value => {
    emit('update:modelValue', value)
})

watch(selectedSectionId, value => {
    emit('update:sectionId', value)
})
</script>

<style scoped>
.season-selector {
    width: 100%;
}

.season-select-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
}

.season-select {
    width: 100%;
}

.season-main-select {
    flex: 1;
    min-width: 0;
}

.season-section-select {
    flex: 1;
    min-width: 140px;
}

.season-option-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 8px 0;
}

.season-option-title {
    flex: 1;
    font-size: 14px;
    color: #303133;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-right: 12px;
}

.season-option-id {
    flex-shrink: 0;
    font-size: 12px;
    color: #909399;
    background: #f0f2f5;
    padding: 2px 6px;
    border-radius: 3px;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
}

.season-select :deep(.el-input__inner) {
    border-radius: 6px;
    border-color: #dcdfe6;
    transition: all 0.3s;
}

.season-select :deep(.el-input__inner:focus) {
    border-color: #409eff;
    box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.1);
}

.season-select :deep(.el-input__inner:hover) {
    border-color: #409eff;
}

.season-select :deep(.el-input__suffix) {
    color: #c0c4cc;
}

:deep(.el-select-dropdown) {
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    border: 1px solid #e1e6ea;
}

:deep(.el-select-dropdown .el-select-dropdown__item) {
    padding: 12px 16px;
    line-height: 1.4;
}

:deep(.el-select-dropdown .el-select-dropdown__item:hover) {
    background-color: #f0f9ff;
}

:deep(.el-select-dropdown .el-select-dropdown__item.is-selected) {
    background-color: #f0f9ff;
    color: #409eff;
}

:deep(.el-select .el-input.is-focus .el-input__inner) {
    border-color: #409eff;
}

:deep(.el-select .el-input__suffix .el-input__suffix-inner .el-select__caret) {
    transition: all 0.3s;
}

:deep(.el-select .el-input__suffix .el-input__suffix-inner .el-select__caret.is-reverse) {
    transform: rotateZ(180deg);
}

@media (max-width: 768px) {
    .season-select-row {
        flex-direction: column;
        align-items: stretch;
    }

    .season-option-title {
        font-size: 13px;
    }

    .season-option-id {
        font-size: 11px;
        padding: 1px 4px;
    }
}
</style>
