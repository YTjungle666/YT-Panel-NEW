import type { Ref } from 'vue'
import { getList as getBookmarksList } from '@/api/panel/bookmark'
import { logError } from '@/utils/logger'
import { ss } from '@/utils/storage/local'
import {
  BOOKMARKS_CACHE_KEY,
  buildBookmarkTree,
  cloneTree,
  convertBookmarkToCacheNode,
  convertServerTreeToFrontendTree,
  filterFoldersOnly,
  findNodeInTree,
  normalizeFolderTree,
  removeNodeFromTree,
} from './bookmarkTreeState'
import type { TreeOption } from './types'

interface UseBookmarkManagerCacheOptions {
  bookmarkTree: Ref<TreeOption[]>
  defaultExpandedKeys: Ref<string[]>
  fullData: Ref<TreeOption[]>
}

export function useBookmarkManagerCache(options: UseBookmarkManagerCacheOptions) {
  const { bookmarkTree, defaultExpandedKeys, fullData } = options

  function applyTreeData(treeDataResult: TreeOption[]) {
    fullData.value = treeDataResult
    const normalized = normalizeFolderTree(treeDataResult)
    bookmarkTree.value = normalized.bookmarkTree
    defaultExpandedKeys.value = normalized.defaultExpandedKeys
    if (globalThis)
      Object.defineProperty(globalThis, '__bookmarksFullData', { value: treeDataResult, configurable: true })
  }

  function resetTreeData() {
    fullData.value = []
    bookmarkTree.value = []
    defaultExpandedKeys.value = []
    if (globalThis)
      Object.defineProperty(globalThis, '__bookmarksFullData', { value: [], configurable: true })
  }

  function buildTreeDataFromCache(cachedData: any): TreeOption[] {
    if (Array.isArray(cachedData) && cachedData.length > 0 && 'children' in cachedData[0])
      return convertServerTreeToFrontendTree(cachedData)

    if ((cachedData as { list?: any[] }).list && Array.isArray((cachedData as { list?: any[] }).list)) {
      const serverBookmarks = (cachedData as { list: any[] }).list
      return serverBookmarks.length > 0 && 'children' in serverBookmarks[0]
        ? convertServerTreeToFrontendTree(serverBookmarks)
        : buildBookmarkTree(serverBookmarks)
    }

    return buildBookmarkTree(Array.isArray(cachedData) ? cachedData : [])
  }

  async function refreshBookmarks(forceRefresh = false) {
    try {
      if (forceRefresh)
        ss.remove(BOOKMARKS_CACHE_KEY)

      if (!forceRefresh) {
        const cachedData = ss.get(BOOKMARKS_CACHE_KEY)
        if (cachedData) {
          const treeDataResult = buildTreeDataFromCache(cachedData)
          applyTreeData(treeDataResult)
          if (treeDataResult.length > 0)
            return
        }
      }

      const response = await getBookmarksList()
      if (response.code === 0) {
        let serverBookmarks: any[] = []
        if (response.data) {
          if (Array.isArray(response.data))
            serverBookmarks = response.data
          else if (Array.isArray((response.data as any).list))
            serverBookmarks = (response.data as any).list
        }

        const treeDataResult = serverBookmarks.length > 0 && 'children' in serverBookmarks[0]
          ? convertServerTreeToFrontendTree(serverBookmarks)
          : buildBookmarkTree(serverBookmarks)

        ss.set(BOOKMARKS_CACHE_KEY, response.data)
        applyTreeData(treeDataResult)
      } else {
        logError('获取书签数据失败', response)
        resetTreeData()
      }
    } catch (error) {
      logError('刷新书签列表发生异常', error)
      resetTreeData()
    }
  }

  function updateCacheAfterAdd(bookmark: any) {
    try {
      const cachedData = ss.get(BOOKMARKS_CACHE_KEY)
      if (!cachedData) {
        void refreshBookmarks(false)
        return
      }

      const newBookmark = convertBookmarkToCacheNode(bookmark)
      let cacheList: any[] = []
      if (Array.isArray(cachedData)) {
        cacheList = cachedData
      } else if (cachedData.list && Array.isArray(cachedData.list)) {
        cacheList = cachedData.list
      } else {
        void refreshBookmarks(false)
        return
      }

      if (newBookmark.parentId === '0') {
        cacheList.push(newBookmark)
        cacheList.sort((a, b) => (a.sort || 0) - (b.sort || 0))
      } else {
        const parentNode = findNodeInTree(cacheList, newBookmark.parentId)
        if (parentNode) {
          if (!parentNode.children)
            parentNode.children = []

          parentNode.children.push(newBookmark)
          parentNode.children.sort((a: any, b: any) => (a.sort || 0) - (b.sort || 0))
        } else {
          cacheList.push(newBookmark)
          cacheList.sort((a, b) => (a.sort || 0) - (b.sort || 0))
        }
      }

      ss.set(BOOKMARKS_CACHE_KEY, cacheList)
      const treeDataResult = convertServerTreeToFrontendTree(cacheList)
      fullData.value = treeDataResult
      bookmarkTree.value = cloneTree(filterFoldersOnly(treeDataResult))
    } catch (error) {
      logError('更新缓存失败，刷新数据', error)
      void refreshBookmarks(false)
    }
  }

  function updateCacheAfterUpdate(bookmark: any) {
    try {
      const cachedData = ss.get(BOOKMARKS_CACHE_KEY)
      if (!cachedData) {
        void refreshBookmarks(false)
        return
      }

      let cacheList: any[] = []
      if (Array.isArray(cachedData)) {
        cacheList = cachedData
      } else if (cachedData.list && Array.isArray(cachedData.list)) {
        cacheList = cachedData.list
      } else {
        void refreshBookmarks(false)
        return
      }

      const node = findNodeInTree(cacheList, bookmark.id)
      if (!node) {
        void refreshBookmarks(false)
        return
      }

      const oldParentId = node.parentId || '0'
      const newParentId = bookmark.parentId || bookmark.ParentId || '0'
      node.title = bookmark.title
      node.url = bookmark.url || ''
      node.iconJson = bookmark.iconJson || ''
      node.sort = bookmark.sort || 0
      node.parentId = newParentId

      if (String(oldParentId) !== String(newParentId)) {
        removeNodeFromTree(cacheList, bookmark.id)
        if (newParentId === '0' || newParentId === '' || newParentId === 'null' || newParentId === null || newParentId === undefined) {
          cacheList.push(node)
          cacheList.sort((a, b) => (a.sort || 0) - (b.sort || 0))
        } else {
          const parentNode = findNodeInTree(cacheList, newParentId)
          if (parentNode) {
            if (!parentNode.children)
              parentNode.children = []

            parentNode.children.push(node)
            parentNode.children.sort((a: any, b: any) => (a.sort || 0) - (b.sort || 0))
          } else {
            cacheList.push(node)
            cacheList.sort((a, b) => (a.sort || 0) - (b.sort || 0))
          }
        }
      }

      ss.set(BOOKMARKS_CACHE_KEY, cacheList)
      const treeDataResult = convertServerTreeToFrontendTree(cacheList)
      fullData.value = treeDataResult
      bookmarkTree.value = cloneTree(filterFoldersOnly(treeDataResult))
    } catch (error) {
      logError('更新缓存失败，刷新数据', error)
      void refreshBookmarks(false)
    }
  }

  function updateCacheAfterDelete(bookmarkId: number) {
    try {
      const cachedData = ss.get(BOOKMARKS_CACHE_KEY)
      if (!cachedData) {
        void refreshBookmarks(false)
        return
      }

      let cacheList: any[] = []
      if (Array.isArray(cachedData)) {
        cacheList = cachedData
      } else if (cachedData.list && Array.isArray(cachedData.list)) {
        cacheList = cachedData.list
      } else {
        void refreshBookmarks(false)
        return
      }

      const collectAllIdsToDelete = (nodes: any[], targetId: number): number[] => {
        const idsToDelete: number[] = []
        const findNode = (nodeList: any[]): any => {
          for (const node of nodeList) {
            const nodeId = Number(node.id || node.key || 0)
            if (nodeId === targetId)
              return node

            if (node.children && node.children.length > 0) {
              const found = findNode(node.children)
              if (found)
                return found
            }
          }

          return null
        }

        const targetNode = findNode(nodes)
        if (!targetNode)
          return idsToDelete

        idsToDelete.push(Number(targetNode.id || targetNode.key || bookmarkId))
        if ((targetNode.isFolder === 1 || targetNode.isFolder === true) && targetNode.children && targetNode.children.length > 0) {
          const collectChildrenIds = (children: any[]) => {
            for (const child of children) {
              const childId = Number(child.id || child.key || 0)
              if (childId > 0)
                idsToDelete.push(childId)

              if ((child.isFolder === 1 || child.isFolder === true) && child.children && child.children.length > 0)
                collectChildrenIds(child.children)
            }
          }
          collectChildrenIds(targetNode.children)
        }

        return idsToDelete
      }

      const idsToDelete = collectAllIdsToDelete(cacheList, bookmarkId)
      if (idsToDelete.length === 0) {
        void refreshBookmarks(false)
        return
      }

      const deleteNodeById = (nodes: any[], id: number): boolean => {
        for (let index = 0; index < nodes.length; index += 1) {
          const nodeId = Number(nodes[index].id || nodes[index].key || 0)
          if (nodeId === id) {
            nodes.splice(index, 1)
            return true
          }

          if (nodes[index].children && nodes[index].children.length > 0 && deleteNodeById(nodes[index].children, id))
            return true
        }

        return false
      }

      for (const id of [...idsToDelete].sort((a, b) => b - a))
        deleteNodeById(cacheList, id)

      ss.set(BOOKMARKS_CACHE_KEY, cacheList)
      const treeDataResult = convertServerTreeToFrontendTree(cacheList)
      fullData.value = treeDataResult
      const folderTreeData = filterFoldersOnly(treeDataResult)
      bookmarkTree.value = folderTreeData.length > 0 ? cloneTree(folderTreeData) : []
    } catch (error) {
      logError('更新缓存失败，刷新数据', error)
      void refreshBookmarks(false)
    }
  }

  return {
    applyTreeData,
    refreshBookmarks,
    updateCacheAfterAdd,
    updateCacheAfterDelete,
    updateCacheAfterUpdate,
  }
}
