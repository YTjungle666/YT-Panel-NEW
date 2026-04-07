import { ref } from 'vue'
import { useMessage } from 'naive-ui'
import { update } from '@/api/panel/bookmark'
import { t } from '@/locales'
import { logError } from '@/utils/logger'
import { ss } from '@/utils/storage/local'
import { BOOKMARKS_CACHE_KEY } from './bookmarkTreeState'
import {
  checkBookmarkTreeDescendant,
  serializeBookmarkCacheNode,
  sortBookmarkTreeChildren,
  updateBookmarkLocalSort,
} from './bookmarkManagerDragDropUtils'
import type { BookmarkManagerData, BookmarkManagerDragItem } from './bookmarkManagerUiTypes'

export function useBookmarkManagerDragDrop(data: BookmarkManagerData) {
  const ms = useMessage()
  const draggedItem = ref<BookmarkManagerDragItem | null>(null)
  const dragOverTarget = ref<string | number | null>(null)
  const dragInsertPosition = ref<'before' | 'after' | null>(null)
  const dragIndicatorTop = ref<number | null>(null)

  function handleDragStart(event: DragEvent, item: BookmarkManagerDragItem) {
    draggedItem.value = item
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'all'
      event.dataTransfer.dropEffect = 'move'
      event.dataTransfer.setData('text/plain', item.id.toString())
    }

    if (event.currentTarget instanceof HTMLElement) {
      const element = event.currentTarget
      setTimeout(() => {
        element.classList.add('opacity-50')
      }, 0)
    }
  }

  function handleDragEnd(event: DragEvent) {
    if (event.currentTarget instanceof HTMLElement)
      event.currentTarget.classList.remove('opacity-50')

    draggedItem.value = null
  }

  function resetDragState() {
    dragOverTarget.value = null
    dragInsertPosition.value = null
    dragIndicatorTop.value = null
  }

  function handleDragOver(event: DragEvent, item?: BookmarkManagerDragItem) {
    event.preventDefault()
    if (event.dataTransfer)
      event.dataTransfer.dropEffect = 'move'

    if (!item || !event.currentTarget) {
      resetDragState()
      return
    }

    const wrapper = event.currentTarget as HTMLElement
    const rect = wrapper.getBoundingClientRect()
    const mousePosition = event.clientY - rect.top
    const itemHeight = rect.height

    if (item.isFolder) {
      if (mousePosition < itemHeight / 3) {
        dragOverTarget.value = item.id
        dragInsertPosition.value = 'before'
        dragIndicatorTop.value = wrapper.offsetTop
      } else if (mousePosition > (2 * itemHeight) / 3) {
        dragOverTarget.value = item.id
        dragInsertPosition.value = 'after'
        dragIndicatorTop.value = wrapper.offsetTop + wrapper.offsetHeight
      } else {
        dragOverTarget.value = item.id
        dragInsertPosition.value = null
        dragIndicatorTop.value = null
      }
    } else {
      dragOverTarget.value = item.id
      if (mousePosition < itemHeight / 2) {
        dragInsertPosition.value = 'before'
        dragIndicatorTop.value = wrapper.offsetTop
      } else {
        dragInsertPosition.value = 'after'
        dragIndicatorTop.value = wrapper.offsetTop + wrapper.offsetHeight
      }
    }
  }

  function handleContainerDragOver(event: DragEvent) {
    event.preventDefault()
    if (event.dataTransfer)
      event.dataTransfer.dropEffect = 'move'
  }

  function handleDragLeave(event: DragEvent) {
    event.preventDefault()
  }

  async function handleDrop(event: DragEvent, targetItem: BookmarkManagerDragItem) {
    event.preventDefault()

    if (event.currentTarget instanceof HTMLElement)
      event.currentTarget.classList.remove('opacity-50')

    if (!draggedItem.value || draggedItem.value.id === targetItem.id) {
      draggedItem.value = null
      return
    }

    const draggedItemData = draggedItem.value
    const isTargetFolder = targetItem.isFolder

    if (isTargetFolder && !dragInsertPosition.value) {
      if (draggedItemData.isFolder && checkBookmarkTreeDescendant(data.fullData.value, draggedItemData.id, targetItem.id)) {
        ms.warning('不能将文件夹移动到自己的子文件夹中')
        draggedItem.value = null
        return
      }

      const targetFolderId = String(targetItem.id)
      const draggedParentId = String(draggedItemData.folderId || '0')
      if (draggedParentId === targetFolderId) {
        draggedItem.value = null
        return
      }

      const targetFolderItems = data.allItems.value.filter(item => String(item.folderId || '0') === targetFolderId)
      const maxSort = targetFolderItems.length > 0 ? Math.max(...targetFolderItems.map(item => item.sort || 0)) : 0
      const updateData = {
        id: Number(draggedItemData.id),
        title: draggedItemData.title,
        url: draggedItemData.isFolder ? draggedItemData.title : (draggedItemData.url || ''),
        parentId: Number(targetFolderId),
        sort: maxSort + 1,
        lanUrl: draggedItemData.lanUrl || '',
        openMethod: draggedItemData.openMethod || 0,
        icon: draggedItemData.icon || null,
        iconJson: draggedItemData.iconJson || '',
      }

      try {
        const response = await update(updateData)
        if (response?.code === 0)
          data.updateCacheAfterUpdate(response.data)
      } catch (error) {
        logError('移动书签失败', error)
        ms.error(`${t('bookmarkManager.moveFailed') || '移动失败'}: ${(error as Error).message || ''}`)
      }

      draggedItem.value = null
      return
    }

    const draggedFolderId = String(draggedItemData.folderId || '0')
    const targetFolderId = String(targetItem.folderId || '0')

    if (isTargetFolder && draggedFolderId !== targetFolderId) {
      const targetFolderIdValue = String(targetItem.id)
      if (draggedItemData.isFolder && checkBookmarkTreeDescendant(data.fullData.value, draggedItemData.id, targetItem.id)) {
        ms.warning('不能将文件夹移动到自己的子文件夹中')
        draggedItem.value = null
        return
      }

      const targetFolderItems = data.allItems.value.filter(item => String(item.folderId || '0') === targetFolderIdValue)
      const maxSort = targetFolderItems.length > 0 ? Math.max(...targetFolderItems.map(item => item.sort || 0)) : 0
      const updateData = {
        id: Number(draggedItemData.id),
        title: draggedItemData.title,
        url: draggedItemData.isFolder ? draggedItemData.title : (draggedItemData.url || ''),
        parentId: Number(targetFolderIdValue),
        sort: maxSort + 1,
        lanUrl: draggedItemData.lanUrl || '',
        openMethod: draggedItemData.openMethod || 0,
        icon: draggedItemData.icon || null,
        iconJson: draggedItemData.iconJson || '',
      }

      try {
        const response = await update(updateData)
        if (response?.code === 0)
          data.updateCacheAfterUpdate(response.data)
      } catch (error) {
        logError('移动书签失败', error)
        ms.error(`${t('bookmarkManager.moveFailed') || '移动失败'}: ${(error as Error).message || ''}`)
      }

      draggedItem.value = null
      return
    }

    if (draggedFolderId !== targetFolderId) {
      ms.warning('只能在同一文件夹内拖拽排序')
      draggedItem.value = null
      return
    }

    const currentFolderItems = data.allItems.value.filter(item => String(item.folderId || '0') === draggedFolderId)
    const draggedIndex = currentFolderItems.findIndex(item => String(item.id) === String(draggedItemData.id))
    const targetIndex = currentFolderItems.findIndex(item => String(item.id) === String(targetItem.id))

    if (draggedIndex === -1 || targetIndex === -1) {
      ms.warning('排序更新失败：找不到相关项目')
      draggedItem.value = null
      return
    }

    const sortedFolderItems = [...currentFolderItems].sort((a, b) => (a.sort || 0) - (b.sort || 0))
    const sortValues = sortedFolderItems.map(item => item.sort || 0)
    const hasDuplicates = sortValues.some((value, index) => sortValues.indexOf(value) !== index)

    if (hasDuplicates) {
      const normalizedItems = sortedFolderItems.map((item, index) => ({
        id: Number(item.id),
        title: item.title,
        url: item.isFolder ? item.title : (item.url || ''),
        parentId: Number(draggedFolderId),
        sort: index + 1,
        lanUrl: (item as any).lanUrl || '',
        openMethod: (item as any).openMethod || 0,
        icon: (item as any).icon || null,
        iconJson: item.iconJson || '',
      }))

      for (const normalizedItem of normalizedItems) {
        const originalItem = sortedFolderItems.find(item => Number(item.id) === normalizedItem.id)
        if (originalItem)
          originalItem.sort = normalizedItem.sort
      }

      Promise.all(normalizedItems.map(item => update(item))).catch(error => logError('Sort值规范化同步失败', error))
    }

    const sortedDraggedIndex = sortedFolderItems.findIndex(item => String(item.id) === String(draggedItemData.id))
    const sortedTargetIndex = sortedFolderItems.findIndex(item => String(item.id) === String(targetItem.id))
    let newIndex = dragInsertPosition.value === 'before' ? sortedTargetIndex : sortedTargetIndex + 1
    if (!dragInsertPosition.value)
      newIndex = draggedIndex < targetIndex ? targetIndex : targetIndex + 1

    const updatedItems = [...sortedFolderItems]
    const draggedSortedItem = sortedFolderItems[sortedDraggedIndex]
    updatedItems.splice(sortedDraggedIndex, 1)
    const adjustedNewIndex = newIndex > sortedDraggedIndex ? newIndex - 1 : newIndex
    updatedItems.splice(adjustedNewIndex, 0, draggedSortedItem)
    const startUpdateIndex = Math.min(adjustedNewIndex, sortedDraggedIndex)
    const endUpdateIndex = Math.max(adjustedNewIndex, sortedDraggedIndex)

    const itemsToUpdate = updatedItems.slice(startUpdateIndex, endUpdateIndex + 1).map((item, offset) => ({
      id: Number(item.id),
      title: item.title,
      url: item.isFolder ? item.title : (item.url || ''),
      parentId: Number(draggedFolderId),
      sort: startUpdateIndex + 1 + offset,
      lanUrl: (item as any).lanUrl || '',
      openMethod: (item as any).openMethod || 0,
      icon: (item as any).icon || null,
      iconJson: item.iconJson || '',
      }))

    try {
      for (const item of itemsToUpdate)
        updateBookmarkLocalSort(data.fullData, data.bookmarkTree, Number(item.id), item.sort)

      data.fullData.value.sort((a, b) => (a.sort || 0) - (b.sort || 0))
      sortBookmarkTreeChildren(data.fullData.value)
      sortBookmarkTreeChildren(data.bookmarkTree.value)
      data.fullData.value = [...data.fullData.value]
      data.bookmarkTree.value = [...data.bookmarkTree.value]

      ss.set(BOOKMARKS_CACHE_KEY, data.fullData.value.map(serializeBookmarkCacheNode))
      Promise.all(itemsToUpdate.map(item => update(item))).catch((error) => {
        logError('服务器同步失败,回滚本地数据', error)
        ms.error('排序保存失败,已恢复')
        void data.refreshBookmarks(true)
      })
    } catch (error) {
      logError('本地更新失败', error)
      ms.error('排序更新失败')
    }

    draggedItem.value = null
  }

  return {
    dragIndicatorTop,
    dragInsertPosition,
    dragOverTarget,
    handleContainerDragOver,
    handleDragEnd,
    handleDragLeave,
    handleDragOver,
    handleDragStart,
    handleDrop,
    resetDragState,
  }
}
