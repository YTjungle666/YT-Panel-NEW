import { defineStore } from 'pinia'
import type { AdminState, Language, Theme } from './helper'
import { defaultSetting } from './helper'
import { store } from '@/store'

export const useAdminStore = defineStore('admin-store', {
  state: (): AdminState => defaultSetting(),
  actions: {
    setSiderCollapsed(collapsed: boolean) {
      this.siderCollapsed = collapsed
    },

    setTheme(theme: Theme) {
      this.theme = theme
    },

    setLanguage(language: Language) {
      if (this.language !== language)
        this.language = language
    },
  },
})

export function useAdminStoreWithOut() {
  return useAdminStore(store)
}
