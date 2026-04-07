import { computed, h, ref, type Ref } from 'vue'
import { useRouter } from 'vue-router'
import { openUrlWithoutReferer } from '@/utils/cmn'
import type { BookmarkManagerData, BookmarkManagerDragItem } from './bookmarkManagerUiTypes'
import type { Bookmark, TreeOption } from './types'

interface UseBookmarkManagerContextActionsOptions {
  data: BookmarkManagerData
  handleDragEnd: (event: DragEvent) => void
  handleDragStart: (event: DragEvent, item: BookmarkManagerDragItem) => void
  handleDrop: (event: DragEvent, item: BookmarkManagerDragItem) => Promise<void>
  isMobile: Ref<boolean>
  selectedKeysRef: Ref<(string | number)[]>
}

export function useBookmarkManagerContextActions(options: UseBookmarkManagerContextActionsOptions) {
  const { data, handleDragEnd, handleDragStart, handleDrop, isMobile, selectedKeysRef } = options
  const router = useRouter()
  const isContextMenuOpen = ref(false)
  const contextMenuX = ref(0)
  const contextMenuY = ref(0)
  const isDropdownOpen = ref(false)

  const contextMenuStyle = computed(() => {
    const menuWidth = 160
    const menuHeight = 80
    const screenWidth = window.innerWidth
    const screenHeight = window.innerHeight
    let left = contextMenuX.value
    let top = contextMenuY.value

    if (left + menuWidth > screenWidth)
      left = screenWidth - menuWidth

    if (top + menuHeight > screenHeight)
      top = screenHeight - menuHeight

    return {
      top: `${Math.max(0, top)}px`,
      left: `${Math.max(0, left)}px`,
    }
  })

  function goBackToHome() {
    router.push('/home')
  }

  function openContextMenu(event: MouseEvent, bookmark: Bookmark & { isFolder?: boolean }) {
    event.preventDefault()
    event.stopPropagation()
    isContextMenuOpen.value = true
    contextMenuX.value = event.clientX
    contextMenuY.value = event.clientY
    data.currentBookmark.value = bookmark
  }

  function handleTreeContextMenu({ node, event }: { node: TreeOption, event: MouseEvent }) {
    event.preventDefault()
    event.stopPropagation()
    isContextMenuOpen.value = true
    contextMenuX.value = event.clientX
    contextMenuY.value = event.clientY

    let parentFolderId = '0'
    if (node.bookmark?.folderId) {
      parentFolderId = String(node.bookmark.folderId)
    } else {
      const findParentId = (treeNodes: TreeOption[], targetId: string | number): string => {
        for (const item of treeNodes) {
          if (item.children) {
            const childFound = item.children.some(child => String(child.key) === String(targetId))
            if (childFound)
              return String(item.key)

            const parentId = findParentId(item.children, targetId)
            if (parentId !== '0')
              return parentId
          }
        }

        return '0'
      }

      parentFolderId = findParentId(data.bookmarkTree.value, String(node.key))
    }

    data.currentBookmark.value = {
      id: Number(node.key),
      title: node.label,
      url: node.bookmark?.url || '',
      folderId: parentFolderId,
      isFolder: !node.isLeaf,
    }
  }

  const renderExpandIcon = ({ option }: { option: TreeOption }) => {
    void option
    return undefined
  }

  const renderTreeLabel = ({ option }: { option: TreeOption }) => {
    const isFolder = option.isFolder || (!option.isLeaf && !option.bookmark?.url)
    const nodeTitle = option.label || '未命名'
    const content: any[] = []

    if (isFolder) {
      const isSelected = selectedKeysRef.value.includes(option.key)
      const iconColor = isSelected ? '#4285F4' : '#9CA3AF'
      content.push(
        h('svg', {
          xmlns: 'http://www.w3.org/2000/svg',
          class: 'w-5 h-5 inline-block mr-1',
          width: '24',
          height: '24',
          fill: iconColor,
          viewBox: '0 0 24 24',
        }, [
          h('path', {
            d: 'M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm0 12H4V8h16v10z',
          }),
        ]),
      )
      content.push(nodeTitle)
    } else {
      content.push(
        h('img', {
          src: data.getBookmarkDisplayIcon(option.bookmark?.iconJson),
          class: 'w-4 h-4 inline-block mr-1 rounded-full',
          alt: 'bookmark icon',
          onError: (event: Event) => data.handleBookmarkIconError(event),
        }),
      )
      content.push(nodeTitle)
    }

    return h(
      'div',
      {
        class: 'px-1 py-0.5 rounded hover:bg-gray-100 cursor-default flex items-center text-gray-700 dark:text-white',
        draggable: true,
        onDragstart: (event: DragEvent) => {
          handleDragStart(event, {
            id: option.key,
            title: option.label,
            url: option.bookmark?.url || '',
            isFolder: option.isFolder || false,
            folderId: option.rawNode?.parentId || '0',
            iconJson: option.bookmark?.iconJson || '',
            sort: option.sort || 0,
          })
        },
        onDragend: (event: DragEvent) => {
          handleDragEnd(event)
        },
        onDragover: (event: DragEvent) => {
          event.preventDefault()
          if (event.dataTransfer)
            event.dataTransfer.dropEffect = 'move'

          if (event.currentTarget instanceof HTMLElement)
            event.currentTarget.classList.add('bg-blue-100', 'dark:bg-blue-900')
        },
        onDragleave: (event: DragEvent) => {
          if (event.currentTarget instanceof HTMLElement)
            event.currentTarget.classList.remove('bg-blue-100', 'dark:bg-blue-900')
        },
        onDrop: (event: DragEvent) => {
          event.preventDefault()
          if (event.currentTarget instanceof HTMLElement)
            event.currentTarget.classList.remove('bg-blue-100', 'dark:bg-blue-900')

          void handleDrop(event, {
            id: option.key,
            title: option.label,
            url: option.bookmark?.url || '',
            isFolder: option.isFolder || false,
            folderId: option.rawNode?.parentId || '0',
            label: option.label,
            iconJson: option.bookmark?.iconJson || '',
            sort: option.sort || 0,
          })
        },
        onContextmenu: (event: MouseEvent) => {
          event.preventDefault()
          event.stopPropagation()
          if (!isMobile.value)
            handleTreeContextMenu({ node: option, event })
        },
      },
      content,
    )
  }

  function handleGlobalClick(event: MouseEvent) {
    const path = event.composedPath() as HTMLElement[]
    const clickedInsideMenu = path.some(el => el.classList && (el.classList.contains('context-menu') || el.closest('.custom-dropdown')))
    if (!clickedInsideMenu) {
      closeContextMenu()
      isDropdownOpen.value = false
    }
  }

  function closeContextMenu() {
    isContextMenuOpen.value = false
  }

  function handleEditBookmark() {
    data.handleEditBookmark()
    isContextMenuOpen.value = false
  }

  function handleDeleteBookmark() {
    data.handleDeleteBookmark()
    isContextMenuOpen.value = false
  }

  function openBookmark(bookmark: Bookmark) {
    if (bookmark.url && !bookmark.isFolder) {
      openUrlWithoutReferer(bookmark.url, '_blank')
      void data.refreshBookmarkIconInBackground(bookmark, true)
    }
  }

  return {
    contextMenuStyle,
    goBackToHome,
    handleDeleteBookmark,
    handleEditBookmark,
    handleGlobalClick,
    isContextMenuOpen,
    isDropdownOpen,
    openBookmark,
    openContextMenu,
    renderExpandIcon,
    renderTreeLabel,
  }
}
