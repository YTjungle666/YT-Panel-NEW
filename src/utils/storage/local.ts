interface StorageData<T = any> {
  data: T
  expire: number | null
}

export function createLocalStorage(options?: { expire?: number | null; crypto?: boolean }) {
  const DEFAULT_CACHE_TIME = 60 * 60 * 24 * 7

  const { expire } = Object.assign(
    {
      expire: DEFAULT_CACHE_TIME,
      crypto: true,
    },
    options,
  )

  function set<T = any>(key: string, data: T) {
    const storageData: StorageData<T> = {
      data,
      expire: expire !== null ? new Date().getTime() + expire * 1000 : null,
    }

    const json = JSON.stringify(storageData)
    window.localStorage.setItem(key, json)
  }

  function get(key: string) {
    const json = window.localStorage.getItem(key)
    if (json) {
      let storageData: StorageData | null = null

      try {
        storageData = JSON.parse(json)
      }
      catch {
        // Prevent failure
      }

      if (storageData) {
        const { data, expire } = storageData
        if (expire === null || expire >= Date.now())
          return data
      }

      remove(key)
      return null
    }
  }

  function remove(key: string) {
    window.localStorage.removeItem(key)
  }

  return {
    set,
    get,
    remove,
  }
}

export const ls = createLocalStorage()

export const ss = createLocalStorage({ expire: null, crypto: false })

function removeKeysFromStorage(storage: Storage, keys: string[]) {
  for (const key of keys)
    storage.removeItem(key)
}

function removeByPrefixesFromStorage(storage: Storage, prefixes: string[]) {
  const keys = [...Array.from({ length: storage.length }, (_, index) => storage.key(index)).filter(Boolean)] as string[]
  for (const key of keys) {
    if (prefixes.some(prefix => key.startsWith(prefix)))
      storage.removeItem(key)
  }
}

export function removeScopedStorage(keys: string[], prefixes: string[] = []) {
  removeKeysFromStorage(window.localStorage, keys)
  removeKeysFromStorage(window.sessionStorage, keys)

  if (prefixes.length > 0) {
    removeByPrefixesFromStorage(window.localStorage, prefixes)
    removeByPrefixesFromStorage(window.sessionStorage, prefixes)
  }
}
