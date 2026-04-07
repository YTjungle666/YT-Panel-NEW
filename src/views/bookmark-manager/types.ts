export interface Bookmark {
  id: number
  title: string
  url: string
  folderId?: string | number
  isFolder?: boolean
  parentId?: number
  createTime?: string
  updateTime?: string
  iconJson?: string
  sort?: number
  icon?: any | null
  lanUrl?: string
  openMethod?: number
}

export interface TreeOption {
  key: string
  label: string
  isLeaf: boolean
  isFolder: boolean
  bookmark?: Bookmark
  children: TreeOption[]
  rawNode: any
  disabledExpand: boolean
  sort?: number
  ParentId?: string
}
