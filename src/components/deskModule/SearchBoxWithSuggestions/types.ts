export interface SearchBoxWithSuggestionsProps {
  background?: string
  textColor?: string
  searchItems?: Panel.ItemInfo[]
}

export type SearchBoxWithSuggestionsEmit
  = | ((e: 'itemSearch', keyword: string) => void)
    | ((e: 'itemSelect', item: Panel.ItemInfo) => void)

export interface SearchBoxState {
  currentSearchEngine: DeskModule.SearchBox.SearchEngine
  searchEngineList: DeskModule.SearchBox.SearchEngine[]
  newWindowOpen: boolean
  searchBookmarks: boolean
}

export interface SuggestionItem {
  value: string
  type?: 'bookmark' | 'item' | 'search'
  isBookmark?: boolean
  url?: string
  item?: Panel.ItemInfo
  [key: string]: any
}
