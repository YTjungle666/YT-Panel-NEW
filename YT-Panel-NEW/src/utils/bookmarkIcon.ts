const defaultBookmarkSvg = `
<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64" fill="none">
  <defs>
    <linearGradient id="bookmarkBg" x1="8" y1="8" x2="56" y2="56" gradientUnits="userSpaceOnUse">
      <stop stop-color="#4F8CFF"/>
      <stop offset="1" stop-color="#2563EB"/>
    </linearGradient>
  </defs>
  <rect x="8" y="8" width="48" height="48" rx="14" fill="url(#bookmarkBg)"/>
  <path d="M24 18C24 16.8954 24.8954 16 26 16H38C39.1046 16 40 16.8954 40 18V46L32 39.5L24 46V18Z" fill="white" fill-opacity="0.96"/>
  <path d="M28 24H36" stroke="#2563EB" stroke-width="2.6" stroke-linecap="round" opacity="0.18"/>
</svg>
`.trim()

const defaultFolderSvg = `
<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64" fill="none">
  <defs>
    <linearGradient id="folderBody" x1="10" y1="18" x2="54" y2="50" gradientUnits="userSpaceOnUse">
      <stop stop-color="#FFD978"/>
      <stop offset="1" stop-color="#F4B740"/>
    </linearGradient>
  </defs>
  <path d="M12 20C12 16.6863 14.6863 14 18 14H26L31 20H46C49.3137 20 52 22.6863 52 26V44C52 47.3137 49.3137 50 46 50H18C14.6863 50 12 47.3137 12 44V20Z" fill="url(#folderBody)"/>
  <path d="M12 24C12 21.7909 13.7909 20 16 20H48C50.2091 20 52 21.7909 52 24V28H12V24Z" fill="#FFE39D" fill-opacity="0.9"/>
</svg>
`.trim()

function svgToDataUrl(svg: string) {
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`
}

export const DEFAULT_BOOKMARK_ICON = svgToDataUrl(defaultBookmarkSvg)
export const DEFAULT_FOLDER_ICON = svgToDataUrl(defaultFolderSvg)

function isClearlyInvalidIconValue(value: string) {
  const normalized = value.trim().toLowerCase()
  return [
    '',
    'null',
    'undefined',
    '{}',
    '[]',
    '[object object]',
  ].includes(normalized)
}

export function normalizeIconDataUrl(icon?: string | null) {
  if (!icon)
    return ''

  const trimmed = icon.trim()
  if (!trimmed || isClearlyInvalidIconValue(trimmed))
    return ''

  if (trimmed.startsWith('data:'))
    return trimmed

  if (trimmed.startsWith('http://') || trimmed.startsWith('https://'))
    return trimmed

  if (!/^[a-z0-9+/=\r\n]+$/i.test(trimmed))
    return ''

  return `data:image/png;base64,${trimmed}`
}

export function getBookmarkIconSrc(icon?: string | null) {
  return normalizeIconDataUrl(icon) || DEFAULT_BOOKMARK_ICON
}

export function getFolderIconSrc() {
  return DEFAULT_FOLDER_ICON
}
