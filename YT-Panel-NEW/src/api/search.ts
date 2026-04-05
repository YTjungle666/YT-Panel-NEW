import { get } from '@/utils/request'

export interface SearchRequest {
  query: string
  limit?: number
  search_url?: boolean
}

export interface SearchResult {
  id: number
  title: string
  url: string
  lan_url?: string
  icon?: string
  sort: number
  is_folder: number
  parent_id: number
  score?: number
}

export function searchBookmarks(params: SearchRequest) {
  return get<SearchResult[]>({ url: '/api/search/bookmarks', data: params })
}

export function searchSuggestions(params: { query: string; limit?: number }) {
  return get<string[]>({ url: '/api/search/suggestions', data: params })
}
