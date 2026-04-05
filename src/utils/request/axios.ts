import axios, { type AxiosResponse } from 'axios'

function resolveApiBaseUrl() {
  const raw = import.meta.env.VITE_GLOB_API_URL?.trim()
  if (!raw)
    return '/api'

  return raw.endsWith('/')
    ? raw.slice(0, -1)
    : raw
}

const service = axios.create({
  baseURL: resolveApiBaseUrl(),
  withCredentials: true,
})

service.interceptors.request.use(
  (config) => {
    return config
  },
  (error) => {
    return Promise.reject(error.response)
  },
)

service.interceptors.response.use(
  (response: AxiosResponse): AxiosResponse => {
    if (response.status === 200)
      return response

    throw new Error(response.status.toString())
  },
  (error) => {
    return Promise.reject(error)
  },
)

export default service
