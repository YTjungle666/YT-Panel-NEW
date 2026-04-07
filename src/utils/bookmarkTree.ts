export interface BookmarkTreeBookmark {
  id: string | number
  title: string
  url: string
  folderId: string | null
  iconJson?: string
  sort?: number
}

export interface BookmarkTreeNode {
  key: string | number
  label: string
  isLeaf: boolean
  isFolder: boolean
  sort?: number
  bookmark?: BookmarkTreeBookmark
  children?: BookmarkTreeNode[]
  rawNode?: Record<string, any>
}

function sortBySort<T extends { sort?: number }>(items: T[]) {
  return [...items].sort((a, b) => (a.sort || 0) - (b.sort || 0))
}

export function convertServerTreeToBookmarkTree(serverTree: any[]): BookmarkTreeNode[] {
  const sortedServerTree = sortBySort(serverTree || [])

  return sortedServerTree.map((node) => {
    const isFrontendFormat = Object.prototype.hasOwnProperty.call(node, 'key')
      && Object.prototype.hasOwnProperty.call(node, 'label')

    const nodeId = isFrontendFormat ? node.key : node.id
    const title = isFrontendFormat ? node.label : node.title
    const isFolder = isFrontendFormat ? !!node.isFolder : node.isFolder === 1
    const url = isFrontendFormat ? (node.bookmark?.url || '') : (node.url || '')
    const iconJson = isFrontendFormat ? (node.bookmark?.iconJson || '') : (node.iconJson || '')
    const parentId = isFrontendFormat
      ? (node.rawNode?.parentId || node.ParentId || '0')
      : (node.parentId || node.ParentId || '0')
    const sort = node.sort || node.rawNode?.sort || 0

    const frontendNode: BookmarkTreeNode = {
      key: nodeId,
      label: title || '未命名',
      isLeaf: !isFolder,
      isFolder,
      sort,
      rawNode: {
        ...(node.rawNode || node),
        parentId,
      },
    }

    if (!isFolder && url) {
      frontendNode.bookmark = {
        id: nodeId,
        title: title || '未命名',
        url,
        folderId: parentId !== undefined ? String(parentId) : null,
        iconJson,
        sort,
      }
    }

    if (Array.isArray(node.children) && node.children.length > 0) {
      frontendNode.children = convertServerTreeToBookmarkTree(sortBySort(node.children))
      frontendNode.isLeaf = false
    } else {
      frontendNode.children = []
    }

    return frontendNode
  })
}

export function buildBookmarkTree(bookmarks: any[]): BookmarkTreeNode[] {
  const folders = (bookmarks || []).filter((item) => item.isFolder === 1 || item.isFolder === true)
  const items = (bookmarks || []).filter((item) => item.isFolder === 0 || item.isFolder === false || item.isFolder === undefined)

  const rootFolders: BookmarkTreeNode[] = []
  const folderMap = new Map<string, BookmarkTreeNode>()

  folders.forEach((folder) => {
    const isFrontendFormat = Object.prototype.hasOwnProperty.call(folder, 'key')
      && Object.prototype.hasOwnProperty.call(folder, 'label')
    const folderId = isFrontendFormat ? folder.key : folder.id
    const folderTitle = isFrontendFormat ? folder.label : folder.title
    const sort = folder.sort || folder.rawNode?.sort || 0
    const folderNode: BookmarkTreeNode = {
      key: folderId,
      label: folderTitle || '未命名',
      isLeaf: false,
      isFolder: true,
      sort,
      children: [],
      rawNode: {
        ...(folder.rawNode || folder),
        parentId: isFrontendFormat ? (folder.rawNode?.parentId || folder.ParentId || '0') : (folder.parentId || folder.ParentId || '0'),
      },
    }
    folderMap.set(String(folderId), folderNode)
    folderMap.set(String(folderTitle), folderNode)
  })

  folders.forEach((folder) => {
    const folderId = Object.prototype.hasOwnProperty.call(folder, 'key') ? folder.key : folder.id
    const folderNode = folderMap.get(String(folderId))
    const parentId = Object.prototype.hasOwnProperty.call(folder, 'key')
      ? (folder.rawNode?.parentId || folder.ParentId || '0')
      : (folder.parentId || folder.ParentId || '0')

    if (parentId && parentId !== '0' && parentId !== 0) {
      const parentFolder = folderMap.get(String(parentId))
      if (parentFolder && folderNode) {
        parentFolder.children = sortBySort([...(parentFolder.children || []), folderNode])
        parentFolder.isLeaf = false
        return
      }
    }

    if (folderNode)
      rootFolders.push(folderNode)
  })

  items.forEach((item) => {
    const isFrontendFormat = Object.prototype.hasOwnProperty.call(item, 'key')
      && Object.prototype.hasOwnProperty.call(item, 'label')
    const bookmarkId = isFrontendFormat ? item.key : item.id
    const bookmarkTitle = isFrontendFormat ? item.label : (item.title || '未命名')
    const bookmarkUrl = isFrontendFormat ? (item.bookmark?.url || '') : (item.url || '')
    const bookmarkIconJson = isFrontendFormat ? (item.bookmark?.iconJson || '') : (item.iconJson || '')
    const folderId = isFrontendFormat
      ? (item.rawNode?.parentId || item.ParentId || '0')
      : (item.parentId || item.ParentId || '0')
    const stringFolderId = String(folderId)
    const sort = isFrontendFormat ? (item.bookmark?.sort || item.sort || 0) : (item.sort || 0)

    let targetFolder = folderMap.get(stringFolderId)

    if (stringFolderId === '0' || stringFolderId === 'null' || stringFolderId === 'undefined') {
      targetFolder = folderMap.get('未分类')
      if (!targetFolder) {
        targetFolder = {
          key: '未分类',
          label: '未分类',
          isLeaf: false,
          isFolder: true,
          sort: 0,
          children: [],
          rawNode: { parentId: '0' },
        }
        folderMap.set('未分类', targetFolder)
        rootFolders.push(targetFolder)
      }
    }

    const bookmarkNode: BookmarkTreeNode = {
      key: bookmarkId,
      label: bookmarkTitle,
      isLeaf: true,
      isFolder: false,
      sort,
      bookmark: {
        id: bookmarkId,
        title: bookmarkTitle,
        url: bookmarkUrl,
        folderId: stringFolderId,
        iconJson: bookmarkIconJson,
        sort,
      },
      children: [],
      rawNode: {
        ...(item.rawNode || item),
        parentId: stringFolderId,
      },
    }

    if (targetFolder) {
      targetFolder.children = sortBySort([...(targetFolder.children || []), bookmarkNode])
      targetFolder.isLeaf = false
    } else {
      rootFolders.push(bookmarkNode)
    }
  })

  const sortTreeNodes = (nodes: BookmarkTreeNode[]) => {
    nodes.sort((a, b) => (a.sort || 0) - (b.sort || 0))
    nodes.forEach((node) => {
      if (node.children && node.children.length > 0)
        sortTreeNodes(node.children)
    })
  }

  sortTreeNodes(rootFolders)
  return rootFolders
}
