import { h, ref } from 'vue'
import { useRouter } from 'vue-router'
import { getList as getBookmarksList } from '@/api/panel/bookmark'
import { DEFAULT_BOOKMARK_ICON, getBookmarkIconSrc } from '@/utils/bookmarkIcon'
import { buildBookmarkTree, convertServerTreeToBookmarkTree, type BookmarkTreeNode } from '@/utils/bookmarkTree'
import { openUrlWithoutReferer } from '@/utils/cmn'
import { ss } from '@/utils/storage/local'
import { logError } from '@/utils/logger'

export const BOOKMARKS_CACHE_KEY = 'bookmarksTreeCache'

interface HomeBookmarksAuthStore {
  userInfo?: {
    mustChangePassword?: boolean
  } | null
}

function toBookmarkTree(data: any): BookmarkTreeNode[] {
  if (Array.isArray(data) && data.length > 0 && 'children' in data[0])
    return convertServerTreeToBookmarkTree(data)

  if (data?.list && Array.isArray(data.list)) {
    const serverBookmarks = data.list
    return serverBookmarks.length > 0 && 'children' in serverBookmarks[0]
      ? convertServerTreeToBookmarkTree(serverBookmarks)
      : buildBookmarkTree(serverBookmarks)
  }

  return buildBookmarkTree(Array.isArray(data) ? data : [])
}

function findNodeByKey(nodes: BookmarkTreeNode[], key: string | number): BookmarkTreeNode | null {
  for (const node of nodes) {
    if (node.key === key)
      return node

    if (node.children && node.children.length > 0) {
      const found = findNodeByKey(node.children, key)
      if (found)
        return found
    }
  }

  return null
}

export function useHomeBookmarks(authStore: HomeBookmarksAuthStore) {
  const router = useRouter()
  const treeData = ref<BookmarkTreeNode[]>([])

  async function loadBookmarkTree(forceRefresh = false) {
    if (authStore.userInfo?.mustChangePassword) {
      treeData.value = []
      return
    }

    try {
      if (forceRefresh)
        ss.remove(BOOKMARKS_CACHE_KEY)

      if (!forceRefresh) {
        const cachedData = ss.get(BOOKMARKS_CACHE_KEY)
        if (cachedData) {
          treeData.value = toBookmarkTree(cachedData)
          return
        }
      }

      const response = await getBookmarksList()
      if (response.code === 0) {
        const data = response.data || []
        treeData.value = toBookmarkTree(data)
        ss.set(BOOKMARKS_CACHE_KEY, data)
      }
    } catch (error) {
      logError('获取书签数据失败', error)
      const cachedData = ss.get(BOOKMARKS_CACHE_KEY)
      if (cachedData)
        treeData.value = toBookmarkTree(cachedData)
    }
  }

  function navigateToBookmarkManager() {
    router.push('/bookmark-manager')
  }

  function handleTreeSelect(keys: (string | number)[]) {
    if (!keys || keys.length === 0)
      return

    const selectedNode = findNodeByKey(treeData.value, keys[0])
    if (selectedNode?.isLeaf && selectedNode.bookmark?.url)
      openUrlWithoutReferer(selectedNode.bookmark.url, '_blank')
  }

  function getBookmarkDisplayIcon(iconJson?: string | null) {
    return getBookmarkIconSrc(iconJson)
  }

  function handleBookmarkIconError(event: Event) {
    const target = event.target as HTMLImageElement | null
    if (!target || target.src === DEFAULT_BOOKMARK_ICON)
      return

    target.src = DEFAULT_BOOKMARK_ICON
  }

  const renderTreeLabel = ({ option }: { option: BookmarkTreeNode }) => {
    const nodeData = option || {} as BookmarkTreeNode
    const isFolder = nodeData.isFolder === true || !nodeData.isLeaf
    const displayText = nodeData.label || '未命名'

    try {
      return h('div', { class: 'flex items-center' }, [
        isFolder
          ? h('svg', {
              xmlns: 'http://www.w3.org/2000/svg',
              class: 'w-4 h-4 mr-2',
              width: '24',
              height: '24',
              fill: '#4285F4',
              viewBox: '0 0 24 24',
            }, [
              h('path', {
                d: 'M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm0 12H4V8h16v10z',
              }),
            ])
          : h('img', {
              src: getBookmarkDisplayIcon(nodeData.bookmark?.iconJson),
              class: 'w-4 h-4 mr-2 rounded-full',
              alt: 'bookmark icon',
              onError: (event: Event) => handleBookmarkIconError(event),
            }),
        h('span', {}, displayText),
      ])
    } catch (error) {
      logError('渲染树节点标签时出错', error)
      return h('span', {}, displayText)
    }
  }

  return {
    treeData,
    loadBookmarkTree,
    navigateToBookmarkManager,
    handleTreeSelect,
    renderTreeLabel,
  }
}
