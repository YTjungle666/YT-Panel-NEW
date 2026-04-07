import { computed } from 'vue'
import { useDialog, useMessage } from 'naive-ui'
import { useAuthStore, usePanelState } from '@/store'
import { sanitizeFooterHtml } from '@/utils/sanitize'
import { useHomeBookmarks } from './useHomeBookmarks'
import { useHomeItemGroups } from './useHomeItemGroups'
import { useHomePageLifecycle } from './useHomePageLifecycle'
import { useHomePageNetworkMode } from './useHomePageNetworkMode'
import { useHomePageViewState } from './useHomePageViewState'

export function useHomePage() {
  const ms = useMessage()
  const dialog = useDialog()
  const panelState = usePanelState()
  const authStore = useAuthStore()
  const viewState = useHomePageViewState()
  const safeFooterHtml = computed(() => {
    const footerHtml = panelState.panelConfig.footerHtml || ''
    if (/powered\s*by/i.test(footerHtml) && /sun-panel/i.test(footerHtml))
      return ''

    return sanitizeFooterHtml(footerHtml)
  })

  const {
    treeData,
    loadBookmarkTree,
    navigateToBookmarkManager,
    handleTreeSelect,
    renderTreeLabel,
  } = useHomeBookmarks(authStore)

  const itemGroups = useHomeItemGroups({
    authStore,
    panelState,
    dialog,
    message: ms,
    openPage: viewState.openPage,
    onEditItem: viewState.handleEditItem,
  })

  const networkMode = useHomePageNetworkMode({
    applyNetworkFilter: itemGroups.applyNetworkFilter,
    authStore,
    dialog,
    handleSaveSort: itemGroups.handleSaveSort,
    items: itemGroups.items,
    message: ms,
    panelState,
  })

  const lifecycle = useHomePageLifecycle({
    applyNetworkFilter: itemGroups.applyNetworkFilter,
    authStore,
    checkIsLanFromServer: networkMode.checkIsLanFromServer,
    getList: itemGroups.getList,
    loadBookmarkTree,
    message: ms,
    notepadInstance: viewState.notepadInstance,
    panelState,
    settingModalShow: viewState.settingModalShow,
  })

  return {
    authStore,
    currentAddItenIconGroupId: viewState.currentAddItenIconGroupId,
    dialog,
    drawerVisible: viewState.drawerVisible,
    dropdownMenuX: itemGroups.dropdownMenuX,
    dropdownMenuY: itemGroups.dropdownMenuY,
    dropdownShow: itemGroups.dropdownShow,
    editItemInfoData: viewState.editItemInfoData,
    editItemInfoShow: viewState.editItemInfoShow,
    filterItems: itemGroups.filterItems,
    getDropdownMenuOptions: itemGroups.getDropdownMenuOptions,
    getScrollListenTarget: viewState.getScrollListenTarget,
    handWindowIframeIdLoad: viewState.handWindowIframeIdLoad,
    handleAddItem: viewState.handleAddItem,
    handleChangeNetwork: networkMode.handleChangeNetwork,
    handleContextMenu: itemGroups.handleContextMenu,
    handleEditSuccess: itemGroups.handleEditSuccess,
    handleRefreshData: lifecycle.handleRefreshData,
    handleItemClick: itemGroups.handleItemClick,
    handleRightMenuSelect: itemGroups.handleRightMenuSelect,
    handleSearchItemSelect: itemGroups.handleSearchItemSelect,
    handleSaveSort: itemGroups.handleSaveSort,
    handleSetHoverStatus: itemGroups.handleSetHoverStatus,
    handleSetSortStatus: itemGroups.handleSetSortStatus,
    handleTreeSelect,
    isMobile: viewState.isMobile,
    itemFrontEndSearch: itemGroups.itemFrontEndSearch,
    navigateToBookmarkManager,
    notepadInstance: viewState.notepadInstance,
    notepadVisible: viewState.notepadVisible,
    onClickoutside: itemGroups.onClickoutside,
    panelState,
    renderTreeLabel,
    safeFooterHtml,
    scrollContainerRef: viewState.scrollContainerRef,
    searchableItems: itemGroups.searchableItems,
    settingModalShow: viewState.settingModalShow,
    treeData,
    windowIframeIsLoad: viewState.windowIframeIsLoad,
    windowShow: viewState.windowShow,
    windowSrc: viewState.windowSrc,
    windowTitle: viewState.windowTitle,
  }
}
