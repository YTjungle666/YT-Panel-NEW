import type { Ref } from 'vue'
import { useBookmarkManagerContextActions } from './useBookmarkManagerContextActions'
import { useBookmarkManagerSelection } from './useBookmarkManagerSelection'
import type { BookmarkManagerData, BookmarkManagerDragItem } from './bookmarkManagerUiTypes'

interface UseBookmarkManagerTreeUiOptions {
  data: BookmarkManagerData
  isMobile: Ref<boolean>
  handleDragStart: (event: DragEvent, item: BookmarkManagerDragItem) => void
  handleDragEnd: (event: DragEvent) => void
  handleDrop: (event: DragEvent, item: BookmarkManagerDragItem) => Promise<void>
}

export function useBookmarkManagerTreeUi(options: UseBookmarkManagerTreeUiOptions) {
  const selection = useBookmarkManagerSelection(options.data)
  const contextActions = useBookmarkManagerContextActions({
    data: options.data,
    handleDragEnd: options.handleDragEnd,
    handleDragStart: options.handleDragStart,
    handleDrop: options.handleDrop,
    isMobile: options.isMobile,
    selectedKeysRef: selection.selectedKeysRef,
  })

  return {
    contextMenuStyle: contextActions.contextMenuStyle,
    currentPath: selection.currentPath,
    goBackToHome: contextActions.goBackToHome,
    handleBreadcrumbClick: selection.handleBreadcrumbClick,
    handleDeleteBookmark: contextActions.handleDeleteBookmark,
    handleEditBookmark: contextActions.handleEditBookmark,
    handleGlobalClick: contextActions.handleGlobalClick,
    handleNodeExpand: selection.handleNodeExpand,
    handleSearch: selection.handleSearch,
    handleSelect: selection.handleSelect,
    isContextMenuOpen: contextActions.isContextMenuOpen,
    isDropdownOpen: contextActions.isDropdownOpen,
    openBookmark: contextActions.openBookmark,
    openContextMenu: contextActions.openContextMenu,
    openFolder: selection.openFolder,
    renderExpandIcon: contextActions.renderExpandIcon,
    renderTreeLabel: contextActions.renderTreeLabel,
    selectedKeysRef: selection.selectedKeysRef,
    setFocusedItemId: selection.setFocusedItemId,
  }
}
