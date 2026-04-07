import { ref } from 'vue'
import type { BookmarkManagerData } from './bookmarkManagerUiTypes'

export function useBookmarkManagerLayout(data: BookmarkManagerData) {
  const isMobile = ref(false)
  const showLeftPanel = ref(true)
  const leftPanelWidth = ref(256)
  const isResizing = ref(false)
  const isPanelExpanded = ref(false)
  const isBookmarkPageInitialized = ref(false)

  function togglePanel() {
    showLeftPanel.value = true
    isPanelExpanded.value = true
  }

  function collapsePanel() {
    if (!isPanelExpanded.value)
      return

    isPanelExpanded.value = false
    setTimeout(() => {
      showLeftPanel.value = false
    }, 300)
  }

  function handleGlobalDragOver(event: DragEvent) {
    event.preventDefault()
    if (event.dataTransfer)
      event.dataTransfer.dropEffect = 'move'
  }

  function startResize(event: MouseEvent) {
    isResizing.value = true
    event.preventDefault()
  }

  function handleMouseMove(event: MouseEvent) {
    if (!isResizing.value)
      return

    const container = document.querySelector('.flex-1.overflow-hidden') as HTMLElement | null
    if (!container)
      return

    const containerRect = container.getBoundingClientRect()
    const newWidth = event.clientX - containerRect.left
    if (newWidth > 150 && newWidth < containerRect.width - 200)
      leftPanelWidth.value = newWidth
  }

  function stopResize() {
    isResizing.value = false
  }

  function handleResize() {
    isMobile.value = window.innerWidth < 768
    showLeftPanel.value = !isMobile.value
  }

  async function initializePage() {
    if (isBookmarkPageInitialized.value)
      return

    isBookmarkPageInitialized.value = true
    data.selectedFolder.value = '0'
    await data.refreshBookmarks()
    handleResize()
  }

  function setupLayoutListeners(handleGlobalClick: (event: MouseEvent) => void) {
    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', stopResize)
    document.addEventListener('click', handleGlobalClick)
    document.addEventListener('dragover', handleGlobalDragOver, true)
    document.addEventListener('dragenter', handleGlobalDragOver, true)
    window.addEventListener('resize', handleResize)
  }

  function cleanupLayoutListeners(handleGlobalClick: (event: MouseEvent) => void) {
    document.removeEventListener('mousemove', handleMouseMove)
    document.removeEventListener('mouseup', stopResize)
    document.removeEventListener('click', handleGlobalClick)
    document.removeEventListener('dragover', handleGlobalDragOver, true)
    document.removeEventListener('dragenter', handleGlobalDragOver, true)
    window.removeEventListener('resize', handleResize)
  }

  return {
    cleanupLayoutListeners,
    collapsePanel,
    initializePage,
    isMobile,
    isPanelExpanded,
    leftPanelWidth,
    setupLayoutListeners,
    showLeftPanel,
    startResize,
    togglePanel,
  }
}
