import type { BookmarkTreeNode } from '@/utils/bookmarkImportExport'
import type { TreeOption } from './types'

export const BOOKMARKS_CACHE_KEY = 'bookmarksTreeCache'

export function filterFoldersOnly(nodes: TreeOption[]): TreeOption[] {
  return nodes
    .filter(node => node.isFolder)
    .map((node) => {
      const children = filterFoldersOnly(node.children || [])
      return {
        ...node,
        children,
        isLeaf: children.length === 0,
        disabledExpand: children.length === 0,
      }
    })
}

export function toExportBookmarkTree(nodes: TreeOption[]): BookmarkTreeNode[] {
  return nodes.map((node) => {
    if (node.isFolder) {
      return {
        id: Number(node.key),
        title: node.label,
        isFolder: true,
        children: toExportBookmarkTree(node.children || []),
      }
    }

    return {
      id: Number(node.bookmark?.id || node.key),
      title: node.bookmark?.title || node.label,
      url: node.bookmark?.url || '',
      icon: node.bookmark?.iconJson || undefined,
      isFolder: false,
    }
  })
}

export function convertServerTreeToFrontendTree(serverTree: any[]): TreeOption[] {
  const result: TreeOption[] = []

  function processNode(node: any): TreeOption {
    const isTreeOption = Object.prototype.hasOwnProperty.call(node, 'key')
      && Object.prototype.hasOwnProperty.call(node, 'label')

    let nodeKey: string
    let isFolder: boolean
    let title: string
    let url: string
    let iconJson: string
    let children: any[]
    let rawNode: any

    if (isTreeOption) {
      nodeKey = String(node.key)
      isFolder = node.isFolder
      title = node.label
      url = node.bookmark?.url || ''
      iconJson = node.bookmark?.iconJson || ''
      children = node.children || []
      rawNode = {
        ...node.rawNode,
        parentId: node.rawNode?.parentId || '0',
      }
    } else {
      nodeKey = String(node.id)
      isFolder = node.isFolder === 1
      title = node.title
      url = node.url || ''
      iconJson = node.iconJson || ''
      children = node.children || []
      rawNode = {
        ...node,
        parentId: node.parentId || '0',
      }

      if (!isFolder && rawNode.iconJson)
        delete rawNode.iconJson
    }

    const hasChildren = Array.isArray(children) && children.length > 0
    const frontendNode: TreeOption = {
      key: nodeKey,
      label: title || '未命名',
      isLeaf: !isFolder || !hasChildren,
      isFolder,
      bookmark: isFolder
        ? undefined
        : {
            id: node.id || nodeKey,
            title,
            url: url || '',
            iconJson: iconJson || '',
            sort: node.sort || 0,
          },
      children: [],
      rawNode,
      disabledExpand: !hasChildren,
      sort: node.sort || rawNode.sort || 0,
    }

    if (hasChildren) {
      frontendNode.children = children.map((child: any) => processNode(child))
      frontendNode.children.sort((a, b) => (a.rawNode?.sort ?? 0) - (b.rawNode?.sort ?? 0))
    }

    return frontendNode
  }

  for (const node of serverTree || [])
    result.push(processNode(node))

  result.sort((a, b) => (a.rawNode?.sort ?? 0) - (b.rawNode?.sort ?? 0))
  return result
}

