import { onMounted, onUnmounted } from 'vue'
import type { BookmarkManagerData } from './bookmarkManagerUiTypes'
import { useBookmarkManagerDragDrop } from './useBookmarkManagerDragDrop'
import { useBookmarkManagerLayout } from './useBookmarkManagerLayout'
import { useBookmarkManagerTreeUi } from './useBookmarkManagerTreeUi'

export function useBookmarkManagerUi(data: BookmarkManagerData) {
  const layout = useBookmarkManagerLayout(data)
  const dragDrop = useBookmarkManagerDragDrop(data)
  const treeUi = useBookmarkManagerTreeUi({
    data,
    isMobile: layout.isMobile,
    handleDragStart: dragDrop.handleDragStart,
    handleDragEnd: dragDrop.handleDragEnd,
    handleDrop: dragDrop.handleDrop,
  })

  onMounted(async () => {
    await layout.initializePage()
    layout.setupLayoutListeners(treeUi.handleGlobalClick)
  })

  onUnmounted(() => {
    layout.cleanupLayoutListeners(treeUi.handleGlobalClick)
    if ((globalThis as any).__bookmarksFullData)
      delete (globalThis as any).__bookmarksFullData
  })

  return {
    collapsePanel: layout.collapsePanel,
    contextMenuStyle: treeUi.contextMenuStyle,
    currentPath: treeUi.currentPath,
    dragIndicatorTop: dragDrop.dragIndicatorTop,
    dragInsertPosition: dragDrop.dragInsertPosition,
    dragOverTarget: dragDrop.dragOverTarget,
    focusedItemId: data.focusedItemId,
    goBackToHome: treeUi.goBackToHome,
    handleBreadcrumbClick: treeUi.handleBreadcrumbClick,
    handleContainerDragOver: dragDrop.handleContainerDragOver,
    handleDeleteBookmark: treeUi.handleDeleteBookmark,
    handleDragEnd: dragDrop.handleDragEnd,
    handleDragLeave: dragDrop.handleDragLeave,
    handleDragOver: dragDrop.handleDragOver,
    handleDragStart: dragDrop.handleDragStart,
    handleDrop: dragDrop.handleDrop,
    handleEditBookmark: treeUi.handleEditBookmark,
    handleNodeExpand: treeUi.handleNodeExpand,
    handleSearch: treeUi.handleSearch,
    handleSelect: treeUi.handleSelect,
    isContextMenuOpen: treeUi.isContextMenuOpen,
    isDropdownOpen: treeUi.isDropdownOpen,
    isMobile: layout.isMobile,
    isPanelExpanded: layout.isPanelExpanded,
    leftPanelWidth: layout.leftPanelWidth,
    openBookmark: treeUi.openBookmark,
    openContextMenu: treeUi.openContextMenu,
    openFolder: treeUi.openFolder,
    renderExpandIcon: treeUi.renderExpandIcon,
    renderTreeLabel: treeUi.renderTreeLabel,
    resetDragState: dragDrop.resetDragState,
    selectedKeysRef: treeUi.selectedKeysRef,
    setFocusedItemId: treeUi.setFocusedItemId,
    showLeftPanel: layout.showLeftPanel,
    startResize: layout.startResize,
    togglePanel: layout.togglePanel,
  }
}
