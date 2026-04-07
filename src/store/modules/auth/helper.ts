import type { AuthState } from './index'
import { removeScopedStorage } from '@/utils/storage'

const LOCAL_NAME = 'AUTH_TOKEN'
const APP_STORAGE_KEYS = [
  LOCAL_NAME,
  'USER_AUTH_INFO_CACHE',
  'USER_CONFIG_CACHE',
  'groupListCache',
  'bookmarksTreeCache',
  'searchEngineListCache',
]
const APP_STORAGE_PREFIXES = [
  'moduleConfig_',
  'ITEM_ICON_LIST_CACHE_',
  'itemIconList_',
]

export function setStorage(_state: AuthState) {
  return null
}

export function getStorage() {
  return null
}

export function removeToken() {
  return removeScopedStorage(APP_STORAGE_KEYS, APP_STORAGE_PREFIXES)
}

export function clearAppScopedStorage() {
  return removeScopedStorage(APP_STORAGE_KEYS, APP_STORAGE_PREFIXES)
}
