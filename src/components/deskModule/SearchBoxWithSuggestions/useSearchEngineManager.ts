import { onMounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { useAuthStore } from '@/store'
import { useModuleConfig } from '@/store/modules'
import { VisitMode } from '@/enums/auth'
import { t } from '@/locales'
import { add, deletes, getList, update, updateSort } from '@/api/panel/searchEngine'
import { logError } from '@/utils/logger'
import { ss } from '@/utils/storage/local'
import SvgSrcBing from '@/assets/search_engine_svg/bing.svg'
import SvgSrcGoogle from '@/assets/search_engine_svg/google.svg'
import type { SearchBoxState } from './types'

const LEGACY_BAIDU_URL = 'https://www.baidu.com/s?wd=%s'
const SEARCH_ENGINE_LIST_CACHE_KEY = 'searchEngineListCache'
const moduleConfigName = 'deskModuleSearchBox'

export function useSearchEngineManager() {
  const moduleConfig = useModuleConfig()
  const authStore = useAuthStore()
  const ms = useMessage()
  const searchSelectListShow = ref(false)
  const searchEngineDialogVisible = ref(false)
  const editingSearchEngine = ref<DeskModule.SearchBox.SearchEngine | null>(null)
  const editingSearchEngineIndex = ref(-1)
  const searchEngineForm = ref({
    id: 0,
    iconSrc: '',
    title: '',
    url: '',
  })
  const draggedEngineIndex = ref<number | null>(null)
  const defaultSearchEngineList = ref<DeskModule.SearchBox.SearchEngine[]>([])

  const defaultState: SearchBoxState = {
    currentSearchEngine: defaultSearchEngineList.value[0] as DeskModule.SearchBox.SearchEngine,
    searchEngineList: defaultSearchEngineList.value,
    newWindowOpen: true,
    searchBookmarks: true,
  }

  const state = ref<SearchBoxState>({ ...defaultState })

  function isLegacyDefaultBaiduEngine(engine: DeskModule.SearchBox.SearchEngine) {
    return engine.title === 'Baidu' && engine.url === LEGACY_BAIDU_URL
  }

  async function removeLegacyDefaultBaiduEngines(engines: DeskModule.SearchBox.SearchEngine[]) {
    const legacyBaiduEngines = engines.filter(engine => engine.id && isLegacyDefaultBaiduEngine(engine))
    if (legacyBaiduEngines.length === 0)
      return engines

    for (const engine of legacyBaiduEngines) {
      if (engine.id)
        await deletes({ id: engine.id })
    }

    const { code, data } = await getList()
    if (code === 0)
      return (data && data.list) || []

    return engines.filter(engine => !isLegacyDefaultBaiduEngine(engine))
  }

  function checkCurrentEngine() {
    if (!state.value.currentSearchEngine || !state.value.currentSearchEngine.url) {
      if (defaultSearchEngineList.value.length > 0)
        state.value.currentSearchEngine = defaultSearchEngineList.value[0]
      return
    }

    const match = defaultSearchEngineList.value.find((engine) => {
      if (state.value.currentSearchEngine.id && engine.id)
        return engine.id === state.value.currentSearchEngine.id

      return engine.url === state.value.currentSearchEngine.url
    })

    if (match)
      state.value.currentSearchEngine = match
    else if (defaultSearchEngineList.value.length > 0)
      state.value.currentSearchEngine = defaultSearchEngineList.value[0]
  }

  async function createDefaultEngines() {
    const defaults = [
      {
        iconSrc: SvgSrcGoogle,
        title: 'Google',
        url: 'https://www.google.com/search?q=%s',
      },
      {
        iconSrc: SvgSrcBing,
        title: 'Bing',
        url: 'https://www.bing.com/search?q=%s',
      },
    ]

    for (const engine of defaults)
      await add(engine)

    const { code, data } = await getList()
    if (code === 0) {
      defaultSearchEngineList.value = (data && data.list) || []
      ss.set(SEARCH_ENGINE_LIST_CACHE_KEY, defaultSearchEngineList.value)
      if (defaultSearchEngineList.value.length > 0) {
        state.value.currentSearchEngine = defaultSearchEngineList.value[0]
        void moduleConfig.saveToCloud(moduleConfigName, state.value)
      }
    }
  }

  async function initSearchEngines(forceRefresh = false) {
    try {
      if (forceRefresh)
        ss.remove(SEARCH_ENGINE_LIST_CACHE_KEY)

      if (!forceRefresh) {
        const cachedData = ss.get(SEARCH_ENGINE_LIST_CACHE_KEY)
        if (cachedData) {
          defaultSearchEngineList.value = cachedData
          checkCurrentEngine()
          return
        }
      }

      const { code, data } = await getList()
      if (code === 0) {
        defaultSearchEngineList.value = await removeLegacyDefaultBaiduEngines((data && data.list) || [])
        ss.set(SEARCH_ENGINE_LIST_CACHE_KEY, defaultSearchEngineList.value)

        if (defaultSearchEngineList.value.length === 0)
          await createDefaultEngines()
        else
          checkCurrentEngine()
      }
    } catch (error) {
      logError('Failed to load search engines', error)
    }
  }

  function openSearchEngineDialog() {
    searchEngineDialogVisible.value = true
  }

  function closeSearchEngineDialog() {
    searchEngineDialogVisible.value = false
    resetSearchEngineForm()
  }

  function resetSearchEngineForm() {
    searchEngineForm.value = {
      id: 0,
      iconSrc: '',
      title: '',
      url: '',
    }
    editingSearchEngine.value = null
    editingSearchEngineIndex.value = -1
  }

  function startEditSearchEngine(engine: DeskModule.SearchBox.SearchEngine, index: number) {
    editingSearchEngine.value = engine
    editingSearchEngineIndex.value = index
    searchEngineForm.value = {
      id: engine.id || 0,
      iconSrc: engine.iconSrc,
      title: engine.title,
      url: engine.url,
    }
  }

  async function saveSearchEngine() {
    if (!searchEngineForm.value.title || !searchEngineForm.value.url)
      return

    try {
      if (editingSearchEngineIndex.value >= 0) {
        const { code } = await update({
          id: searchEngineForm.value.id,
          title: searchEngineForm.value.title,
          url: searchEngineForm.value.url,
          iconSrc: searchEngineForm.value.iconSrc,
        })
        if (code !== 0)
          return

        ms.success(t('common.saveSuccess') || '保存成功')
        closeSearchEngineDialog()
      } else {
        const { code } = await add({
          title: searchEngineForm.value.title,
          url: searchEngineForm.value.url,
          iconSrc: searchEngineForm.value.iconSrc,
        })
        if (code !== 0)
          return

        ms.success(t('common.addSuccess') || '添加成功')
        closeSearchEngineDialog()
      }
    } catch {
      ms.error(t('common.failed') || '操作失败')
      return
    }

    await initSearchEngines(true)
    resetSearchEngineForm()
  }

  async function deleteSearchEngine(index: number) {
    const engine = defaultSearchEngineList.value[index]
    if (!engine?.id)
      return

    try {
      const { code } = await deletes({ id: engine.id })
      if (code === 0) {
        ms.success(t('common.deleteSuccess') || '删除成功')
        await initSearchEngines(true)
      } else {
        ms.error(t('common.deleteFail') || '删除失败')
      }
    } catch {
      ms.error(t('common.deleteFail') || '删除失败')
    }
  }

  function handleDragStart(index: number) {
    draggedEngineIndex.value = index
  }

  async function handleDragEnd() {
    draggedEngineIndex.value = null
    const items = defaultSearchEngineList.value.map((item, index) => ({
      id: item.id!,
      sort: index + 1,
    }))

    try {
      await updateSort({ items })
    } catch (error) {
      logError('Failed to save sort order', error)
    }
  }

  function handleDragOver(event: DragEvent, index: number) {
    event.preventDefault()
    if (draggedEngineIndex.value === null || draggedEngineIndex.value === index)
      return

    const draggedItem = defaultSearchEngineList.value[draggedEngineIndex.value]
    const newList = [...defaultSearchEngineList.value]
    newList.splice(draggedEngineIndex.value, 1)
    newList.splice(index, 0, draggedItem)
    defaultSearchEngineList.value = newList
    draggedEngineIndex.value = index
  }

  function handleIconUpload(event: Event) {
    const target = event.target as HTMLInputElement
    const file = target.files?.[0]
    if (!file)
      return

    const reader = new FileReader()
    reader.onload = (loadEvent) => {
      searchEngineForm.value.iconSrc = loadEvent.target?.result as string
    }
    reader.readAsDataURL(file)
  }

  function handleEngineClick() {
    if (authStore.visitMode === VisitMode.VISIT_MODE_PUBLIC)
      return

    searchSelectListShow.value = !searchSelectListShow.value
  }

  function handleEngineUpdate(engine: DeskModule.SearchBox.SearchEngine) {
    state.value.currentSearchEngine = engine
    void moduleConfig.saveToCloud(moduleConfigName, state.value)
    searchSelectListShow.value = false
  }

  onMounted(() => {
    moduleConfig.getValueByNameFromCloud<SearchBoxState>(moduleConfigName).then(({ code, data }) => {
      if (code === 0)
        state.value = { ...defaultState, ...(data || {}) }
      else
        state.value = { ...defaultState }

      void initSearchEngines()
    })
  })

  return {
    closeSearchEngineDialog,
    defaultSearchEngineList,
    deleteSearchEngine,
    draggedEngineIndex,
    editingSearchEngineIndex,
    handleDragEnd,
    handleDragOver,
    handleDragStart,
    handleEngineClick,
    handleEngineUpdate,
    handleIconUpload,
    moduleConfig,
    moduleConfigName,
    openSearchEngineDialog,
    resetSearchEngineForm,
    saveSearchEngine,
    searchEngineDialogVisible,
    searchEngineForm,
    searchSelectListShow,
    startEditSearchEngine,
    state,
  }
}
