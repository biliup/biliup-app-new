import { nextTick, type ComputedRef, type Ref } from 'vue'
import Sortable, { type SortableEvent } from 'sortablejs'

interface UseUserOrderSortableOptions {
    containerRef: Ref<HTMLElement | null>
    enabled: ComputedRef<boolean>
    getCurrentOrder: () => number[]
    onOrderChange: (nextOrder: number[]) => Promise<void>
    sortable: {
        draggable: string
        handle: string
        animation?: number
        chosenClass?: string
        dragClass?: string
        ghostClass?: string
        fallbackClass?: string
        forceFallback?: boolean
        fallbackOnBody?: boolean
        fallbackTolerance?: number
    }
}

export const useUserOrderSortable = (options: UseUserOrderSortableOptions) => {
    let sortable: Sortable | null = null

    const destroySortable = () => {
        if (!sortable) {
            return
        }

        sortable.destroy()
        sortable = null
    }

    const syncSortable = async () => {
        await nextTick()

        const container = options.containerRef.value
        if (!container || !options.enabled.value) {
            destroySortable()
            return
        }

        destroySortable()

        sortable = Sortable.create(container, {
            animation: options.sortable.animation ?? 180,
            draggable: options.sortable.draggable,
            handle: options.sortable.handle,
            chosenClass: options.sortable.chosenClass,
            dragClass: options.sortable.dragClass,
            ghostClass: options.sortable.ghostClass,
            fallbackClass: options.sortable.fallbackClass,
            forceFallback: options.sortable.forceFallback,
            fallbackOnBody: options.sortable.fallbackOnBody,
            fallbackTolerance: options.sortable.fallbackTolerance,
            onEnd: async (event: SortableEvent) => {
                const fromIndex = event.oldDraggableIndex ?? event.oldIndex
                const toIndex = event.newDraggableIndex ?? event.newIndex

                if (
                    typeof fromIndex !== 'number' ||
                    typeof toIndex !== 'number' ||
                    fromIndex === toIndex
                ) {
                    return
                }

                const nextOrder = options.getCurrentOrder()
                const [movedUid] = nextOrder.splice(fromIndex, 1)

                if (typeof movedUid !== 'number') {
                    return
                }

                nextOrder.splice(toIndex, 0, movedUid)
                await options.onOrderChange(nextOrder)
            }
        })
    }

    return {
        syncSortable,
        destroySortable
    }
}
