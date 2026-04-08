import { defineStore } from 'pinia'
import type { AppState, Theme } from './helper'
import { defaultSetting, getLocalSetting, removeLocalState, setLocalSetting } from './helper'
import { store } from '@/store'
import { useTheme } from '@/hooks/useTheme'

export const useAppStore = defineStore('app-store', {
  state: (): AppState => getLocalSetting(),
  actions: {
    setSiderCollapsed(collapsed: boolean) {
      this.siderCollapsed = collapsed
      this.recordState()
    },

    setTheme(theme: Theme) {
      this.theme = theme
      this.recordState()
    },

    getTheme() {
      const { theme } = useTheme()
      return theme
    },

    recordState() {
      setLocalSetting(this.$state)
    },

    removeToken() {
      this.$state = defaultSetting()
      removeLocalState()
    },
  },
})

export function useAppStoreWithOut() {
  return useAppStore(store)
}
