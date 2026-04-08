import { createDiscreteApi, darkTheme, lightTheme, useOsTheme } from 'naive-ui'
import { computed, ref } from 'vue'
import type { Response } from './index'
import type { ConfigProviderProps } from 'naive-ui'
import { t } from '@/locales'
import { useAppStore } from '@/store'

const themeRef = ref<'light' | 'dark'>('light')
const configProviderPropsRef = computed<ConfigProviderProps>(() => ({
  theme: themeRef.value === 'light' ? lightTheme : darkTheme,
}))
export const { message, dialog } = createDiscreteApi(['message', 'dialog'], { configProviderProps: configProviderPropsRef })

export function resolveApiErrorMessage(res: Pick<Response, 'code' | 'msg'>): string {
  const apiErrorCodeName = `apiErrorCode.${res.code}`
  const translated = t(apiErrorCodeName)
  if (translated !== apiErrorCodeName)
    return translated

  return res.msg || t('common.failed')
}

export function apiRespErrMsg(res: Response): boolean {
  const appStore = useAppStore()
  const osTheme = useOsTheme()
  if (appStore.theme === 'auto')
    themeRef.value = osTheme.value as 'dark' | 'light'
  else
    themeRef.value = appStore.theme as 'dark' | 'light'

  const apiErrorCodeName = `apiErrorCode.${res.code}`
  const localized = t(apiErrorCodeName)
  if (localized === apiErrorCodeName) {
    return false
  }
  else {
    message.error(resolveApiErrorMessage(res))
    return true
  }
}
