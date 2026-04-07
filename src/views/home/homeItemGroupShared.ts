import { PanelStateNetworkModeEnum } from '@/enums'
import { VisitMode } from '@/enums/auth'

export interface ItemGroup extends Panel.ItemIconGroup {
  sortStatus?: boolean
  hoverStatus: boolean
  items?: Panel.ItemInfo[]
}

export const GROUP_LIST_CACHE_KEY = 'groupListCache'
export const ITEM_ICON_LIST_CACHE_KEY_PREFIX = 'itemIconList_'

export interface HomeItemGroupsMessageApi {
  success: (content: string) => void
  error: (content: string) => void
}

export interface HomeItemGroupsDialogApi {
  warning: (options: {
    title: string
    content: string
    positiveText: string
    negativeText: string
    onPositiveClick: () => void
  }) => void
}

export interface HomeItemGroupsAuthStore {
  userInfo?: {
    mustChangePassword?: boolean
  } | null
  visitMode: VisitMode
}

export interface HomeItemGroupsPanelState {
  networkMode: PanelStateNetworkModeEnum
  panelConfig: {
    searchBoxSearchIcon?: boolean
  }
}

export interface UseHomeItemGroupsOptions {
  authStore: HomeItemGroupsAuthStore
  panelState: HomeItemGroupsPanelState
  dialog: HomeItemGroupsDialogApi
  message: HomeItemGroupsMessageApi
  openPage: (openMethod: number, url: string, title?: string) => void
  onEditItem: (item: Panel.ItemInfo) => void
}

export function normalizeUrl(url: string | undefined | null) {
  if (!url)
    return ''

  const trimmed = url.trim()
  if (!trimmed)
    return ''

  if (/^[a-z]+:/i.test(trimmed) || trimmed.startsWith('/') || trimmed.startsWith('./') || trimmed.startsWith('../'))
    return trimmed

  return `http://${trimmed}`
}

export function isValidUrl(url: string | undefined | null) {
  if (!url)
    return false

  const trimmed = url.trim()
  return trimmed !== '' && trimmed !== 'null' && trimmed !== 'undefined'
}

export function resolveJumpUrl(item: Panel.ItemInfo, networkMode: PanelStateNetworkModeEnum) {
  const publicUrl = normalizeUrl(item.url)
  const lanUrl = normalizeUrl(item.lanUrl)
  return networkMode === PanelStateNetworkModeEnum.lan && isValidUrl(item.lanUrl)
    ? lanUrl
    : publicUrl
}

export function matchesItemSearchKeyword(item: Panel.ItemInfo, lowerCaseKeyword: string) {
  const textIconKeyword = item.icon?.itemType === 1 ? item.icon.text : ''

  return (
    item.title.toLowerCase().includes(lowerCaseKeyword)
    || item.url.toLowerCase().includes(lowerCaseKeyword)
    || item.description?.toLowerCase().includes(lowerCaseKeyword)
    || textIconKeyword?.toLowerCase().includes(lowerCaseKeyword)
  )
}
