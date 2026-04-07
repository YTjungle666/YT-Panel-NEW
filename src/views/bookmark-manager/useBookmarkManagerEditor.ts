import type { Ref } from 'vue'
import { add as addBookmark, deletes, update } from '@/api/panel/bookmark'
import { t } from '@/locales'
import { logError } from '@/utils/logger'
import { dialog } from '@/utils/request/apiMessage'
import type { Bookmark } from './types'

interface BookmarkManagerMessageApi {
  error: (content: string) => void
}

interface BookmarkEditForm {
  id: number
  title: string
  url: string
  folderId?: string | number
  iconJson?: string
}

interface UseBookmarkManagerEditorOptions {
  bookmarkType: Ref<'bookmark' | 'folder'>
  currentBookmark: Ref<(Bookmark & { isFolder?: boolean }) | null>
  currentEditBookmark: Ref<BookmarkEditForm>
  isCreateMode: Ref<boolean>
  isEditDialogOpen: Ref<boolean>
  message: BookmarkManagerMessageApi
  resolveBookmarkIcon: (url: string) => Promise<string>
  selectedBookmarkId: Ref<string>
  selectedFolder: Ref<string>
  updateCacheAfterAdd: (bookmark: any) => void
  updateCacheAfterDelete: (bookmarkId: number) => void
  updateCacheAfterUpdate: (bookmark: any) => void
}

export function useBookmarkManagerEditor(options: UseBookmarkManagerEditorOptions) {
  const {
    bookmarkType,
    currentBookmark,
    currentEditBookmark,
    isCreateMode,
    isEditDialogOpen,
    message,
    resolveBookmarkIcon,
    selectedBookmarkId,
    selectedFolder,
    updateCacheAfterAdd,
    updateCacheAfterDelete,
    updateCacheAfterUpdate,
  } = options

  function handleEditBookmark() {
    if (currentBookmark.value) {
      currentEditBookmark.value = {
        ...currentBookmark.value,
      }
      bookmarkType.value = currentBookmark.value.isFolder ? 'folder' : 'bookmark'
      isCreateMode.value = false
      isEditDialogOpen.value = true
    }
  }

  function createNewBookmark() {
    const defaultFolderId = selectedFolder.value && selectedFolder.value !== '0' ? selectedFolder.value : '0'
    currentEditBookmark.value = {
      id: 0,
      title: '',
      url: '',
      folderId: defaultFolderId,
    }
    isCreateMode.value = true
    isEditDialogOpen.value = true
  }

  function closeEditDialog() {
    isEditDialogOpen.value = false
  }

  async function saveBookmarkChanges() {
    try {
      if (!currentEditBookmark.value.title.trim()) {
        message.error(t('bookmarkManager.titleRequired'))
        return
      }

      const isFolderItem = bookmarkType.value === 'folder'
      if (!isFolderItem && !currentEditBookmark.value.url.trim()) {
        message.error(t('bookmarkManager.urlRequired'))
        return
      }

      if (!isFolderItem && currentEditBookmark.value.url.trim()) {
        let url = currentEditBookmark.value.url.trim()
        if (!url.startsWith('http://') && !url.startsWith('https://')) {
          url = `https://${url}`
          currentEditBookmark.value.url = url
        }
      }

      if (!isFolderItem && currentEditBookmark.value.url.trim()) {
        try {
          const iconJson = await resolveBookmarkIcon(currentEditBookmark.value.url)
          if (iconJson)
            currentEditBookmark.value.iconJson = iconJson
        } catch (error) {
          logError('获取书签图标失败', error)
        }
      }

      if (isCreateMode.value) {
        const parentId = currentEditBookmark.value.folderId ? Number(currentEditBookmark.value.folderId) : 0
        const createData: any = {
          title: currentEditBookmark.value.title,
          url: '',
          parentId,
          sort: 9999,
          lanUrl: '',
          icon: null,
          openMethod: 0,
          iconJson: currentEditBookmark.value.iconJson || '',
        }

        if (bookmarkType.value === 'folder') {
          Object.assign(createData, {
            isFolder: 1,
            url: currentEditBookmark.value.title,
          })
        } else {
          Object.assign(createData, {
            isFolder: 0,
            url: currentEditBookmark.value.url,
          })
        }

        const createResponse = await addBookmark(createData)
        if (createResponse?.code === 0) {
          updateCacheAfterAdd(createResponse.data)
          isEditDialogOpen.value = false
          isCreateMode.value = false
        } else {
          message.error(`${t('bookmarkManager.createFailed')} ${createResponse?.msg || t('bookmarkManager.unknownError')}`)
        }
      } else {
        const isFolderItem = currentBookmark.value?.isFolder
        const selectedFolderId = currentEditBookmark.value.folderId ? currentEditBookmark.value.folderId.toString() : '0'
        const updateData: any = {
          id: Number(currentEditBookmark.value.id),
          title: currentEditBookmark.value.title,
          parentId: selectedFolderId !== '0' ? Number(selectedFolderId) : 0,
          sort: 9999,
          lanUrl: '',
          icon: null,
          openMethod: 0,
          url: isFolderItem ? currentEditBookmark.value.title : currentEditBookmark.value.url,
          iconJson: currentEditBookmark.value.iconJson || '',
        }

        const updateResponse = await update(updateData)
        if (updateResponse?.code === 0) {
          updateCacheAfterUpdate(updateResponse.data)
          isEditDialogOpen.value = false
        } else {
          message.error(`${t('bookmarkManager.updateFailed')} ${updateResponse?.msg || t('bookmarkManager.unknownError')}`)
        }
      }
    } catch (error) {
      logError('保存书签失败', error)
      message.error(`${t('bookmarkManager.updateFailed')} ${(error as Error).message || t('common.networkError')}`)
    }
  }

  async function deleteBookmark(bookmark: Bookmark) {
    const confirmMessage = bookmark.isFolder
      ? t('bookmarkManager.deleteFolderConfirm').replace('name', `【${bookmark.title}】`)
      : t('bookmarkManager.deleteBookmarkConfirm').replace('name', `【${bookmark.title}】`)

    dialog.warning({
      title: t('bookmarkManager.confirmDelete'),
      content: confirmMessage,
      positiveText: t('bookmarkManager.confirm'),
      negativeText: t('bookmarkManager.cancel'),
      onPositiveClick: async () => {
        try {
          const response = await deletes([Number(bookmark.id)])
          if (response.code === 0) {
            if (selectedBookmarkId.value === bookmark.id.toString())
              selectedBookmarkId.value = ''

            if (bookmark.isFolder && selectedFolder.value === bookmark.id.toString())
              selectedFolder.value = '0'

            updateCacheAfterDelete(Number(bookmark.id))
          } else {
            message.error(`${t('common.failed')}: ${response.msg}`)
          }
        } catch (error) {
          message.error(`${t('common.failed')} ${(error as Error).message || t('bookmarkManager.unknownError')}`)
        }
      },
    })
  }

  function handleDeleteBookmark() {
    if (currentBookmark.value)
      void deleteBookmark(currentBookmark.value)
  }

  return {
    closeEditDialog,
    createNewBookmark,
    handleDeleteBookmark,
    handleEditBookmark,
    saveBookmarkChanges,
  }
}
