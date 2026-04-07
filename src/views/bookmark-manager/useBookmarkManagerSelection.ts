import { computed, ref, watch } from 'vue'
import { t } from '@/locales'
import type { BookmarkManagerData } from './bookmarkManagerUiTypes'
import type { TreeOption } from './types'

export function useBookmarkManagerSelection(data: BookmarkManagerData) {
  const selectedKeysRef = ref<(string | number)[]>([])

  const currentPath = computed(() => {
    const rootPath = [{ id: '0', name: t('bookmarkManager.rootDirectory') }]
    if (!data.selectedFolder.value || data.selectedFolder.value === '0')
      return rootPath

    const findNodePath = (
      nodes: TreeOption[],
      targetId: string,
      currentNodes: { id: string, name: string }[] = [],
    ): { id: string, name: string }[] | null => {
      for (const node of nodes) {
        const newPath = [...currentNodes, { id: String(node.key), name: node.label }]
        if (String(node.key) === targetId)
          return newPath

        if (node.children && node.children.length > 0) {
          const foundPath = findNodePath(node.children, targetId, newPath)
          if (foundPath)
            return foundPath
        }
      }

      return null
    }

    const fullPath = findNodePath(data.bookmarkTree.value, data.selectedFolder.value, [])
    if (fullPath)
      return rootPath.concat(fullPath)

    return rootPath.concat([{ id: data.selectedFolder.value, name: '...' }])
  })

  function handleBreadcrumbClick(id: string) {
    data.selectedFolder.value = id
    data.selectedBookmarkId.value = ''
  }

  function handleSelect(keys: (string | number)[]) {
    selectedKeysRef.value = keys
    data.selectedBookmarkId.value = ''

    if (!keys || !Array.isArray(keys) || keys.length === 0)
      return

    const key = String(keys[0])
    const findNodeById = (nodes: TreeOption[], id: string): TreeOption | null => {
      for (const node of nodes) {
        if (String(node.key) === id)
          return node

        if (node.children && node.children.length > 0) {
          const found = findNodeById(node.children, id)
          if (found)
            return found
        }
      }

      return null
    }

    const selectedNode = findNodeById(data.bookmarkTree.value, key)
    if (!selectedNode)
      return

    if (selectedNode.isLeaf && selectedNode.bookmark) {
      data.selectedBookmarkId.value = key
      data.selectedFolder.value = ''
    } else if (selectedNode.isFolder || !selectedNode.isLeaf || !selectedNode.bookmark?.url) {
      data.selectedFolder.value = key
      selectedKeysRef.value = [key]
    }
  }

  function handleNodeExpand(node: TreeOption) {
    if (!node?.key)
      return

    const key = String(node.key)
    data.selectedFolder.value = key
    selectedKeysRef.value = [key]
  }

  function handleSearch() {
    if (data.searchQuery.value) {
      data.selectedFolder.value = ''
      data.selectedBookmarkId.value = ''
    }
  }

  function setFocusedItemId(id: string) {
    data.focusedItemId.value = id
  }

  function openFolder(folderId: string | number) {
    data.selectedFolder.value = String(folderId)
    data.searchQuery.value = ''
    selectedKeysRef.value = [String(folderId)]
  }

  watch(data.selectedFolder, (newFolderId) => {
    if (newFolderId && newFolderId !== '0') {
      const buildPath = (nodes: TreeOption[], targetId: string, path: string[] = []): string[] | null => {
        for (const node of nodes) {
          const nodeId = String(node.key)
          const currentPath = [...path, nodeId]
          if (nodeId === targetId)
            return currentPath

          if (node.children && node.children.length > 0) {
            const found = buildPath(node.children, targetId, currentPath)
            if (found)
              return found
          }
        }

        return null
      }

      const path = buildPath(data.bookmarkTree.value, newFolderId)
      if (path) {
        data.defaultExpandedKeys.value = [...new Set([...data.defaultExpandedKeys.value, ...path.slice(0, -1)])]
        selectedKeysRef.value = [newFolderId]
      }
    } else if (newFolderId === '0') {
      selectedKeysRef.value = []
    }
  })

  return {
    currentPath,
    handleBreadcrumbClick,
    handleNodeExpand,
    handleSearch,
    handleSelect,
    openFolder,
    selectedKeysRef,
    setFocusedItemId,
  }
}
