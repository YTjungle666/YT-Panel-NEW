import { ref, computed, watch, type Ref } from 'vue'
import { debounce } from 'lodash-es'
import Fuse from 'fuse.js'
import { searchBookmarks, type SearchResult } from '@/api/search'

// Re-export SearchResult for convenience
export type { SearchResult } from '@/api/search'

interface SearchOptions {
  debounceMs?: number
  minQueryLength?: number
  useFuseThreshold?: number
}

// 书签搜索专用 composable
export function useBookmarkSearch(
  items: Ref<SearchResult[]>,
  options: SearchOptions = {}
) {
  const { debounceMs = 300, minQueryLength = 1, useFuseThreshold = 0.4 } = options

  const query = ref('')
  const loading = ref(false)
  const backendResults = ref<SearchResult[]>([])

  // Fuse.js 本地模糊搜索索引
  const fuseIndex = computed(() => {
    return new Fuse(items.value, {
      keys: ['title', 'url'],
      threshold: useFuseThreshold,
      includeScore: true,
    })
  })

  // 本地 Fuse 搜索结果（快速预览）
  const fuseResults = computed(() => {
    if (!query.value || query.value.length < minQueryLength) return []
    return fuseIndex.value.search(query.value).map((r: any) => ({ ...r.item }))
  })

  // 防抖触发后端搜索
  const debouncedSearch = debounce(async (q: string) => {
    if (q.length < 2) {
      backendResults.value = []
      return
    }
    loading.value = true
    try {
      const res = await searchBookmarks({ query: q, limit: 20, search_url: true })
      backendResults.value = res.data || []
    } catch (e) {
      console.error('Search failed:', e)
      backendResults.value = []
    } finally {
      loading.value = false
    }
  }, debounceMs)

  // 监听输入
  watch(query, (newQuery) => {
    debouncedSearch(newQuery)
  })

  // 最终结果：优先后端，Fuse兜底
  const results = computed(() => {
    if (backendResults.value.length > 0) {
      return backendResults.value
    }
    return fuseResults.value.slice(0, 20)
  })

  return {
    query,
    results,
    loading,
    fuseResults,
    backendResults,
  }
}
