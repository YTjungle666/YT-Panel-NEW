import type { AxiosProgressEvent, AxiosResponse, GenericAbortSignal } from 'axios'
import request from './axios'
import { apiRespErrMsg, message, resolveApiErrorMessage } from './apiMessage'
import { t } from '@/locales'
import { useAuthStore } from '@/store'
import { router } from '@/router'

let loginMessageShow = false
let passwordChangeRequiredNoticeUntil = 0
let suppressLoginExpiredNoticeUntil = 0
export interface HttpOption {
  url: string
  data?: any
  method?: string
  headers?: any
  onDownloadProgress?: (progressEvent: AxiosProgressEvent) => void
  signal?: GenericAbortSignal
  beforeRequest?: () => void
  afterRequest?: () => void
}

export interface Response<T = any> {
  data: T
  msg: string
  code: number
}

export function suppressLoginExpiredNotice(durationMs = 3000) {
  suppressLoginExpiredNoticeUntil = Date.now() + durationMs
}

function http<T = any>(
  { url, data, method, headers, onDownloadProgress, signal, beforeRequest, afterRequest }: HttpOption,
) {
  const authStore = useAuthStore()
  const successHandler = (res: AxiosResponse<Response<T>>) => {
    if (res.data.code === 0)
      return res.data

    if (res.data.code === 1001) {
      if (Date.now() < suppressLoginExpiredNoticeUntil) {
        router.push({ path: '/login' })
        authStore.removeToken()
        return res.data
      }

      if (loginMessageShow === false) {
        loginMessageShow = true
        message.warning(resolveApiErrorMessage(res.data) || t('api.loginExpires'), {
          onLeave() {
            loginMessageShow = false
          },
        })
      }

      router.push({ path: '/login' })
      authStore.removeToken()
      return res.data
    }

    if (res.data.code === 1000) {
      router.push({ path: '/login' })
      authStore.removeToken()
      return res.data
    }

    if (res.data.code === 1005) {
      message.warning(resolveApiErrorMessage(res.data))
      return res.data
    }

    if (res.data.code === 1108) {
      if (Date.now() >= passwordChangeRequiredNoticeUntil) {
        passwordChangeRequiredNoticeUntil = Date.now() + 1500
        message.warning(resolveApiErrorMessage(res.data))
      }
      return res.data
    }

    if (res.data.code === -1) {
      return res.data
    }

    if (!apiRespErrMsg(res.data))
      return Promise.reject(res.data)
    else
      return res.data
  }

  const failHandler = (error: any) => {
    afterRequest?.()
    const structured = error?.response?.data
    if (structured && typeof structured.code === 'number')
      return successHandler({ data: structured } as AxiosResponse<Response<T>>)

    message.error(t('common.networkError'), {
      duration: 50000,
      closable: true,
    })
    throw new Error(error?.response?.data?.msg || error?.msg || 'Error')
  }

  beforeRequest?.()

  method = method || 'GET'

  const params = Object.assign(typeof data === 'function' ? data() : data ?? {}, {})
  if (!headers)
    headers = {}

  return method === 'GET'
    ? request.get(url, { params, headers, signal, onDownloadProgress }).then(successHandler, failHandler)
    : request.post(url, params, { headers, signal, onDownloadProgress }).then(successHandler, failHandler)
}

export function get<T = any>(
  { url, data, method = 'GET', onDownloadProgress, signal, beforeRequest, afterRequest }: HttpOption,
): Promise<Response<T>> {
  return http<T>({
    url,
    method,
    data,
    onDownloadProgress,
    signal,
    beforeRequest,
    afterRequest,
  })
}

export function post<T = any>(
  { url, data, method = 'POST', headers, onDownloadProgress, signal, beforeRequest, afterRequest }: HttpOption,
): Promise<Response<T>> {
  return http<T>({
    url,
    method,
    data,
    headers,
    onDownloadProgress,
    signal,
    beforeRequest,
    afterRequest,
  })
}

export default post
