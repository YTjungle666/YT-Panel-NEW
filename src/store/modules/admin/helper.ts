export type Theme = 'light' | 'dark' | 'auto'

export interface AdminState {
  siderCollapsed: boolean
  theme: Theme
}

export function defaultSetting(): AdminState {
  return { siderCollapsed: false, theme: 'light' }
}
