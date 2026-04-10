<template>
    <div class="data-picker" ref="rootRef">
        <button
            type="button"
            class="picker-trigger"
            :class="{ disabled, empty: !modelValue }"
            :disabled="disabled"
            @click="togglePanel"
        >
            <span class="picker-text">{{ displayText }}</span>
            <span class="picker-icon">▾</span>
        </button>

        <div v-if="panelVisible" class="picker-panel">
            <div class="panel-date">
                <div class="month-header">
                    <button
                        type="button"
                        class="month-btn"
                        :disabled="!canGoPrevMonth"
                        @click="goPrevMonth"
                    >
                        <
                    </button>
                    <span class="month-title">{{ viewYear }}-{{ pad2(viewMonth + 1) }}</span>
                    <button
                        type="button"
                        class="month-btn"
                        :disabled="!canGoNextMonth"
                        @click="goNextMonth"
                    >
                        >
                    </button>
                </div>

                <div class="week-row">
                    <span v-for="day in weekDays" :key="day">{{ day }}</span>
                </div>

                <div class="date-grid">
                    <button
                        v-for="cell in dayCells"
                        :key="cell.key"
                        type="button"
                        class="date-cell"
                        :class="{
                            selected: cell.selectable && isSameDay(cell.date, selectedDay),
                            hidden: !cell.selectable
                        }"
                        :disabled="!cell.selectable"
                        @click="selectDay(cell.date)"
                    >
                        <span v-if="cell.selectable">{{ cell.day }}</span>
                    </button>
                </div>
            </div>

            <div class="panel-time">
                <div v-if="!hasAnyTime" class="time-empty">该日期无可选时间</div>
                <div v-else class="time-columns">
                    <div class="time-col">
                        <div class="time-col-list">
                            <button
                                v-for="hour in hourOptions"
                                :key="`h-${hour}`"
                                type="button"
                                class="time-item"
                                :class="{ selected: selectedHour === hour }"
                                @click="selectHour(hour)"
                            >
                                {{ pad2(hour) }}
                            </button>
                        </div>
                    </div>

                    <div class="time-col">
                        <div class="time-col-list">
                            <button
                                v-for="minute in minuteOptions"
                                :key="`m-${minute}`"
                                type="button"
                                class="time-item"
                                :class="{ selected: selectedMinute === minute }"
                                @click="selectMinute(minute)"
                            >
                                {{ pad2(minute) }}
                            </button>
                        </div>
                    </div>

                    <div class="time-col">
                        <div class="time-col-list">
                            <button
                                v-for="second in secondOptions"
                                :key="`s-${second}`"
                                type="button"
                                class="time-item"
                                :class="{ selected: selectedSecond === second }"
                                @click="selectSecond(second)"
                            >
                                {{ pad2(second) }}
                            </button>
                        </div>
                    </div>
                </div>
            </div>

            <div class="panel-footer">
                <button type="button" class="footer-btn btn-clear" @click="handleClear">
                    清空
                </button>
                <button type="button" class="footer-btn btn-cancel" @click="handleCancel">
                    取消
                </button>
                <button
                    type="button"
                    class="footer-btn btn-confirm"
                    :disabled="!canConfirm"
                    @click="handleConfirm"
                >
                    确定
                </button>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

const props = withDefaults(
    defineProps<{
        modelValue: Date | null
        disabled?: boolean
        placeholder?: string
    }>(),
    {
        disabled: false,
        placeholder: '选择发布时间'
    }
)

const emit = defineEmits<{
    (e: 'update:modelValue', value: Date | null): void
}>()

const rootRef = ref<HTMLElement | null>(null)
const panelVisible = ref(false)

const minDateTime = () => new Date(Date.now() + 2 * 60 * 60 * 1000)
const maxDateTime = () => new Date(Date.now() + 15 * 24 * 60 * 60 * 1000)

const selectedDay = ref<Date>(stripTime(minDateTime()))
const viewYear = ref(selectedDay.value.getFullYear())
const viewMonth = ref(selectedDay.value.getMonth())
const selectedHour = ref<number | null>(null)
const selectedMinute = ref<number | null>(null)
const selectedSecond = ref<number | null>(null)

const weekDays = ['一', '二', '三', '四', '五', '六', '日']

const displayText = computed(() => {
    if (!props.modelValue) return props.placeholder
    return formatDateTime(props.modelValue)
})

const canGoPrevMonth = computed(() => {
    const min = minDateTime()
    return monthKey(viewYear.value, viewMonth.value) > monthKey(min.getFullYear(), min.getMonth())
})

