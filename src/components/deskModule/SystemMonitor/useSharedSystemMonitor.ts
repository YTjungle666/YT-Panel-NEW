import { computed, onUnmounted, ref } from 'vue'
import { getAll } from '@/api/system/systemMonitor'

const monitorData = ref<any>(null)
const loading = ref(false)
let timer: ReturnType<typeof setTimeout> | null = null
let subscribers = 0
let currentInterval = 2000
let inFlight = false

async function refresh() {
  if (inFlight)
    return

  inFlight = true
  loading.value = true
  try {
    const { data, code } = await getAll<any>()
    if (code === 0)
      monitorData.value = data
  }
  finally {
    inFlight = false
    loading.value = false
  }
}

function scheduleNext() {
  if (subscribers <= 0)
    return

  timer = setTimeout(() => {
    void tick()
  }, currentInterval)
}

async function tick() {
  await refresh()
  scheduleNext()
}

function startPolling(interval: number) {
  const normalized = (!interval || interval <= 2000) ? 2000 : interval
  if (timer && currentInterval === normalized)
    return

  if (timer)
    clearTimeout(timer)

  currentInterval = normalized
  void tick()
}

function stopPolling() {
  if (timer) {
    clearTimeout(timer)
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
