import { computed, ref, watch } from 'vue'
import { useMessage } from 'naive-ui'
import { update } from '@/api/panel/bookmark'
import { getSiteFavicon } from '@/api/panel/itemIcon'
import { t } from '@/locales'
import { exportBookmarkJson } from '@/utils/bookmarkImportExport'
import { DEFAULT_BOOKMARK_ICON, getBookmarkIconSrc } from '@/utils/bookmarkIcon'
import { logError } from '@/utils/logger'
import { buildFolderTreeOptions, toExportBookmarkTree } from './bookmarkTreeState'
import { useBookmarkManagerCache } from './bookmarkManagerCache'
import { useBookmarkManagerEditor } from './useBookmarkManagerEditor'
import { useBookmarkManagerImport } from './useBookmarkManagerImport'
import type { Bookmark, TreeOption } from './types'

export function useBookmarkManagerData() {
  const refreshingBookmarkIconIds = new Set<number>()
  const ms = useMessage()
  const fileInput = ref<HTMLInputElement>()
  const bookmarkTree = ref<TreeOption[]>([])
  const defaultExpandedKeys = ref<string[]>([])
  const selectedFolder = ref('0')
  const fullData = ref<TreeOption[]>([])
  const searchQuery = ref('')
  const selectedBookmarkId = ref('')
  const focusedItemId = ref('')
  const currentBookmark = ref<(Bookmark & { isFolder?: boolean }) | null>(null)
  const isEditDialogOpen = ref(false)
  const isCreateMode = ref(false)
  const bookmarkType = ref<'bookmark' | 'folder'>('bookmark')
  const currentEditBookmark = ref({
    id: 0,
    title: '',
    url: '',
    folderId: '0' as string | number | undefined,
    iconJson: '',
  } as { id: number, title: string, url: string, folderId?: string | number, iconJson?: string })
  const uploadLoading = ref(false)
  const jsonData = ref<string | null>(null)

  const cache = useBookmarkManagerCache({
    bookmarkTree,
    defaultExpandedKeys,
    fullData,
  })

  function getBookmarkDisplayIcon(iconJson?: string | null) {
    return getBookmarkIconSrc(iconJson)
  }

  function handleBookmarkIconError(event: Event) {
    const target = event.target as HTMLImageElement | null
    if (!target || target.src === DEFAULT_BOOKMARK_ICON)
      return

    target.src = DEFAULT_BOOKMARK_ICON
  }

  async function resolveBookmarkIcon(url: string) {
    const { code, data } = await getSiteFavicon<{ iconUrl: string }>(url)
    if (code === 0 && data?.iconUrl)
      return data.iconUrl

    return ''
  }

  async function persistBookmarkIcon(bookmark: Bookmark, iconJson: string) {
    if (!bookmark.id || bookmark.isFolder)
      return

    const payload = {
      id: Number(bookmark.id),
      title: bookmark.title,
      url: bookmark.url || '',
      parentId: bookmark.folderId ? Number(bookmark.folderId) : 0,
      sort: bookmark.sort || 9999,
      lanUrl: bookmark.lanUrl || '',
      icon: null,
      openMethod: bookmark.openMethod || 0,
      iconJson,
    }

    const response = await update(payload)
    if (response?.code === 0)
      cache.updateCacheAfterUpdate({ ...payload, iconJson })
  }

  async function refreshBookmarkIconInBackground(bookmark: Bookmark, force = false) {
    if (!bookmark.id || bookmark.isFolder || !bookmark.url)
      return

    const bookmarkId = Number(bookmark.id)
    if (refreshingBookmarkIconIds.has(bookmarkId))
      return

    if (!force && bookmark.iconJson)
      return

    refreshingBookmarkIconIds.add(bookmarkId)
    try {
      const iconJson = await resolveBookmarkIcon(bookmark.url)
      if (iconJson && iconJson !== bookmark.iconJson) {
        bookmark.iconJson = iconJson
        await persistBookmarkIcon(bookmark, iconJson)
      }
    } catch (error) {
      logError('刷新书签图标失败', error)
    } finally {
      refreshingBookmarkIconIds.delete(bookmarkId)
    }
  }

  const allItems = computed<(Bookmark & { isFolder?: boolean })[]>(() => {
    const items: (Bookmark & { isFolder?: boolean })[] = []
    const sourceData = fullData.value.length > 0 ? fullData.value : bookmarkTree.value

    function traverseItems(nodes: TreeOption[], folderId: string) {
      for (const node of nodes) {
        if (node.isFolder || (node.children && node.children.length > 0)) {
          items.push({
            id: Number(node.key),
            title: node.label,
            url: '',
            folderId: String(folderId),
            isFolder: true,
            sort: node.sort || node.rawNode?.sort || 0,
            iconJson: node.rawNode?.iconJson || '',
            lanUrl: node.rawNode?.lanUrl || '',
            openMethod: node.rawNode?.openMethod || 0,
            icon: node.rawNode?.icon || null,
          })
        }

        if ((node.isLeaf && node.bookmark) || (!node.isFolder && node.rawNode?.id !== undefined) || (!node.isLeaf && node.rawNode?.url)) {
          const bookmarkData = node.bookmark || node.rawNode
          items.push({
            ...bookmarkData,
            folderId: String(folderId),
            isFolder: false,
          })
        }

        if (node.children && node.children.length > 0)
          traverseItems(node.children, String(node.key))
      }
    }

    traverseItems(sourceData, '0')
    return items
  })

  const filteredBookmarks = computed(() => {
    if (selectedBookmarkId.value) {
      const bookmark = allItems.value.find(item => String(item.id) === selectedBookmarkId.value)
      return bookmark ? [bookmark] : []
    }

    let items = allItems.value
    if (!searchQuery.value.trim()) {
      const targetFolderId = selectedFolder.value || '0'
      items = items.filter(item => String(item.folderId || '0') === targetFolderId)
    }

    if (searchQuery.value.trim()) {
      const query = searchQuery.value.toLowerCase()
      items = items.filter(item => item.title.toLowerCase().includes(query) || (item.url && item.url.toLowerCase().includes(query)))
    }

    return items
  })

  async function refreshVisibleBookmarkIcons() {
    for (const item of filteredBookmarks.value.slice(0, 30)) {
      if (!item.isFolder && item.url && !item.iconJson)
        await refreshBookmarkIconInBackground(item, false)
    }
  }

  watch(filteredBookmarks, () => {
    void refreshVisibleBookmarkIcons()
  }, { immediate: true })

  const folderTreeOptions = computed(() => {
    const treeData = fullData.value.length > 0 ? fullData.value : bookmarkTree.value
    return buildFolderTreeOptions(treeData, t('bookmarkManager.rootDirectory'))
  })

  function exportBookmarks() {
    const sourceTree = fullData.value.length > 0 ? fullData.value : []
    if (sourceTree.length === 0) {
      ms.warning(t('bookmarkManager.noData'))
      return
    }

    exportBookmarkJson().addBookmarksData(toExportBookmarkTree(sourceTree)).exportFile()
  }

  const editor = useBookmarkManagerEditor({
    bookmarkType,
    currentBookmark,
    currentEditBookmark,
    isCreateMode,
    isEditDialogOpen,
    message: ms,
    resolveBookmarkIcon,
    selectedBookmarkId,
    selectedFolder,
    updateCacheAfterAdd: cache.updateCacheAfterAdd,
    updateCacheAfterDelete: cache.updateCacheAfterDelete,
    updateCacheAfterUpdate: cache.updateCacheAfterUpdate,
  })

  const importer = useBookmarkManagerImport({
    fileInput,
    jsonData,
    message: ms,
    refreshBookmarks: cache.refreshBookmarks,
    uploadLoading,
  })

  return {
    allItems,
    bookmarkTree,
    bookmarkType,
    closeEditDialog: editor.closeEditDialog,
    createNewBookmark: editor.createNewBookmark,
    currentBookmark,
    currentEditBookmark,
    defaultExpandedKeys,
    exportBookmarks,
    fileInput,
    filteredBookmarks,
    focusedItemId,
    folderTreeOptions,
    fullData,
    getBookmarkDisplayIcon,
    handleBookmarkIconError,
    handleDeleteBookmark: editor.handleDeleteBookmark,
    handleEditBookmark: editor.handleEditBookmark,
    handleFileChange: importer.handleFileChange,
    isCreateMode,
    isEditDialogOpen,
    refreshBookmarkIconInBackground,
    refreshBookmarks: cache.refreshBookmarks,
    saveBookmarkChanges: editor.saveBookmarkChanges,
    searchQuery,
    selectedBookmarkId,
    selectedFolder,
    triggerImportBookmarks: importer.triggerImportBookmarks,
    updateCacheAfterUpdate: cache.updateCacheAfterUpdate,
  }
}
