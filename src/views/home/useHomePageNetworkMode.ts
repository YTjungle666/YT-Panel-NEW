import { h, ref, type Ref } from 'vue'
import { checkIsLan } from '@/api/system/network'
import { PanelStateNetworkModeEnum } from '@/enums'
import { t } from '@/locales'
import { logError } from '@/utils/logger'
import { getLanPingUrl, isPrivateHostname, testLanPingUrl } from '@/utils/network'
import type { ItemGroup } from './homeItemGroupShared'

interface HomePageNetworkMessageApi {
  error: (content: string) => void
  success: (content: string) => void
  warning: (content: string) => void
}

interface HomePageNetworkDialogApi {
  create: (options: {
    title: string
    content: () => any
    positiveText: string
    negativeText: string
    onPositiveClick: () => Promise<boolean> | boolean
  }) => void
}

interface HomePageNetworkAuthStore {
  userInfo?: {
    username?: string
  } | null
}

interface HomePageNetworkPanelState {
  networkMode: PanelStateNetworkModeEnum
  setNetworkMode: (mode: PanelStateNetworkModeEnum) => void
}

interface UseHomePageNetworkModeOptions {
  applyNetworkFilter: () => void
  authStore: HomePageNetworkAuthStore
  dialog: HomePageNetworkDialogApi
  handleSaveSort: (itemGroup: ItemGroup) => void
  items: Ref<ItemGroup[]>
  message: HomePageNetworkMessageApi
  panelState: HomePageNetworkPanelState
}

export function useHomePageNetworkMode(options: UseHomePageNetworkModeOptions) {
  const { applyNetworkFilter, authStore, dialog, handleSaveSort, items, message, panelState } = options
  let isLanCached: boolean | null = null

  async function checkIsLanFromServer() {
    if (isLanCached !== null)
      return isLanCached

    if (isPrivateHostname(window.location.hostname)) {
      isLanCached = true
      return true
    }

    try {
      const { data, code } = await checkIsLan()
      const lanData = data as any
      if (code === 0 && (lanData?.isLan === true || lanData === true)) {
        isLanCached = true
        return true
      }
    } catch {
      // noop
    }

    const pingUrl = getLanPingUrl()
    if (pingUrl) {
      isLanCached = await testLanPingUrl(pingUrl)
      return isLanCached
    }

    isLanCached = false
    return false
  }

  function finalizeSortBeforeModeChange() {
    items.value.forEach((group) => {
      if (group.sortStatus) {
        handleSaveSort(group)
        group.sortStatus = false
      }
    })
  }

  function handleChangeNetwork(targetMode: PanelStateNetworkModeEnum) {
    if (panelState.networkMode === PanelStateNetworkModeEnum.wan && targetMode === PanelStateNetworkModeEnum.lan) {
      const passwordInput = ref('')

      dialog.create({
        title: t('panelHome.verifyPassword'),
        content: () => h('div', { class: 'mt-4' }, [
          h('div', { class: 'mb-2 text-sm text-gray-600 dark:text-gray-400' }, t('panelHome.enterPasswordToSwitchLan')),
          h('input', {
            type: 'password',
            class: 'w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white',
            placeholder: t('common.password'),
            value: passwordInput.value,
            onInput: (event: Event) => {
              passwordInput.value = (event.target as HTMLInputElement).value
            },
            onKeydown: (event: KeyboardEvent) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                const positiveButton = document.querySelector('.n-dialog__action button:last-child') as HTMLButtonElement | null
                positiveButton?.click()
              }
            },
          }),
        ]),
        positiveText: t('common.confirm'),
        negativeText: t('common.cancel'),
        onPositiveClick: async () => {
          if (!passwordInput.value) {
            message.warning(t('panelHome.passwordRequired'))
            return false
          }

          try {
            const response = await fetch('/api/login', {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
              },
              credentials: 'include',
              body: JSON.stringify({
                username: authStore.userInfo?.username,
                password: passwordInput.value,
              }),
            })

            const result = await response.json()
            if (result.code === 0) {
              finalizeSortBeforeModeChange()
              panelState.setNetworkMode(targetMode)
              message.success(t('panelHome.changeToLanModelSuccess'))
              applyNetworkFilter()
              return true
            }

            message.error(t('panelHome.passwordIncorrect'))
            return false
          } catch (error) {
            logError('验证密码失败', error)
            message.error(t('common.networkError'))
            return false
          }
        },
      })

      return
    }

    finalizeSortBeforeModeChange()
    panelState.setNetworkMode(targetMode)
    applyNetworkFilter()
    if (targetMode === PanelStateNetworkModeEnum.wan)
      message.success(t('panelHome.changeToWanModelSuccess'))
  }

  return {
    checkIsLanFromServer,
    handleChangeNetwork,
  }
}
