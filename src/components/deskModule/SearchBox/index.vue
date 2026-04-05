<script setup lang="ts">
import { defineEmits, onMounted, ref, watch } from 'vue'
import { NAvatar, NCheckbox } from 'naive-ui'
import { SvgIcon } from '@/components/common'
import { useModuleConfig } from '@/store/modules'
import { useAuthStore } from '@/store'
import { VisitMode } from '@/enums/auth'
import { useBookmarkSearch } from '@/composables/useSmartSearch'
import type { SearchResult } from '@/composables/useSmartSearch'
import SvgSrcBing from '@/assets/search_engine_svg/bing.svg'
import SvgSrcGoogle from '@/assets/search_engine_svg/google.svg'
import { openUrlWithoutReferer } from '@/utils/cmn'

withDefaults(defineProps<{
  background?: string
  textColor?: string
}>(), {
  background: '#2a2a2a6b',
  textColor: 'white',
})

const emits = defineEmits(['itemSearch', 'bookmarkSelect'])

interface State {
  currentSearchEngine: DeskModule.SearchBox.SearchEngine
  searchEngineList: DeskModule.SearchBox.SearchEngine[]
  newWindowOpen: boolean
}

const moduleConfigName = 'deskModuleSearchBox'
const moduleConfig = useModuleConfig()
const authStore = useAuthStore()
const searchTerm = ref('')
const isFocused = ref(false)
const searchSelectListShow = ref(false)
const showBookmarkResults = ref(false)

// 书签搜索
const bookmarks = ref<SearchResult[]>([])
const { query: bookmarkQuery, results: bookmarkResults, loading: searchLoading } = useBookmarkSearch(bookmarks, {
  debounceMs: 200,
  minQueryLength: 1,
  useFuseThreshold: 0.4
})

// 同步搜索词
watch(searchTerm, (val: string) => {
  bookmarkQuery.value = val
  showBookmarkResults.value = isFocused.value && val.length > 0
})

// 选择书签
function selectBookmark(item: SearchResult) {
  const url = item.url || item.lan_url
  if (url) {
    openUrlWithoutReferer(url, state.value.newWindowOpen ? '_blank' : '_self')
  }
  emits('bookmarkSelect', item)
  handleClearSearchTerm()
}

// 高亮文本
function highlightText(text: string, query: string): string {
  if (!query) return text
  const escaped = query.replace(/[.*+?^${}()|[\]\\\\]/g, '\\\\$&')
  return text.replace(new RegExp(`(${escaped})`, 'gi'), '<mark class="bg-yellow-400/30 text-white">$1</mark>')
}

const defaultSearchEngineList = ref<DeskModule.SearchBox.SearchEngine[]>([
  {
    id: 1,
    iconSrc: SvgSrcGoogle,
    title: 'Google',
    url: 'https://www.google.com/search?q=%s',
  },
  {
    id: 2,
    iconSrc: SvgSrcBing,
    title: 'Bing',
    url: 'https://www.bing.com/search?q=%s',
  },
])

const defaultState: State = {
  currentSearchEngine: defaultSearchEngineList.value[0],
  searchEngineList: defaultSearchEngineList.value,
  newWindowOpen: true,
}

const state = ref<State>({ ...defaultState })

const onFocus = (): void => {
  isFocused.value = true
  showBookmarkResults.value = searchTerm.value.length > 0
}

const onBlur = (): void => {
  setTimeout(() => {
    isFocused.value = false
    showBookmarkResults.value = false
  }, 200)
}

function handleEngineClick() {
  if (authStore.visitMode === VisitMode.VISIT_MODE_PUBLIC) return
  searchSelectListShow.value = !searchSelectListShow.value
}

function handleEngineUpdate(engine: DeskModule.SearchBox.SearchEngine) {
  state.value.currentSearchEngine = engine
  moduleConfig.saveToCloud(moduleConfigName, state.value)
  searchSelectListShow.value = false
}

function getSearchOpenTarget() {
  return state.value.newWindowOpen ? '_blank' : '_self'
}

function handleSearchClick() {
  // 直接联网搜索（不再优先打开书签）
  const url = state.value.currentSearchEngine.url
  const keyword = searchTerm
  const fullUrl = replaceOrAppendKeywordToUrl(url, keyword.value)
  handleClearSearchTerm()
  openUrlWithoutReferer(fullUrl, getSearchOpenTarget())
}

function replaceOrAppendKeywordToUrl(url: string, keyword: string) {
  if (url.includes('%s')) {
    return url.replace('%s', encodeURIComponent(keyword))
  }
  return url + (keyword ? `${encodeURIComponent(keyword)}` : '')
}

const handleItemSearch = () => {
  emits('itemSearch', searchTerm.value)
}

function handleClearSearchTerm() {
  searchTerm.value = ''
  bookmarkQuery.value = ''
  showBookmarkResults.value = false
  emits('itemSearch', searchTerm.value)
}

