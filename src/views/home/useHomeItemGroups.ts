import { useHomeItemGroupContextMenu } from './useHomeItemGroupContextMenu'
import { useHomeItemGroupData } from './useHomeItemGroupData'
import type { UseHomeItemGroupsOptions } from './homeItemGroupShared'

export type {
  HomeItemGroupsAuthStore,
  HomeItemGroupsDialogApi,
  HomeItemGroupsMessageApi,
  HomeItemGroupsPanelState,
  ItemGroup,
  UseHomeItemGroupsOptions,
} from './homeItemGroupShared'

export {
  GROUP_LIST_CACHE_KEY,
  ITEM_ICON_LIST_CACHE_KEY_PREFIX,
} from './homeItemGroupShared'

export function useHomeItemGroups(options: UseHomeItemGroupsOptions) {
  const data = useHomeItemGroupData(options)
  const contextMenu = useHomeItemGroupContextMenu({
    ...options,
    handleDeleteSuccess: data.handleDeleteSuccess,
    items: data.items,
  })

  return {
    applyNetworkFilter: data.applyNetworkFilter,
    dropdownMenuX: contextMenu.dropdownMenuX,
    dropdownMenuY: contextMenu.dropdownMenuY,
    dropdownShow: contextMenu.dropdownShow,
    filterItems: data.filterItems,
    getDropdownMenuOptions: contextMenu.getDropdownMenuOptions,
    getList: data.getList,
    handleContextMenu: contextMenu.handleContextMenu,
    handleEditSuccess: data.handleEditSuccess,
    handleItemClick: data.handleItemClick,
    handleRightMenuSelect: contextMenu.handleRightMenuSelect,
    handleSaveSort: data.handleSaveSort,
    handleSearchItemSelect: data.handleSearchItemSelect,
    handleSetHoverStatus: data.handleSetHoverStatus,
    handleSetSortStatus: data.handleSetSortStatus,
    itemFrontEndSearch: data.itemFrontEndSearch,
    items: data.items,
    onClickoutside: contextMenu.onClickoutside,
    searchableItems: data.searchableItems,
  }
}