const canGoNextMonth = computed(() => {
    const max = maxDateTime()
    return monthKey(viewYear.value, viewMonth.value) < monthKey(max.getFullYear(), max.getMonth())
})

const dayCells = computed(() => {
    const year = viewYear.value
    const month = viewMonth.value
    const first = new Date(year, month, 1)
    const firstWeekDay = (first.getDay() + 6) % 7
    const daysInMonth = new Date(year, month + 1, 0).getDate()

    const cells: Array<{ key: string; day: number; date: Date; selectable: boolean }> = []

    for (let i = 0; i < firstWeekDay; i++) {
        cells.push({
            key: `pad-${i}`,
            day: 0,
            date: new Date(year, month, 1 - (firstWeekDay - i)),
            selectable: false
        })
    }

    for (let day = 1; day <= daysInMonth; day++) {
        const current = new Date(year, month, day)
        const selectable = hasAnyTimeOption(current)
        cells.push({
            key: `${year}-${month + 1}-${day}`,
            day,
            date: current,
            selectable
        })
    }

    while (cells.length % 7 !== 0) {
        const idx = cells.length
        cells.push({
            key: `tail-${idx}`,
            day: 0,
            date: new Date(year, month + 1, idx),
            selectable: false
        })
    }

    return cells
})

const hasAnyTime = computed(() => hasAnyTimeOption(selectedDay.value))

const hourOptions = computed(() => {
    const range = getDayRange(selectedDay.value)
    if (!range) return []
    const result: number[] = []

    for (let h = 0; h < 24; h++) {
        const from = new Date(
            selectedDay.value.getFullYear(),
            selectedDay.value.getMonth(),
            selectedDay.value.getDate(),
            h,
            0,
            0
        )
        const to = new Date(
            selectedDay.value.getFullYear(),
            selectedDay.value.getMonth(),
            selectedDay.value.getDate(),
            h,
            59,
            59
        )

        if (from <= range.end && to >= range.start) {
            result.push(h)
        }
    }

    return result
})

const minuteOptions = computed(() => {
    if (selectedHour.value === null) return []
    const range = getDayRange(selectedDay.value)
    if (!range) return []
    const result: number[] = []

    for (let m = 0; m < 60; m++) {
        const from = new Date(
            selectedDay.value.getFullYear(),
            selectedDay.value.getMonth(),
            selectedDay.value.getDate(),
            selectedHour.value,
            m,
            0
        )
        const to = new Date(
            selectedDay.value.getFullYear(),
            selectedDay.value.getMonth(),
            selectedDay.value.getDate(),
            selectedHour.value,
            m,
            59
        )
        if (from <= range.end && to >= range.start) {
            result.push(m)
        }
    }

    return result
})

const secondOptions = computed(() => {
    if (selectedHour.value === null || selectedMinute.value === null) return []
    const range = getDayRange(selectedDay.value)
    if (!range) return []
    const result: number[] = []

    for (let s = 0; s < 60; s++) {
        const current = new Date(
            selectedDay.value.getFullYear(),
            selectedDay.value.getMonth(),
            selectedDay.value.getDate(),
            selectedHour.value,
            selectedMinute.value,
            s
        )
        if (current >= range.start && current <= range.end) {
            result.push(s)
        }
    }

    return result
})

const canConfirm = computed(() => {
    return (
        selectedHour.value !== null &&
        selectedMinute.value !== null &&
        selectedSecond.value !== null
    )
})

watch(
    () => props.modelValue,
    value => {
        if (panelVisible.value) return
        if (!value) return
        if (!isWithinRange(value)) return

        const day = stripTime(value)
        selectedDay.value = day
        viewYear.value = day.getFullYear()
        viewMonth.value = day.getMonth()
        selectedHour.value = value.getHours()
        selectedMinute.value = value.getMinutes()
        selectedSecond.value = value.getSeconds()
    },
    { immediate: true }
)

watch(selectedDay, day => {
    viewYear.value = day.getFullYear()
    viewMonth.value = day.getMonth()
    ensureValidSelections()
})

watch(hourOptions, options => {
    if (!options.length) {
        selectedHour.value = null
        return
    }
    if (selectedHour.value === null || !options.includes(selectedHour.value)) {
        selectedHour.value = options[0]
    }
})

watch(minuteOptions, options => {
    if (!options.length) {
        selectedMinute.value = null
        return
    }
    if (selectedMinute.value === null || !options.includes(selectedMinute.value)) {
        selectedMinute.value = options[0]
    }
})

