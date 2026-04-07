import { computed, ref } from 'vue'
import { useWindowSize } from '@vueuse/core'
import { openUrlWithoutReferer } from '@/utils/cmn'

export function useHomePageViewState() {
  const scrollContainerRef = ref<HTMLElement | null>(null)
  const editItemInfoShow = ref(false)
  const editItemInfoData = ref<Panel.ItemInfo | null>(null)
  const windowShow = ref(false)
  const windowSrc = ref('')
  const windowTitle = ref('')
  const windowIframeIsLoad = ref(false)
  const settingModalShow = ref(false)
  const drawerVisible = ref(false)
  const notepadVisible = ref(false)
  const notepadInstance = ref<any>(null)
  const currentAddItenIconGroupId = ref<number | undefined>()
  const { width } = useWindowSize()
  const isMobile = computed(() => width.value < 768)

  function openPage(openMethod: number, url: string, title?: string) {
    switch (openMethod) {
      case 1:
        window.location.replace(url)
        break
      case 2:
        openUrlWithoutReferer(url, '_blank')
        break
      case 3:
        windowShow.value = true
        windowSrc.value = url
        windowTitle.value = title || url
        windowIframeIsLoad.value = true
        break
      default:
        break
    }
  }

  function handleEditItem(item: Panel.ItemInfo) {
    editItemInfoData.value = item
    editItemInfoShow.value = true
    currentAddItenIconGroupId.value = undefined
  }

  function handleAddItem(itemIconGroupId?: number) {
    editItemInfoData.value = null
    editItemInfoShow.value = true
    currentAddItenIconGroupId.value = itemIconGroupId
  }

  function handWindowIframeIdLoad() {
    windowIframeIsLoad.value = false
  }

  function getScrollListenTarget() {
    return scrollContainerRef.value || document.body
  }

  return {
    currentAddItenIconGroupId,
    drawerVisible,
    editItemInfoData,
    editItemInfoShow,
    getScrollListenTarget,
    handWindowIframeIdLoad,
    handleAddItem,
    handleEditItem,
    isMobile,
    notepadInstance,
    notepadVisible,
    openPage,
    scrollContainerRef,
    settingModalShow,
    windowIframeIsLoad,
    windowShow,
    windowSrc,
    windowTitle,
  }
}
