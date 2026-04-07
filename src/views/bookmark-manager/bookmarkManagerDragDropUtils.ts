import type { Ref } from 'vue'
import type { TreeOption } from './types'

export function checkBookmarkTreeDescendant(nodes: TreeOption[], parentId: string | number, childId: string | number): boolean {
  const findNode = (treeNodes: TreeOption[], targetId: string | number): TreeOption | null => {
    for (const node of treeNodes) {
      if (String(node.rawNode?.id || node.key) === String(targetId))
        return node

      if (node.children && node.children.length > 0) {
        const found = findNode(node.children, targetId)
        if (found)
          return found
      }
    }

    return null
  }

  const parentNode = findNode(nodes, parentId)
  if (!parentNode)
    return false

  const checkChildren = (node: TreeOption): boolean => {
    if (!node.children || node.children.length === 0)
      return false

    for (const child of node.children) {
      if (String(child.rawNode?.id || child.key) === String(childId))
        return true

      if (checkChildren(child))
        return true
    }

    return false
  }

  return checkChildren(parentNode)
}

export function updateBookmarkLocalSort(fullData: Ref<TreeOption[]>, bookmarkTree: Ref<TreeOption[]>, itemId: number, newSort: number) {
  const updateNodeSort = (nodes: TreeOption[]): boolean => {
    for (const node of nodes) {
      const nodeId = Number(node.rawNode?.id) || Number(node.key) || Number(node.bookmark?.id)
      if (nodeId === itemId) {
        if (node.bookmark)
          node.bookmark.sort = newSort

        node.sort = newSort
        return true
      }

      if (node.children?.length && updateNodeSort(node.children))
        return true
    }

    return false
  }

  updateNodeSort(fullData.value)
  updateNodeSort(bookmarkTree.value)
}

export function sortBookmarkTreeChildren(nodes: TreeOption[]) {
  for (const node of nodes) {
    if (node.children?.length) {
      node.children.sort((a, b) => (a.bookmark?.sort || a.sort || 0) - (b.bookmark?.sort || b.sort || 0))
      sortBookmarkTreeChildren(node.children)
    }
  }
}

export function serializeBookmarkCacheNode(node: TreeOption): TreeOption {
  const parentId = node.rawNode?.parentId || node.ParentId || '0'
  const rawNode = { ...node.rawNode }
  if (!node.isFolder && rawNode.iconJson)
    delete rawNode.iconJson

  const processedNode: TreeOption = {
    key: node.key,
    label: node.label,
    isLeaf: node.isLeaf,
    isFolder: node.isFolder,
    bookmark: node.bookmark,
    rawNode,
    disabledExpand: node.disabledExpand,
    sort: node.sort,
    ParentId: String(parentId),
    children: [],
  }

  if (node.children && node.children.length > 0)
    processedNode.children = node.children.map(child => serializeBookmarkCacheNode(child))

  return processedNode
}
