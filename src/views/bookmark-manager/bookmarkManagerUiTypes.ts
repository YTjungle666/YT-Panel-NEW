import { useBookmarkManagerData } from './useBookmarkManagerData'

export type BookmarkManagerData = ReturnType<typeof useBookmarkManagerData>

export interface BookmarkManagerDragItem {
  id: string | number
  title: string
  url: string
  isFolder: boolean
  folderId: string | number
  label?: string
  iconJson?: string
  sort?: number
  lanUrl?: string
  openMethod?: number
  icon?: any
}