onMounted(() => {
  moduleConfig.getValueByNameFromCloud<State>('deskModuleSearchBox').then(({ code, data }) => {
    if (code === 0) state.value = { ...defaultState, ...(data || {}) }
    else state.value = { ...defaultState }
  })
})
</script>

<template>
  <div class="search-box w-full" @keydown.enter="handleSearchClick" @keydown.esc="handleClearSearchTerm">
    <div
      class="search-container flex rounded-2xl items-center justify-center text-white w-full"
      :style="{ background, color: textColor }"
      :class="{ focused: isFocused }"
    >
      <div class="search-box-btn-engine w-[40px] flex justify-center cursor-pointer" @click="handleEngineClick">
        <NAvatar :src="state.currentSearchEngine.iconSrc" style="background-color: transparent;" :size="20" />
      </div>
      <input v-model="searchTerm" :placeholder="$t('deskModule.searchBox.inputPlaceholder')" @focus="onFocus" @blur="onBlur" @input="handleItemSearch">
      <div v-if="searchTerm !== ''" class="search-box-btn-clear w-[25px] mr-[10px] flex justify-center cursor-pointer" @click="handleClearSearchTerm">
        <SvgIcon style="width: 20px;height: 20px;" icon="line-md:close-small" />
      </div>
      <div class="search-box-btn-search w-[25px] flex justify-center cursor-pointer" @click="handleSearchClick">
        <SvgIcon style="width: 20px;height: 20px;" icon="iconamoon:search-fill" />
      </div>
    </div>

    <!-- 书签搜索结果 -->
    <div
      v-if="showBookmarkResults && (bookmarkResults.length > 0 || searchTerm.length >= 2)"
      class="w-full mt-[10px] rounded-xl overflow-hidden z-50"
      :style="{ background }"
      style="backdrop-filter: blur(10px);"
    >
      <!-- 加载中 -->
      <div v-if="searchLoading" class="p-4 text-center text-gray-400 text-sm">
        <span class="animate-pulse">搜索中...</span>
      </div>
      
      <!-- 结果列表 -->
      <div v-else-if="bookmarkResults.length > 0" class="py-2">
        <div class="px-3 py-1 text-xs text-gray-400 border-b border-gray-700/50 flex justify-between">
          <span>📑 书签</span>
          <span>{{ bookmarkResults.length }} 个结果</span>
        </div>
        <div
          v-for="item in bookmarkResults.slice(0, 6)"
          :key="item.id"
          class="px-3 py-2 flex items-center gap-3 cursor-pointer hover:bg-white/10"
          @mousedown.prevent
          @click="selectBookmark(item)"
        >
          <div class="w-6 h-6 rounded bg-white/10 flex items-center justify-center">
            <img v-if="item.icon" :src="item.icon" class="w-4 h-4 object-contain">
            <span v-else>🔗</span>
          </div>
          <div class="flex-1 min-w-0">
            <div class="text-sm truncate" v-html="highlightText(item.title, searchTerm)"></div>
            <div class="text-xs text-gray-400 truncate">{{ item.url }}</div>
          </div>
        </div>
      </div>
      
      <!-- 无结果 -->
      <div v-else-if="searchTerm.length >= 2" class="p-4 text-center text-gray-400 text-sm">
        未找到匹配的书签
      </div>
    </div>

    <!-- 搜索引擎选择 -->
    <div v-if="searchSelectListShow" class="w-full mt-[10px] rounded-xl p-[10px]" :style="{ background }">
      <div class="flex items-center">
        <div class="flex items-center">
          <div v-for="(item, index) in defaultSearchEngineList" :key="index" :title="item.title" class="w-[40px] h-[40px] mr-[10px] cursor-pointer bg-[#ffffff] flex items-center justify-center rounded-xl" @click="handleEngineUpdate(item)">
            <NAvatar :src="item.iconSrc" style="background-color: transparent;" :size="20" />
          </div>
        </div>
      </div>
      <div class="mt-[10px]">
        <NCheckbox v-model:checked="state.newWindowOpen" @update-checked="moduleConfig.saveToCloud(moduleConfigName, state)">
          <span :style="{ color: textColor }">{{ $t('deskModule.searchBox.openWithNewOpen') }}</span>
        </NCheckbox>
      </div>
    </div>
  </div>
</template>

<style scoped>
.search-container { border: 1px solid #ccc; transition: box-shadow 0.5s,backdrop-filter 0.5s; padding: 2px 10px; backdrop-filter:blur(2px) }
.focused, .search-container:hover { box-shadow: 0px 0px 30px -5px rgba(41, 41, 41, 0.45); -webkit-box-shadow: 0px 0px 30px -5px rgba(0, 0, 0, 0.45); -moz-box-shadow: 0px 0px 30px -5px rgba(0, 0, 0, 0.45); backdrop-filter:blur(5px) }
.before { left: 10px; }
.after { right: 10px; }
input { background-color: transparent; box-sizing: border-box; width: 100%; height: 40px; padding: 10px 5px; border: none; outline: none; font-size: 17px; }
mark { padding: 0 2px; border-radius: 2px; }
</style>
