import { computed, ref } from 'vue'
import { edit as editItem, getListByGroupId, getSiteFavicon, saveSort } from '@/api/panel/itemIcon'
import { getList as getGroupList } from '@/api/panel/itemIconGroup'
import { PanelStateNetworkModeEnum } from '@/enums'
import { VisitMode } from '@/enums/auth'
import { t } from '@/locales'
import { logError } from '@/utils/logger'
import { ss } from '@/utils/storage/local'
import {
  GROUP_LIST_CACHE_KEY,
  ITEM_ICON_LIST_CACHE_KEY_PREFIX,
  matchesItemSearchKeyword,
  resolveJumpUrl,
  type ItemGroup,
  type UseHomeItemGroupsOptions,
} from './homeItemGroupShared'

export function useHomeItemGroupData(options: UseHomeItemGroupsOptions) {
  const { authStore, panelState, message, openPage, onEditItem } = options
  const items = ref<ItemGroup[]>([])
  const filterItems = ref<ItemGroup[]>([])
  const searchableItems = computed(() => items.value.flatMap(group => group.items || []))

  async function refreshItemIconInBackground(item: Panel.ItemInfo, targetUrl: string) {
    if (authStore.visitMode !== VisitMode.VISIT_MODE_LOGIN)
      return

    if (!targetUrl || !item.itemIconGroupId)
      return

    const shouldAutoRefresh = !item.icon
      || (item.icon.itemType === 2 && (!item.icon.src || item.icon.src.startsWith('data:')))

    if (!shouldAutoRefresh)
      return

    try {
      const { code, data } = await getSiteFavicon<{ iconUrl: string }>(targetUrl)
      if (code !== 0 || !data?.iconUrl)
        return

      const currentIconSrc = item.icon?.itemType === 2 ? item.icon.src || '' : ''
      if (currentIconSrc === data.iconUrl)
        return

      const updatedIcon: Panel.ItemIcon = {
        itemType: 2,
        src: data.iconUrl,
        backgroundColor: item.icon?.backgroundColor || '#2a2a2a6b',
      }

      const payload: Panel.ItemInfo = {
        ...item,
        icon: updatedIcon,
      }

      const response = await editItem<Panel.ItemInfo>(payload)
      if (response.code === 0)
        item.icon = updatedIcon
    } catch (error) {
      logError('刷新主页图标失败', error)
    }
  }

  function applyNetworkFilter() {
    if (panelState.networkMode === PanelStateNetworkModeEnum.wan) {
      filterItems.value = items.value.map((group) => {
        if (!group.items)
          return group

        return {
          ...group,
          items: group.items.filter(item => item.lanOnly !== 1),
        }
      })
      return
    }

    filterItems.value = items.value
  }

  async function updateItemIconGroupByNet(itemIconGroupIndex: number, itemIconGroupId: number) {
    if (authStore.userInfo?.mustChangePassword)
      return

    try {
      const cacheKey = `${ITEM_ICON_LIST_CACHE_KEY_PREFIX}${itemIconGroupId}`
      const cachedData = ss.get(cacheKey)
      if (cachedData) {
        if (items.value[itemIconGroupIndex])
          items.value[itemIconGroupIndex].items = cachedData

        if (items.value.every(group => group.items !== undefined))
          applyNetworkFilter()
        return
      }

      const res = await getListByGroupId<Common.ListResponse<Panel.ItemInfo[]>>(itemIconGroupId)
      if (res.code === 0 && items.value[itemIconGroupIndex]) {
        items.value[itemIconGroupIndex].items = res.data.list
        ss.set(cacheKey, res.data.list)

        if (items.value.every(group => group.items !== undefined))
          applyNetworkFilter()
      }
    } catch (error) {
      logError('获取首页分组图标失败', error)
      const cacheKey = `${ITEM_ICON_LIST_CACHE_KEY_PREFIX}${itemIconGroupId}`
      const cachedData = ss.get(cacheKey)
      if (cachedData && items.value[itemIconGroupIndex]) {
        items.value[itemIconGroupIndex].items = cachedData
        if (items.value.every(group => group.items !== undefined))
          applyNetworkFilter()
      }
    }
  }

  async function getList(forceRefresh = false) {
    if (authStore.userInfo?.mustChangePassword) {
      items.value = []
      filterItems.value = []
      return
    }

    try {
      const cachedData = forceRefresh ? null : ss.get(GROUP_LIST_CACHE_KEY)
      if (cachedData) {
        items.value = cachedData
        for (let index = 0; index < cachedData.length; index += 1) {
          const group = cachedData[index]
          if (group.id)
            void updateItemIconGroupByNet(index, group.id)
        }
        applyNetworkFilter()
        return
      }

      const response = await getGroupList<Common.ListResponse<ItemGroup[]>>()
      if (response.code === 0) {
        items.value = response.data.list
        ss.set(GROUP_LIST_CACHE_KEY, response.data.list)

        for (let index = 0; index < response.data.list.length; index += 1) {
          const group = response.data.list[index]
          if (group.id)
            void updateItemIconGroupByNet(index, group.id)
        }

        applyNetworkFilter()
      }
    } catch (error) {
      logError('获取首页分组失败', error)
      const cachedData = forceRefresh ? null : ss.get(GROUP_LIST_CACHE_KEY)
      if (cachedData) {
        items.value = cachedData
        for (let index = 0; index < cachedData.length; index += 1) {
          const group = cachedData[index]
          if (group.id)
            void updateItemIconGroupByNet(index, group.id)
        }
        applyNetworkFilter()
      }
    }
  }

  function itemFrontEndSearch(keyword?: string) {
    const trimmedKeyword = keyword?.trim() || ''
    if (trimmedKeyword !== '' && panelState.panelConfig.searchBoxSearchIcon && items.value.length > 0) {
      const lowerCaseKeyword = trimmedKeyword.toLowerCase()
      const filteredData: ItemGroup[] = []

      for (let index = 0; index < items.value.length; index += 1) {
        const group = items.value[index]
        const groupItems = group.items?.filter((item) => {
          const networkModeMatch = panelState.networkMode !== PanelStateNetworkModeEnum.wan || item.lanOnly !== 1
          return networkModeMatch && matchesItemSearchKeyword(item, lowerCaseKeyword)
        })

        if (groupItems && groupItems.length > 0)
          filteredData.push({ items: groupItems, hoverStatus: false })
      }

      filterItems.value = filteredData
      return
    }

    applyNetworkFilter()
  }

  function handleItemClick(itemGroupIndex: number, item: Panel.ItemInfo) {
    if (items.value[itemGroupIndex]?.sortStatus) {
      onEditItem(item)
      return
    }

    const jumpUrl = resolveJumpUrl(item, panelState.networkMode)
    openPage(item.openMethod, jumpUrl, item.title)
    void refreshItemIconInBackground(item, jumpUrl)
  }

  function handleSearchItemSelect(item: Panel.ItemInfo) {
    if (!item)
      return

    const jumpUrl = resolveJumpUrl(item, panelState.networkMode)
    openPage(item.openMethod, jumpUrl, item.title)
    void refreshItemIconInBackground(item, jumpUrl)
  }

  function handleEditSuccess(item: Panel.ItemInfo) {
    for (let index = 0; index < items.value.length; index += 1) {
      const group = items.value[index]
      if (group.id === item.itemIconGroupId) {
        ss.remove(`${ITEM_ICON_LIST_CACHE_KEY_PREFIX}${item.itemIconGroupId}`)
        break
      }
    }

    void getList(true)
  }

  function handleSaveSort(itemGroup: ItemGroup) {
    const saveItems: Common.SortItemRequest[] = []
    if (!itemGroup.items)
      return

    for (let index = 0; index < itemGroup.items.length; index += 1) {
      const element = itemGroup.items[index]
      saveItems.push({
        id: element.id as number,
        sort: index + 1,
      })
    }

    saveSort({ itemIconGroupId: itemGroup.id as number, sortItems: saveItems }).then(({ code, msg }) => {
      if (code === 0) {
        ss.remove(`${ITEM_ICON_LIST_CACHE_KEY_PREFIX}${itemGroup.id}`)
      } else {
        logError(`${t('common.saveFail')}:${msg}`)
      }
    })
  }

  function handleSetHoverStatus(groupIndex: number, hoverStatus: boolean) {
    if (items.value[groupIndex])
      items.value[groupIndex].hoverStatus = hoverStatus
  }

  function handleSetSortStatus(itemGroup: ItemGroup, sortStatus: boolean) {
    itemGroup.sortStatus = sortStatus

    if (!sortStatus && itemGroup.id) {
      const index = items.value.findIndex(group => group.id === itemGroup.id)
      if (index !== -1)
        void updateItemIconGroupByNet(index, itemGroup.id as number)
    }
  }

  function handleDeleteSuccess(itemIconGroupId: number | undefined) {
    message.success(t('common.deleteSuccess'))
    ss.remove(`${ITEM_ICON_LIST_CACHE_KEY_PREFIX}${itemIconGroupId}`)
    void getList(true)
  }

  return {
    applyNetworkFilter,
    filterItems,
    getList,
    handleDeleteSuccess,
    handleEditSuccess,
    handleItemClick,
    handleSaveSort,
    handleSearchItemSelect,
    handleSetHoverStatus,
    handleSetSortStatus,
    itemFrontEndSearch,
    items,
    searchableItems,
  }
}
