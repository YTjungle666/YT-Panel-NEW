import { computed } from 'vue'
import { zhCN } from 'naive-ui'
import { setLocale } from '@/locales'

export function useLanguage() {
  const language = computed(() => {
    setLocale('zh-CN')
    return zhCN
  })

  return { language }
}
