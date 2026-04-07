import { defineStore } from 'pinia'
import { getStorage, removeToken as hRemoveToken, setStorage } from './helper'
import { VisitMode } from '@/enums/auth'

export interface AuthState {
  userInfo: User.Info | null
  visitMode: VisitMode
}

const defaultState: AuthState = {
  userInfo: null,
  visitMode: VisitMode.VISIT_MODE_LOGIN,
}

export const useAuthStore = defineStore('auth-store', {
  state: (): AuthState => getStorage() ?? { ...defaultState },

  actions: {
    setUserInfo(userInfo: User.Info) {
      this.userInfo = userInfo
      this.saveStorage()
    },

    setVisitMode(visitMode: VisitMode) {
      this.visitMode = visitMode
      this.saveStorage()
    },

    saveStorage() {
      setStorage(this.$state)
    },

    removeToken() {
      this.$state = defaultState
      hRemoveToken()
    },
  },

})
