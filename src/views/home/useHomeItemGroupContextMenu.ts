import { nextTick, ref } from 'vue'
import { deletes } from '@/api/panel/itemIcon'
import { PanelStateNetworkModeEnum } from '@/enums'
import { VisitMode } from '@/enums/auth'
import { t } from '@/locales'
import { openUrlWithoutReferer } from '@/utils/cmn'
import { resolveApiErrorMessage } from '@/utils/request/apiMessage'
import {
  type ItemGroup,
  type UseHomeItemGroupsOptions,
} from './homeItemGroupShared'

interface UseHomeItemGroupContextMenuOptions extends UseHomeItemGroupsOptions {
  handleDeleteSuccess: (itemIconGroupId: number | undefined) => void
  items: { value: ItemGroup[] }
}

export function useHomeItemGroupContextMenu(options: UseHomeItemGroupContextMenuOptions) {
  const { authStore, dialog, handleDeleteSuccess, items, message, onEditItem, openPage, panelState } = options
  const dropdownMenuX = ref(0)
  const dropdownMenuY = ref(0)
  const dropdownShow = ref(false)
  const currentRightSelectItem = ref<Panel.ItemInfo | null>(null)

  function handleRightMenuSelect(key: string | number) {
    dropdownShow.value = false
    const selectedItem = currentRightSelectItem.value
    if (!selectedItem)
      return

    let jumpUrl = panelState.networkMode === PanelStateNetworkModeEnum.lan ? selectedItem.lanUrl : selectedItem.url
    if (selectedItem.lanUrl === '')
      jumpUrl = selectedItem.url

    switch (key) {
      case 'newWindows':
        if (jumpUrl)
          openUrlWithoutReferer(jumpUrl, '_blank')
        break
      case 'openWanUrl':
        openPage(selectedItem.openMethod, selectedItem.url, selectedItem.title)
        break
      case 'openLanUrl':
        if (selectedItem.lanUrl)
          openPage(selectedItem.openMethod, selectedItem.lanUrl, selectedItem.title)
        break
      case 'edit':
        onEditItem({ ...selectedItem } as Panel.ItemInfo)
        break
      case 'delete':
        dialog.warning({
          title: t('common.warning'),
          content: t('common.deleteConfirmByName', { name: selectedItem.title }),
          positiveText: t('common.confirm'),
          negativeText: t('common.cancel'),
          onPositiveClick: () => {
            const itemIconGroupId = selectedItem.itemIconGroupId
            deletes([selectedItem.id as number]).then(({ code, msg }) => {
              if (code === 0) {
                handleDeleteSuccess(itemIconGroupId)
              } else {
                message.error(resolveApiErrorMessage({ code, msg }))
              }
            })
          },
        })
        break
      default:
        break
    }
  }

  function handleContextMenu(event: MouseEvent, itemGroupIndex: number, item: Panel.ItemInfo) {
    if (items.value[itemGroupIndex]?.sortStatus)
      return

    event.preventDefault()
    currentRightSelectItem.value = item
    dropdownShow.value = false
    nextTick().then(() => {
      dropdownShow.value = true
      dropdownMenuX.value = event.clientX
      dropdownMenuY.value = event.clientY
    })
  }

  function onClickoutside() {
    dropdownShow.value = false
  }

  function getDropdownMenuOptions() {
    const dropdownMenuOptions = [
      {
        label: t('iconItem.newWindowOpen'),
        key: 'newWindows',
      },
    ]

    if (currentRightSelectItem.value?.url) {
      dropdownMenuOptions.push({
        label: t('panelHome.openWanUrl'),
        key: 'openWanUrl',
      })
    }

    if (currentRightSelectItem.value?.lanUrl) {
      dropdownMenuOptions.push({
        label: t('panelHome.openLanUrl'),
        key: 'openLanUrl',
      })
    }

    if (authStore.visitMode === VisitMode.VISIT_MODE_LOGIN) {
      dropdownMenuOptions.push(
        {
          label: t('common.edit'),
          key: 'edit',
        },
        {
          label: t('common.delete'),
          key: 'delete',
        },
      )
    }

    return dropdownMenuOptions
  }

  return {
    dropdownMenuX,
    dropdownMenuY,
    dropdownShow,
    getDropdownMenuOptions,
    handleContextMenu,
    handleRightMenuSelect,
    onClickoutside,
  }
}