watch(secondOptions, options => {
    if (!options.length) {
        selectedSecond.value = null
        return
    }
    if (selectedSecond.value === null || !options.includes(selectedSecond.value)) {
        selectedSecond.value = options[0]
    }
})

const togglePanel = () => {
    if (props.disabled) return

    if (!panelVisible.value) {
        initializeDraftFromModel()
        panelVisible.value = true
        return
    }

    panelVisible.value = false
}

const goPrevMonth = () => {
    if (!canGoPrevMonth.value) return
    const month = viewMonth.value - 1
    if (month < 0) {
        viewMonth.value = 11
        viewYear.value -= 1
        return
    }
    viewMonth.value = month
}

const goNextMonth = () => {
    if (!canGoNextMonth.value) return
    const month = viewMonth.value + 1
    if (month > 11) {
        viewMonth.value = 0
        viewYear.value += 1
        return
    }
    viewMonth.value = month
}

const selectDay = (date: Date) => {
    selectedDay.value = stripTime(date)
}

const selectHour = (hour: number) => {
    selectedHour.value = hour
}

const selectMinute = (minute: number) => {
    selectedMinute.value = minute
}

const selectSecond = (second: number) => {
    selectedSecond.value = second
}

const handleCancel = () => {
    panelVisible.value = false
    initializeDraftFromModel()
}

const handleClear = () => {
    emit('update:modelValue', null)
    panelVisible.value = false
    initializeDraftFromModel()
}

const handleConfirm = () => {
    if (!canConfirm.value) return

    const value = buildSelectedDateTime()
    if (!value) return

    emit('update:modelValue', value)
    panelVisible.value = false
}

const onDocumentClick = (event: MouseEvent) => {
    const root = rootRef.value
    if (!root) return
    if (!root.contains(event.target as Node)) {
        initializeDraftFromModel()
        panelVisible.value = false
    }
}

onMounted(() => {
    document.addEventListener('click', onDocumentClick)
})

onUnmounted(() => {
    document.removeEventListener('click', onDocumentClick)
})

function hasAnyTimeOption(date: Date): boolean {
    return getDayRange(date) !== null
}

function isWithinRange(date: Date): boolean {
    return date >= minDateTime() && date <= maxDateTime()
}

function stripTime(date: Date): Date {
    return new Date(date.getFullYear(), date.getMonth(), date.getDate())
}

function pad2(n: number): string {
    return n.toString().padStart(2, '0')
}

function formatDateTime(date: Date): string {
    return (
        `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())} ` +
        `${pad2(date.getHours())}:${pad2(date.getMinutes())}:${pad2(date.getSeconds())}`
    )
}

function maxDate(a: Date, b: Date): Date {
    return a > b ? a : b
}

function minDate(a: Date, b: Date): Date {
    return a < b ? a : b
}

function monthKey(year: number, month: number): number {
    return year * 12 + month
}

function isSameDay(a: Date, b: Date): boolean {
    return (
        a.getFullYear() === b.getFullYear() &&
        a.getMonth() === b.getMonth() &&
        a.getDate() === b.getDate()
    )
}

function getDayRange(date: Date): { start: Date; end: Date } | null {
    const dayStart = new Date(date.getFullYear(), date.getMonth(), date.getDate(), 0, 0, 0)
    const dayEnd = new Date(date.getFullYear(), date.getMonth(), date.getDate(), 23, 59, 59)
    const start = maxDate(dayStart, minDateTime())
    const end = minDate(dayEnd, maxDateTime())
    if (start > end) return null
    return { start, end }
}

function ensureValidSelections() {
    const range = getDayRange(selectedDay.value)
    if (!range) {
        selectedHour.value = null
        selectedMinute.value = null
        selectedSecond.value = null
        return
    }

    const preferred =
        props.modelValue && isSameDay(stripTime(props.modelValue), selectedDay.value)
            ? props.modelValue
            : range.start

    const bounded = new Date(
        Math.min(Math.max(preferred.getTime(), range.start.getTime()), range.end.getTime())
    )
    selectedHour.value = bounded.getHours()
    selectedMinute.value = bounded.getMinutes()
    selectedSecond.value = bounded.getSeconds()
}

function buildSelectedDateTime(): Date | null {
    if (
        selectedHour.value === null ||
        selectedMinute.value === null ||
        selectedSecond.value === null
    ) {
        return null
    }

    const value = new Date(
        selectedDay.value.getFullYear(),
        selectedDay.value.getMonth(),
        selectedDay.value.getDate(),
        selectedHour.value,
        selectedMinute.value,
        selectedSecond.value
    )

    return value
}

