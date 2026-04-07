import DOMPurify from 'dompurify'

export function sanitizeHtml(html: string, options?: DOMPurify.Config) {
  return DOMPurify.sanitize(html || '', options)
}

function escapeHtml(text: string) {
  return (text || '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')
}

function escapeRegExp(text: string) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

export function sanitizeFooterHtml(html: string) {
  return sanitizeHtml(html, {
    ALLOWED_TAGS: ['div', 'span', 'a', 'br', 'b', 'strong', 'i', 'em'],
    ALLOWED_ATTR: ['class', 'href', 'target', 'rel'],
    ALLOW_DATA_ATTR: false,
  })
}

export function sanitizeNotepadHtml(html: string) {
  return sanitizeHtml(html, {
    ALLOWED_TAGS: ['div', 'p', 'br', 'b', 'strong', 'i', 'em', 'ul', 'ol', 'li', 'pre', 'blockquote', 'h1', 'h2', 'hr', 'img', 'a'],
    ALLOWED_ATTR: ['href', 'src', 'alt', 'title', 'class', 'data-filename'],
    ALLOW_DATA_ATTR: true,
    ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto):|\/|\.{1,2}\/|data:image\/(?:png|jpeg|jpg|gif|webp|svg\+xml);base64,)/i,
  })
}

export function sanitizeRenderedMarkdown(html: string) {
  return sanitizeHtml(html, {
    ALLOWED_TAGS: [
      'p','br','strong','b','em','i','code','pre','blockquote','ul','ol','li','a','span',
      'h1','h2','h3','h4','h5','h6','hr','table','thead','tbody','tr','th','td','del','img'
    ],
    ALLOWED_ATTR: ['href','target','rel','class','src','alt','title'],
    ALLOW_DATA_ATTR: false,
  })
}

export function buildHighlightedHtml(text: string, query: string, markClass = '') {
  const safeText = escapeHtml(text || '')
  const trimmedQuery = query.trim()

  if (!trimmedQuery)
    return safeText

  const highlighted = safeText.replace(
    new RegExp(`(${escapeRegExp(trimmedQuery)})`, 'gi'),
    `<mark class="${escapeHtml(markClass)}">$1</mark>`,
  )

  return sanitizeHtml(highlighted, {
    ALLOWED_TAGS: ['mark'],
    ALLOWED_ATTR: ['class'],
    ALLOW_DATA_ATTR: false,
  })
}
