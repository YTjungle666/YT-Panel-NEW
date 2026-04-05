import { ss } from '@/utils/storage'

const LAN_PING_URL_STORAGE_KEY = 'yt-panel-lan-ping-url'

function isPrivateIpv4(hostname: string) {
  const match = hostname.match(/^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/)
  if (!match)
    return false

  const [, a, b] = match.map(Number)

  return a === 10
    || (a === 172 && b >= 16 && b <= 31)
    || (a === 192 && b === 168)
    || a === 127
    || (a === 169 && b === 254)
}

export function isPrivateHostname(hostname?: string | null) {
  if (!hostname)
    return false

  const normalized = hostname.trim().toLowerCase()

  if (!normalized)
    return false

  return normalized === 'localhost'
    || normalized.endsWith('.local')
    || normalized.endsWith('.lan')
    || normalized.endsWith('.home')
    || isPrivateIpv4(normalized)
    || normalized.startsWith('fd')
    || normalized.startsWith('fc')
  }

export function getLanPingUrl() {
  return (ss.get(LAN_PING_URL_STORAGE_KEY) || '').trim()
}

export function setLanPingUrl(url: string) {
  const normalized = url.trim()
  if (normalized)
    ss.set(LAN_PING_URL_STORAGE_KEY, normalized)
  else
    ss.remove(LAN_PING_URL_STORAGE_KEY)
}

export async function testLanPingUrl(url: string, timeoutMs = 3000) {
  const normalized = url.trim()
  if (!normalized)
    return false

  const controller = new AbortController()
  const timer = window.setTimeout(() => controller.abort(), timeoutMs)

  try {
    await fetch(normalized, {
      method: 'GET',
      mode: 'no-cors',
      cache: 'no-store',
      signal: controller.signal,
    })
    return true
  } catch {
    return false
  } finally {
    window.clearTimeout(timer)
  }
}