function initializeDraftFromModel() {
    const source =
        props.modelValue && isWithinRange(props.modelValue) ? props.modelValue : minDateTime()
    const day = stripTime(source)

    selectedDay.value = day
    viewYear.value = day.getFullYear()
    viewMonth.value = day.getMonth()
    selectedHour.value = source.getHours()
    selectedMinute.value = source.getMinutes()
    selectedSecond.value = source.getSeconds()
    ensureValidSelections()
}
</script>

<style scoped>
.data-picker {
    position: relative;
    width: 100%;
}

.picker-trigger {
    width: 100%;
    min-height: 32px;
    border: 1px solid #dcdfe6;
    border-radius: 4px;
    background: #ffffff;
    color: #303133;
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    cursor: pointer;
    transition: border-color 0.2s;
}

.picker-trigger:hover {
    border-color: #c0c4cc;
}

.picker-trigger:focus-visible {
    outline: none;
    border-color: #409eff;
}

.picker-trigger.empty {
    color: #a8abb2;
}

.picker-trigger.disabled {
    background: #f5f7fa;
    border-color: #e4e7ed;
    color: #c0c4cc;
    cursor: not-allowed;
}

.picker-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.picker-icon {
    color: #a8abb2;
    margin-left: 8px;
    font-size: 12px;
}

.picker-panel {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 2200;
    width: max-content;
    max-width: calc(100vw - 48px);
    border: 1px solid #e4e7ed;
    border-radius: 4px;
    background: #ffffff;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.12);
    display: grid;
    grid-template-columns: max-content 180px;
    grid-template-rows: auto auto;
    overflow: hidden;
}

.panel-date {
    padding: 10px;
    border-right: 1px solid #f0f2f5;
}

.month-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
}

.month-btn {
    width: 24px;
    height: 24px;
    border: none;
    background: transparent;
    color: #606266;
    cursor: pointer;
    border-radius: 4px;
}

.month-btn:hover:not(:disabled) {
    background: #f2f6fc;
    color: #409eff;
}

.month-btn:disabled {
    color: #c0c4cc;
    cursor: not-allowed;
}

.month-title {
    font-size: 14px;
    font-weight: 500;
    color: #303133;
}

.week-row {
    display: grid;
    grid-template-columns: repeat(7, 26px);
    width: max-content;
    gap: 2px;
    margin-bottom: 6px;
}

.week-row span {
    text-align: center;
    color: #909399;
    font-size: 12px;
    line-height: 28px;
}

.date-grid {
    display: grid;
    grid-template-columns: repeat(7, 26px);
    width: max-content;
    gap: 2px;
}

.date-cell {
    width: 26px;
    height: 32px;
    border: none;
    background: transparent;
    color: #606266;
    cursor: pointer;
    border-radius: 4px;
    font-size: 13px;
}

.date-cell:hover:not(:disabled) {
    background: #f2f6fc;
    color: #409eff;
}

.date-cell.selected {
    background: #409eff;
    color: #ffffff;
}

.date-cell.hidden {
    visibility: hidden;
    cursor: default;
}

.panel-time {
    padding: 10px;
    display: flex;
    flex-direction: column;
}

.panel-footer {
    grid-column: 1 / -1;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-top: 1px solid #f0f2f5;
    background: #ffffff;
}

.footer-btn {
    min-width: 56px;
    height: 28px;
    border-radius: 4px;
    border: 1px solid #dcdfe6;
    background: #ffffff;
    color: #606266;
    font-size: 12px;
    cursor: pointer;
}

.btn-clear {
    margin-right: auto;
}

.footer-btn:hover {
    border-color: #c6e2ff;
    color: #409eff;
}

.btn-confirm {
    background: #409eff;
    border-color: #409eff;
    color: #ffffff;
}

.btn-confirm:hover {
    background: #66b1ff;
    border-color: #66b1ff;
    color: #ffffff;
}

.btn-confirm:disabled {
    background: #a0cfff;
    border-color: #a0cfff;
    color: #ffffff;
    cursor: not-allowed;
}

.time-columns {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
}

.time-col {
    min-width: 0;
}

.time-col-list {
    overflow-y: auto;
    max-height: 240px;
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.time-item {
    border: none;
    background: transparent;
    text-align: center;
    padding: 6px 8px;
    color: #606266;
    border-radius: 4px;
    cursor: pointer;
}

.time-item:hover {
    background: #f2f6fc;
    color: #409eff;
}

.time-item.selected {
    background: #ecf5ff;
    color: #409eff;
    font-weight: 600;
}

.time-empty {
    color: #909399;
    font-size: 12px;
    padding: 8px 4px;
}
</style>
