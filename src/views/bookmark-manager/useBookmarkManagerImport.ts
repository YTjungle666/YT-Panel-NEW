import type { Ref } from 'vue'
import { addMultiple as addMultipleBookmarks } from '@/api/panel/bookmark'
import { t } from '@/locales'
import {
  flattenBookmarkTree,
  parseBrowserBookmarkHTML,
} from '@/utils/bookmarkImportExport'
import { resolveApiErrorMessage } from '@/utils/request/apiMessage'
import { ss } from '@/utils/storage/local'
import { BOOKMARKS_CACHE_KEY } from './bookmarkTreeState'

interface BookmarkManagerMessageApi {
  error: (content: string) => void
  success: (content: string) => void
  warning: (content: string) => void
}

interface UseBookmarkManagerImportOptions {
  fileInput: Ref<HTMLInputElement | undefined>
  jsonData: Ref<string | null>
  message: BookmarkManagerMessageApi
  refreshBookmarks: (forceRefresh?: boolean) => Promise<void>
  uploadLoading: Ref<boolean>
}

export function useBookmarkManagerImport(options: UseBookmarkManagerImportOptions) {
  const { fileInput, jsonData, message, refreshBookmarks, uploadLoading } = options

  function triggerImportBookmarks() {
    fileInput.value?.click()
  }

  function handleFileChange(event: Event) {
    const target = event.target as HTMLInputElement
    const file = target.files?.[0]
    if (file) {
      uploadLoading.value = true
      const reader = new FileReader()
      reader.onload = (readerEvent) => {
        if (readerEvent.target?.result) {
          jsonData.value = readerEvent.target.result as string
          importCheck(file.name)
        } else {
          message.error(`${t('common.failed')}: ${t('common.repeatLater')}`)
          uploadLoading.value = false
        }
      }
      reader.onerror = () => {
        message.error(`${t('common.failed')}: ${t('common.fileReadError')}`)
        uploadLoading.value = false
      }
      reader.readAsText(file)
    } else {
      uploadLoading.value = false
    }

    target.value = ''
  }

  function importCheck(fileName: string) {
    try {
      if (fileName.endsWith('.html')) {
        void importBookmarksToServerWithHTML(jsonData.value!)
      } else {
        message.error(t('bookmarkManager.onlySupportHtml'))
      }
    } catch (error) {
      message.error(`${t('common.failed')}: ${(error as Error).message || t('common.unknownError')}`)
    } finally {
      uploadLoading.value = false
    }
  }

  async function importBookmarksToServerWithHTML(htmlContent: string) {
    uploadLoading.value = true
    try {
      const bookmarkTree = parseBrowserBookmarkHTML(htmlContent)
      const sortCounter = new Map<number, number>()
      const bookmarkList = flattenBookmarkTree(bookmarkTree).map((item) => {
        const isFolder = item.isFolder ? 1 : 0
        const tempId = 'id' in item ? Number(item.id || 0) : 0
        const parentTempId = 'folderId' in item ? Number(item.folderId || 0) : 0
        const nextSort = (sortCounter.get(parentTempId) || 0) + 1
        sortCounter.set(parentTempId, nextSort)
        return {
          title: item.title,
          url: 'url' in item ? item.url || '' : '',
          lanUrl: '',
          parentUrl: '',
          parentId: 'folderId' in item ? item.folderId || 0 : 0,
          tempId,
          parentTempId,
          isFolder,
          iconJson: 'icon' in item ? item.icon || '' : '',
          sort: nextSort,
        }
      })

      if (bookmarkList.length === 0) {
        message.warning(t('bookmarkManager.noData'))
        return
      }

      const response = await addMultipleBookmarks(bookmarkList)
      if (response.code === 0) {
        ss.remove(BOOKMARKS_CACHE_KEY)
        await refreshBookmarks(true)
        message.success(t('bookmarkManager.importSuccess', { count: bookmarkList.length }))
      } else {
        message.error(resolveApiErrorMessage(response))
      }
    } catch (error) {
      message.error((error as Error).message || t('bookmarkManager.unknownError'))
    } finally {
      uploadLoading.value = false
    }
  }

  return {
    handleFileChange,
    triggerImportBookmarks,
  }
}
