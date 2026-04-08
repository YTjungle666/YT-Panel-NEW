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

const rawApiErrorMatchers: Array<{
  pattern: RegExp
  resolve: (...groups: string[]) => string
}> = [
  {
    pattern: /^Name length must be between 2 and 15 characters$/i,
    resolve: () => t('apiErrorMessage.nameLength'),
  },
  {
    pattern: /^The account must be no less than 5 characters long$/i,
    resolve: () => t('apiErrorMessage.usernameMin5'),
  },
  {
    pattern: /^The account must be no less than 3 characters long$/i,
    resolve: () => t('apiErrorMessage.usernameMin3'),
  },
  {
    pattern: /^Username length must be between 3 and 80 characters$/i,
    resolve: () => t('apiErrorMessage.usernameLength'),
  },
  {
    pattern: /^Username can only contain letters, numbers, _, \. and @$/i,
    resolve: () => t('apiErrorMessage.usernameCharset'),
  },
  {
    pattern: /^Password is required$/i,
    resolve: () => t('apiErrorMessage.passwordRequired'),
  },
  {
    pattern: /^Password length must be between 8 and 64 characters$/i,
    resolve: () => t('apiErrorMessage.passwordLengthStrong'),
  },
  {
    pattern: /^Password cannot contain whitespace$/i,
    resolve: () => t('apiErrorMessage.passwordWhitespace'),
  },
  {
    pattern: /^Password is too weak\. Use at least three character types: uppercase, lowercase, numbers, and symbols$/i,
    resolve: () => t('apiErrorMessage.passwordTooWeak'),
  },
  {
    pattern: /^Invalid email address$/i,
    resolve: () => t('apiErrorMessage.emailInvalid'),
  },
  {
    pattern: /^Email must end with (.+)$/i,
    resolve: suffix => t('apiErrorMessage.emailSuffixRequired', { suffix }),
  },
]

function resolveRawApiErrorMessage(msg: string): string | null {
  const normalized = msg.trim()
  if (!normalized)
    return null

  for (const matcher of rawApiErrorMatchers) {
    const matched = normalized.match(matcher.pattern)
    if (matched)
      return matcher.resolve(...matched.slice(1))
  }

  return null
}

export function resolveApiErrorMessage(res: Pick<Response, 'code' | 'msg'>): string {
  const rawMessage = res.msg?.trim()
  const rawLocalized = res.msg ? resolveRawApiErrorMessage(res.msg) : null
  if (rawLocalized)
    return rawLocalized

  if ((res.code === 1400 || res.code === -1) && rawMessage)
    return rawMessage

  const apiErrorCodeName = `apiErrorCode.${res.code}`
  const translated = t(apiErrorCodeName)
  if (translated !== apiErrorCodeName)
    return translated

  return rawMessage || t('common.failed')
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
  const rawLocalized = res.msg ? resolveRawApiErrorMessage(res.msg) : null
  if (localized === apiErrorCodeName && !rawLocalized) {
    return false
  }
  else {
    message.error(resolveApiErrorMessage(res))
    return true
  }
}
