import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { openUrlWithoutReferer } from '@/utils/cmn'
import {
  dedupeSuggestions,
  getDefaultSuggestions,
  searchBookmarkSuggestions,
  searchTextIconItems,
} from './searchSuggestionUtils'
import type {
  SearchBoxWithSuggestionsEmit,
  SearchBoxWithSuggestionsProps,
  SuggestionItem,
} from './types'
import { useSearchEngineManager } from './useSearchEngineManager'

export type {
  SearchBoxState,
  SearchBoxWithSuggestionsEmit,
  SearchBoxWithSuggestionsProps,
  SuggestionItem,
} from './types'

export function useSearchBoxWithSuggestions(
  props: SearchBoxWithSuggestionsProps,
  emits: SearchBoxWithSuggestionsEmit,
) {
  const engineManager = useSearchEngineManager()
  const searchTerm = ref('')
  const isFocused = ref(false)
  const suggestionsVisible = ref(false)
  const dropdownPosition = ref<'bottom' | 'top'>('bottom')
  const searchInputRef = ref<HTMLInputElement | null>(null)
  const dropdownRef = ref<HTMLDivElement | null>(null)
  const suggestionOptions = ref<SuggestionItem[]>([])
  const selectedIndex = ref(-1)
  const loadingSuggestions = ref(false)
  const suggestionCache = new Map<string, SuggestionItem[]>()

  let latestSuggestionRequestId = 0
  let suggestionDebounceTimer: ReturnType<typeof setTimeout> | null = null
  let blurHideTimer: ReturnType<typeof setTimeout> | null = null

  const filteredSuggestions = computed(() => suggestionOptions.value.slice(0, 8))

  watch(searchTerm, (newValue) => {
    selectedIndex.value = -1

    if (suggestionDebounceTimer) {
      clearTimeout(suggestionDebounceTimer)
      suggestionDebounceTimer = null
    }

    if (newValue) {
      suggestionDebounceTimer = setTimeout(() => {
        suggestionDebounceTimer = null
        void fetchSuggestions(newValue)
      }, 250)
    } else {
      suggestionOptions.value = []
    }
  })

  async function fetchSuggestions(keyword: string) {
    const trimmedKeyword = keyword.trim()
    if (!trimmedKeyword)
      return

    const requestId = ++latestSuggestionRequestId
    const engineTitle = engineManager.state.value.currentSearchEngine?.title || 'default'
    const cacheKey = `${engineTitle}:${trimmedKeyword}`
    const cachedSuggestions = suggestionCache.get(cacheKey)
    if (cachedSuggestions) {
      suggestionOptions.value = cachedSuggestions
      return
    }

    loadingSuggestions.value = true
    try {
      const bookmarkSuggestions = engineManager.state.value.searchBookmarks ? searchBookmarkSuggestions(trimmedKeyword) : []
      const itemSuggestions = engineManager.state.value.searchBookmarks
        ? searchTextIconItems(props.searchItems || [], trimmedKeyword)
        : []
      const searchEngineSuggestions = trimmedKeyword.length >= 2 ? getDefaultSuggestions(trimmedKeyword) : []
      const allSuggestions = dedupeSuggestions([
        ...bookmarkSuggestions,
        ...itemSuggestions,
        ...searchEngineSuggestions,
      ])

      if (requestId !== latestSuggestionRequestId)
        return

      suggestionOptions.value = allSuggestions
      suggestionCache.set(cacheKey, allSuggestions)
    } catch {
      const allSuggestions = dedupeSuggestions([
        ...(engineManager.state.value.searchBookmarks ? searchBookmarkSuggestions(trimmedKeyword) : []),
        ...(engineManager.state.value.searchBookmarks ? searchTextIconItems(props.searchItems || [], trimmedKeyword) : []),
        ...getDefaultSuggestions(trimmedKeyword),
      ])

      if (requestId !== latestSuggestionRequestId)
        return

      suggestionOptions.value = allSuggestions
    } finally {
      if (requestId === latestSuggestionRequestId)
        loadingSuggestions.value = false
    }
  }

  function onFocus() {
    isFocused.value = true
    suggestionsVisible.value = true
    nextTick(() => {
      calculateDropdownPosition()
    })

    if (searchTerm.value)
      void fetchSuggestions(searchTerm.value)
  }

  function onBlur() {
    if (blurHideTimer)
      clearTimeout(blurHideTimer)

    blurHideTimer = setTimeout(() => {
      isFocused.value = false
      suggestionsVisible.value = false
      blurHideTimer = null
    }, 200)
  }

  function calculateDropdownPosition() {
    if (!searchInputRef.value)
      return

    const inputRect = searchInputRef.value.getBoundingClientRect()
    const spaceBelow = window.innerHeight - inputRect.bottom
    const dropdownHeight = 200

    dropdownPosition.value = spaceBelow < dropdownHeight && inputRect.top > dropdownHeight ? 'top' : 'bottom'
  }

  function getSearchOpenTarget() {
    return engineManager.state.value.newWindowOpen ? '_blank' : '_self'
  }

  function replaceOrAppendKeywordToUrl(url: string, keyword: string) {
    if (url.includes('%s'))
      return url.replace('%s', encodeURIComponent(keyword))

    return url + (keyword ? encodeURIComponent(keyword) : '')
  }

  function handleClearSearchTerm() {
    searchTerm.value = ''
    ;(emits as (event: 'itemSearch', keyword: string) => void)('itemSearch', searchTerm.value)
    suggestionsVisible.value = false
    suggestionOptions.value = []
    selectedIndex.value = -1
  }

  function handleSearchClick() {
    if (!searchTerm.value.trim())
      return

    const engineUrl = engineManager.state.value.currentSearchEngine?.url
      || engineManager.defaultSearchEngineList.value[0]?.url
    if (!engineUrl)
      return

    const fullUrl = replaceOrAppendKeywordToUrl(engineUrl, searchTerm.value)
    handleClearSearchTerm()
    openUrlWithoutReferer(fullUrl, getSearchOpenTarget())
  }

  function handleSuggestionSelect(suggestion: SuggestionItem) {
    if (suggestion.type === 'item' && suggestion.item) {
      ;(emits as (event: 'itemSelect', item: Panel.ItemInfo) => void)('itemSelect', suggestion.item)
      handleClearSearchTerm()
      return
    }

    if (suggestion.isBookmark && suggestion.url) {
      openUrlWithoutReferer(suggestion.url, getSearchOpenTarget())
      handleClearSearchTerm()
      return
    }

    searchTerm.value = suggestion.value
    suggestionsVisible.value = false
    nextTick(() => {
      handleSearchClick()
    })
  }

  function handleItemSearch() {
    ;(emits as (event: 'itemSearch', keyword: string) => void)('itemSearch', searchTerm.value)
    suggestionsVisible.value = true
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.isComposing)
      return

    if (event.key === 'Enter') {
      if (!searchTerm.value.trim()) {
        event.preventDefault()
        event.stopPropagation()
        return
      }

      if (!suggestionsVisible.value || filteredSuggestions.value.length === 0 || selectedIndex.value < 0) {
        event.preventDefault()
        handleSearchClick()
        return
      }
    }

    if (!suggestionsVisible.value || filteredSuggestions.value.length === 0)
      return

    if (event.key === 'ArrowDown') {
      event.preventDefault()
      selectedIndex.value = (selectedIndex.value + 1) % filteredSuggestions.value.length
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      selectedIndex.value = (selectedIndex.value - 1 + filteredSuggestions.value.length) % filteredSuggestions.value.length
    } else if (event.key === 'Enter') {
      event.preventDefault()
      if (selectedIndex.value >= 0 && filteredSuggestions.value.length > 0)
        handleSuggestionSelect(filteredSuggestions.value[selectedIndex.value])
      else
        handleSearchClick()
    } else if (event.key === 'Escape') {
      suggestionsVisible.value = false
      selectedIndex.value = -1
    }
  }

  function handleEngineUpdate(engine: DeskModule.SearchBox.SearchEngine) {
    engineManager.handleEngineUpdate(engine)

    if (searchTerm.value)
      void fetchSuggestions(searchTerm.value)
  }

  onBeforeUnmount(() => {
    latestSuggestionRequestId += 1
    loadingSuggestions.value = false

    if (suggestionDebounceTimer) {
      clearTimeout(suggestionDebounceTimer)
      suggestionDebounceTimer = null
    }

    if (blurHideTimer) {
      clearTimeout(blurHideTimer)
      blurHideTimer = null
    }
  })

  return {
    closeSearchEngineDialog: engineManager.closeSearchEngineDialog,
    defaultSearchEngineList: engineManager.defaultSearchEngineList,
    deleteSearchEngine: engineManager.deleteSearchEngine,
    draggedEngineIndex: engineManager.draggedEngineIndex,
    dropdownPosition,
    dropdownRef,
    editingSearchEngineIndex: engineManager.editingSearchEngineIndex,
    filteredSuggestions,
    handleClearSearchTerm,
    handleDragEnd: engineManager.handleDragEnd,
    handleDragOver: engineManager.handleDragOver,
    handleDragStart: engineManager.handleDragStart,
    handleEngineClick: engineManager.handleEngineClick,
    handleEngineUpdate,
    handleIconUpload: engineManager.handleIconUpload,
    handleItemSearch,
    handleKeyDown,
    handleSearchClick,
    handleSuggestionSelect,
    isFocused,
    loadingSuggestions,
    moduleConfig: engineManager.moduleConfig,
    moduleConfigName: engineManager.moduleConfigName,
    onBlur,
    onFocus,
    openSearchEngineDialog: engineManager.openSearchEngineDialog,
    resetSearchEngineForm: engineManager.resetSearchEngineForm,
    saveSearchEngine: engineManager.saveSearchEngine,
    searchEngineDialogVisible: engineManager.searchEngineDialogVisible,
    searchEngineForm: engineManager.searchEngineForm,
    searchInputRef,
    searchSelectListShow: engineManager.searchSelectListShow,
    searchTerm,
    selectedIndex,
    startEditSearchEngine: engineManager.startEditSearchEngine,
    state: engineManager.state,
    suggestionsVisible,
  }
}
