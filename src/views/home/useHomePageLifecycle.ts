import { onActivated, onMounted, watch, type Ref } from 'vue'
import { onBeforeRouteUpdate } from 'vue-router'
import { PanelStateNetworkModeEnum } from '@/enums'
import { VisitMode } from '@/enums/auth'
import { t } from '@/locales'
import { setTitle, updateLocalUserInfo } from '@/utils/cmn'
import { logError } from '@/utils/logger'
import { ss } from '@/utils/storage/local'
import { BOOKMARKS_CACHE_KEY } from './useHomeBookmarks'
import { GROUP_LIST_CACHE_KEY, ITEM_ICON_LIST_CACHE_KEY_PREFIX } from './useHomeItemGroups'

interface HomePageLifecycleMessageApi {
  error: (content: string) => void
  success: (content: string) => void
}

interface HomePageLifecycleAuthStore {
  userInfo?: {
    mustChangePassword?: boolean
  } | null
  visitMode: VisitMode
}

interface HomePageLifecyclePanelState {
  panelConfig: {
    autoNetworkWallpaper?: boolean
    autoNetworkWallpaperApi?: string
    backgroundImageSrc?: string
    logoText?: string
  }
  recordState: () => void
  setNetworkMode: (mode: PanelStateNetworkModeEnum) => void
  updatePanelConfigByCloud: () => void
}

interface UseHomePageLifecycleOptions {
  applyNetworkFilter: () => void
  authStore: HomePageLifecycleAuthStore
  checkIsLanFromServer: () => Promise<boolean>
  getList: (forceRefresh?: boolean) => Promise<void>
  loadBookmarkTree: (forceRefresh?: boolean) => Promise<void>
  message: HomePageLifecycleMessageApi
  notepadInstance: Ref<any>
  panelState: HomePageLifecyclePanelState
  settingModalShow: Ref<boolean>
}

export function useHomePageLifecycle(options: UseHomePageLifecycleOptions) {
  const {
    applyNetworkFilter,
    authStore,
    checkIsLanFromServer,
    getList,
    loadBookmarkTree,
    message,
    notepadInstance,
    panelState,
    settingModalShow,
  } = options

  async function handleRefreshData() {
    try {
      ss.remove(BOOKMARKS_CACHE_KEY)
      ss.remove(GROUP_LIST_CACHE_KEY)
      ss.remove('searchEngineListCache')

      Object.keys(localStorage).forEach((key) => {
        if (key.startsWith(ITEM_ICON_LIST_CACHE_KEY_PREFIX))
          ss.remove(key)
      })

      await Promise.all([
        getList(true),
        loadBookmarkTree(true),
      ])

      if (authStore.visitMode === VisitMode.VISIT_MODE_LOGIN)
        await notepadInstance.value?.refreshData?.()

      if (panelState.panelConfig.autoNetworkWallpaper) {
        try {
          const timestamp = Date.now()
          const baseUrl = panelState.panelConfig.autoNetworkWallpaperApi || 'https://img.xjh.me/random_img.php?return=302&type=bg&ctype=nature'
          panelState.panelConfig.backgroundImageSrc = baseUrl.includes('?') ? `${baseUrl}&t=${timestamp}` : `${baseUrl}?t=${timestamp}`
          panelState.recordState()
        } catch (error) {
          logError('重新获取网络壁纸失败', error)
        }
      }

      message.success(t('common.refreshSuccess'))
    } catch (error) {
      logError('刷新数据失败', error)
      message.error(t('common.refreshFailed'))
    }
  }

  watch(settingModalShow, async (visible, previous) => {
    if (previous && !visible) {
      await handleRefreshData()
      applyNetworkFilter()
    }
  })

  onMounted(async () => {
    await updateLocalUserInfo()

    if (authStore.userInfo?.mustChangePassword) {
      settingModalShow.value = true
      return
    }

    void getList()
    panelState.updatePanelConfigByCloud()

    if (panelState.panelConfig.autoNetworkWallpaper) {
      try {
        panelState.panelConfig.backgroundImageSrc = panelState.panelConfig.autoNetworkWallpaperApi || 'https://img.xjh.me/random_img.php?return=302&type=bg&ctype=nature'
        panelState.recordState()
      } catch (error) {
        logError('自动获取网络壁纸失败', error)
      }
    }

    if (panelState.panelConfig.logoText)
      setTitle(panelState.panelConfig.logoText)

    if (authStore.visitMode === VisitMode.VISIT_MODE_PUBLIC) {
      panelState.setNetworkMode(PanelStateNetworkModeEnum.wan)
    } else {
      const isLan = await checkIsLanFromServer()
      panelState.setNetworkMode(isLan ? PanelStateNetworkModeEnum.lan : PanelStateNetworkModeEnum.wan)
    }

    await loadBookmarkTree(false)
  })

  onActivated(() => {
    if (authStore.userInfo?.mustChangePassword)
      return

    setTimeout(() => {
      void loadBookmarkTree(false)
    }, 20)
  })

  onActivated(() => {
    if (authStore.userInfo?.mustChangePassword)
      return

    void loadBookmarkTree(true)
  })

  onBeforeRouteUpdate(() => {
    if (authStore.userInfo?.mustChangePassword)
      return

    void loadBookmarkTree(true)
  })

  return {
    handleRefreshData,
  }
}
