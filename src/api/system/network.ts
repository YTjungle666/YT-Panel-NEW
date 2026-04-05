import { get } from '@/utils/request'

export function checkIsLan() {
  return get<{ isLan: boolean }>({
    url: '/isLan',
  })
}
