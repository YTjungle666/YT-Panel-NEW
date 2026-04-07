import { ss } from '@/utils/storage/local'
import {
  buildBookmarkTree,
  convertServerTreeToBookmarkTree,
  type BookmarkTreeNode,
} from '@/utils/bookmarkTree'
import type { SuggestionItem } from './types'

const BOOKMARKS_CACHE_KEY = 'bookmarksTreeCache'

function includesKeyword(value: string | undefined | null, keyword: string) {
  return (value || '').toLowerCase().includes(keyword)
}

export function searchBookmarkSuggestions(keyword: string): SuggestionItem[] {
  const results: SuggestionItem[] = []
  const lowerCaseKeyword = keyword.toLowerCase()
  const cachedData = ss.get(BOOKMARKS_CACHE_KEY)
  if (!cachedData)
    return results

  let bookmarksTree: BookmarkTreeNode[] = []

  if (Array.isArray(cachedData)) {
    if (cachedData.length > 0 && 'children' in cachedData[0])
      bookmarksTree = convertServerTreeToBookmarkTree(cachedData)
    else if (cachedData[0]?.hasOwnProperty('id') || cachedData[0]?.hasOwnProperty('key'))
      bookmarksTree = buildBookmarkTree(cachedData)
  } else if (cachedData && typeof cachedData === 'object') {
    if (Array.isArray(cachedData.list)) {
      bookmarksTree = cachedData.list.length > 0 && 'children' in cachedData.list[0]
        ? convertServerTreeToBookmarkTree(cachedData.list)
        : buildBookmarkTree(cachedData.list)
    } else if (Array.isArray(cachedData.data)) {
      bookmarksTree = cachedData.data.length > 0 && 'children' in cachedData.data[0]
        ? convertServerTreeToBookmarkTree(cachedData.data)
        : buildBookmarkTree(cachedData.data)
    }
  }

  function traverse(node: any) {
    if (node.isLeaf && node.bookmark) {
      const title = node.bookmark.title.toLowerCase()
      const url = node.bookmark.url.toLowerCase()
      if (title.includes(lowerCaseKeyword) || url.includes(lowerCaseKeyword)) {
        results.push({
          value: node.bookmark.title,
          type: 'bookmark',
          isBookmark: true,
          url: node.bookmark.url,
        })
      }
    }

    if (node.children && node.children.length > 0)
      node.children.forEach((child: any) => traverse(child))
  }

  bookmarksTree.forEach((node: any) => traverse(node))
  return results
}

export function searchTextIconItems(searchItems: Panel.ItemInfo[], keyword: string): SuggestionItem[] {
  const lowerCaseKeyword = keyword.toLowerCase()

  return searchItems
    .filter(item => item.icon?.itemType === 1)
    .filter(item => (
      includesKeyword(item.title, lowerCaseKeyword)
      || includesKeyword(item.url, lowerCaseKeyword)
      || includesKeyword(item.description, lowerCaseKeyword)
      || includesKeyword(item.icon?.text, lowerCaseKeyword)
    ))
    .map(item => ({
      value: item.title || item.icon?.text || item.url,
      type: 'item',
      url: item.url,
      item,
    }))
}

export function dedupeSuggestions(suggestions: SuggestionItem[]) {
  const seen = new Set<string>()
  return suggestions.filter((suggestion) => {
    const key = `${suggestion.type || 'search'}::${suggestion.url || suggestion.value}`
    if (seen.has(key))
      return false

    seen.add(key)
    return true
  })
}

export function getDefaultSuggestions(keyword: string): SuggestionItem[] {
  const defaults = [
    '天气预报',
    '最新新闻',
    '股票行情',
    '电影推荐',
    '菜谱大全',
    '旅游攻略',
    '学习资料',
    '技术文档',
  ]

  return defaults
    .filter(item => item.includes(keyword))
    .map(item => ({ value: item, type: 'search' }))
}