export function buildBookmarkTree(bookmarks: any[]): TreeOption[] {
  const nodeMap = new Map<string, TreeOption>()
  const rootNodes: TreeOption[] = []

  for (const bookmark of bookmarks || []) {
    const isFrontendFormat = Object.prototype.hasOwnProperty.call(bookmark, 'key')
      && Object.prototype.hasOwnProperty.call(bookmark, 'label')

    const nodeId = isFrontendFormat ? bookmark.key : bookmark.id || bookmark.Key || '0'
    const nodeKey = String(nodeId)
    const title = isFrontendFormat ? bookmark.label : bookmark.title
    const isFolder = isFrontendFormat ? bookmark.isFolder : bookmark.isFolder === 1
    const url = isFrontendFormat ? (bookmark.bookmark?.url || '') : bookmark.url || ''
    const iconJson = isFrontendFormat ? (bookmark.bookmark?.iconJson || '') : bookmark.iconJson || ''
    const parentId = isFrontendFormat
      ? (bookmark.rawNode?.parentId || bookmark.ParentId || '0')
      : (bookmark.parentId || bookmark.ParentId || '0')
    const sort = isFrontendFormat ? (bookmark.bookmark?.sort || 0) : (bookmark.sort || 0)

    const rawNodeData = {
      ...bookmark,
      parentId,
    }
    if (!isFolder && rawNodeData.iconJson)
      delete rawNodeData.iconJson

    const node: TreeOption = {
      key: nodeKey,
      label: title || '未命名',
      isLeaf: !isFolder,
      isFolder,
      bookmark: isFolder
        ? undefined
        : {
            id: nodeId,
            title: title || '未命名',
            url,
            iconJson,
            sort,
          },
      rawNode: rawNodeData,
      children: [],
      disabledExpand: true,
      sort,
    }

    nodeMap.set(nodeKey, node)
  }

  for (const bookmark of bookmarks || []) {
    const isFrontendFormat = Object.prototype.hasOwnProperty.call(bookmark, 'key')
      && Object.prototype.hasOwnProperty.call(bookmark, 'label')
    const nodeKey = String(isFrontendFormat ? bookmark.key : bookmark.id || bookmark.Key || '0')
    const node = nodeMap.get(nodeKey)
    if (!node)
      continue

    const parentId = isFrontendFormat
      ? (bookmark.rawNode?.parentId || bookmark.ParentId || '0')
      : (bookmark.parentId || bookmark.ParentId || '0')
    const parentKey = String(parentId)

    if (parentId === '0' || parentId === '' || parentId === 'null' || parentId === null || parentId === undefined) {
      rootNodes.push(node)
    } else {
      const parentNode = nodeMap.get(parentKey)
      if (parentNode && parentNode.key !== node.key) {
        parentNode.children.push(node)
        parentNode.children.sort((a, b) => (a.rawNode?.sort ?? 0) - (b.rawNode?.sort ?? 0))
        parentNode.disabledExpand = false
        parentNode.isLeaf = false
      } else {
        rootNodes.push(node)
      }
    }
  }

  rootNodes.sort((a, b) => (a.rawNode?.sort ?? 0) - (b.rawNode?.sort ?? 0))
  for (const node of nodeMap.values()) {
    if (node.isFolder)
      node.isLeaf = node.children.length === 0
  }

  return rootNodes
}

export function convertBookmarkToCacheNode(bookmark: any) {
  const isFolder = bookmark.isFolder === 1 || bookmark.isFolder === true
  return {
    id: bookmark.id,
    title: bookmark.title,
    url: bookmark.url || '',
    iconJson: bookmark.iconJson || '',
    isFolder: isFolder ? 1 : 0,
    parentId: bookmark.parentId || '0',
    sort: bookmark.sort || 0,
    children: [],
  }
}

export function findNodeInTree(nodes: any[], id: string | number): any {
  for (const node of nodes) {
    if (String(node.id || node.key) === String(id))
      return node

    if (node.children && node.children.length > 0) {
      const found = findNodeInTree(node.children, id)
      if (found)
        return found
    }
  }

  return null
}

export function removeNodeFromTree(nodes: any[], id: string | number): boolean {
  for (let i = 0; i < nodes.length; i += 1) {
    if (String(nodes[i].id || nodes[i].key) === String(id)) {
      nodes.splice(i, 1)
      return true
    }

    if (nodes[i].children && nodes[i].children.length > 0 && removeNodeFromTree(nodes[i].children, id))
      return true
  }

  return false
}

export function cloneTree<T>(nodes: T[]): T[] {
  return structuredClone(nodes)
}

export function normalizeFolderTree(treeDataResult: TreeOption[]) {
  const folderTreeData = filterFoldersOnly(treeDataResult)
  return {
    folderTreeData,
    bookmarkTree: cloneTree(folderTreeData),
    defaultExpandedKeys: folderTreeData[0]?.key ? [folderTreeData[0].key] : [],
  }
}

export function buildFolderTreeOptions(treeData: TreeOption[], rootLabel: string) {
  const processNodes = (nodes: TreeOption[]): any[] => {
    return nodes
      .map((node) => {
        const isFolder = node.isFolder || (!node.isLeaf && !node.bookmark?.url)
        if (!isFolder)
          return null

        const option: any = {
          key: String(node.key),
          label: node.label || 'Unknown',
        }

        if (node.children && node.children.length > 0) {
          const children = processNodes(node.children)
          if (children.length > 0)
            option.children = children
        }

        return option
      })
      .filter(Boolean)
  }

  return [
    {
      key: '0',
      label: rootLabel,
    },
    ...processNodes(treeData),
  ]
}
