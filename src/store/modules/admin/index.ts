import { defineStore } from 'pinia'
import type { AdminState, Theme } from './helper'
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

  },
})

export function useAdminStoreWithOut() {
  return useAdminStore(store)
}
