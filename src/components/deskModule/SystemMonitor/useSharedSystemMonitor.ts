import { computed, onUnmounted, ref } from 'vue'
import { getAll } from '@/api/system/systemMonitor'

const monitorData = ref<any>(null)
const loading = ref(false)
let timer: ReturnType<typeof setInterval> | null = null
let subscribers = 0
let currentInterval = 2000

async function refresh() {
  loading.value = true
  try {
    const { data, code } = await getAll<any>()
    if (code === 0)
      monitorData.value = data
  }
  finally {
    loading.value = false
  }
}

function startPolling(interval: number) {
  const normalized = (!interval || interval <= 2000) ? 2000 : interval
  if (timer && currentInterval === normalized)
    return

  if (timer)
    clearInterval(timer)

  currentInterval = normalized
  void refresh()
  timer = setInterval(() => {
    void refresh()
  }, normalized)
}

function stopPolling() {
  if (timer) {
    clearInterval(timer)
    timer = null
  }
}

export function useSharedSystemMonitor(refreshInterval: number) {
  subscribers += 1
  startPolling(refreshInterval)

  onUnmounted(() => {
    subscribers -= 1
    if (subscribers <= 0) {
      subscribers = 0
      stopPolling()
    }
  })

  return {
    loading: computed(() => loading.value),
    monitorData: computed(() => monitorData.value),
    refresh,
  }
}
