import CryptoJS from 'crypto-js'
import dayjs from 'dayjs'

const VERSION = 1 // 当前书签文件版本
const ALLOW_LOW_VERSION = 1 // 最小支持的书签文件版本号
const APPNAME = 'YT-Panel-Bookmark'

// 书签接口定义
export interface BookmarkItem {
  id: number
  title: string
  url: string
  folderId?: number
  isFolder: boolean
  icon?: string
}

export interface BookmarkFolder {
  id: number
  title: string
  children: (BookmarkItem | BookmarkFolder)[]
  isFolder: boolean
}

export interface BookmarkJsonStructure {
  version: number
  appName: 'YT-Panel-Bookmark'
  exportTime: string
  appVersion: string
  bookmarks: BookmarkFolder[]
  md5: string
}

export type BookmarkTreeNode = BookmarkItem | BookmarkFolder

export class FormatError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'FormatError'
  }
}

export class ConfigVersionLowError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'ConfigVersionLowError'
  }
}

interface ExportBookmarkResult {
  addBookmarksData(datas: BookmarkFolder[]): ExportBookmarkResult
  exportFile(): void
  string(): string
}

// 导出书签数据
export function exportBookmarkJson(appVersion?: string): ExportBookmarkResult {
  const jsonData: BookmarkJsonStructure = {
    version: VERSION,
    appName: APPNAME,
    exportTime: dayjs().format('YYYY-MM-DD HH:mm:ss'),
    appVersion: appVersion || '',
    bookmarks: [],
    md5: '',
  }

  // MD5 生成函数
  function generateMD5AndUpdate() {
    jsonData.md5 = generateMD5(JSON.stringify(jsonData))
  }

  return {
    // 添加书签信息
    addBookmarksData(datas: BookmarkFolder[]) {
      jsonData.bookmarks = datas
      return this
    },

    // 导出json文件
    exportFile() {
      generateMD5AndUpdate()
      const jsonString = JSON.stringify(jsonData)
      if (jsonString) {
        const blob = new Blob([jsonString], { type: 'application/json' })
        const link = document.createElement('a')
        link.href = URL.createObjectURL(blob)
        link.download = `SunPanel-Bookmark${dayjs().format('YYYYMMDDHHmm')}.yt-panel.json`
        link.click()
      }
    },

    // 返回字符串
    string() {
      generateMD5AndUpdate()
      return JSON.stringify(jsonData)
    },
  }
}

export interface ImportBookmarkResult {
  isPassCheckMd5: () => boolean
  isPassCheckConfigVersionOld: () => boolean
  isPassCheckConfigVersionNew: () => boolean
  isPassCheckConfigVersionBest: () => boolean
  jsonStruct: BookmarkJsonStructure
  hasProperty: (key: string) => boolean
  getBookmarks: () => BookmarkFolder[]
}

// 导入书签json数据
export function importBookmarkJsonString(jsonString: string): ImportBookmarkResult | null {
  let data: any
  try {
    data = JSON.parse(jsonString)
  } catch (error) {
    throw new FormatError('file format error')
  }

  const jsonStruct = transformJson(data)
  const md5 = generateMD5(jsonString)

  if (!jsonStruct) {
    throw new FormatError('file format error')
  }

  if (data.version < ALLOW_LOW_VERSION) {
    throw new ConfigVersionLowError('')
  }

  return {
    isPassCheckMd5: () => md5 === jsonStruct.md5,
    isPassCheckConfigVersionOld: () => !(jsonStruct.version < VERSION),
    isPassCheckConfigVersionNew: () => !(jsonStruct.version > VERSION),
    isPassCheckConfigVersionBest: () => jsonStruct.version === VERSION,
    jsonStruct,
    hasProperty: (key: string): boolean => {
      return key in jsonStruct
    },
    getBookmarks: (): BookmarkFolder[] => {
      return jsonStruct.bookmarks || []
    },
  }
}

function transformJson(jsonData: any): BookmarkJsonStructure | null {
  // 检查必须存在的键
  const requiredKeys: Array<keyof BookmarkJsonStructure> = ['version', 'appName', 'exportTime', 'appVersion', 'md5']
  for (const key of requiredKeys) {
    if (!(key in jsonData)) {
      return null
    }
  }

  // 使用类型断言将 JSON 数据转换为指定类型
  const transformedData: BookmarkJsonStructure = jsonData as BookmarkJsonStructure

  // 返回转换后的数据
  return transformedData
}

function generateMD5(jsonString: string): string {
  try {
    // 解析 JSON 字符串
    const data: any = JSON.parse(jsonString)
    // 移除 md5 字段及其对应的值
    removeMD5Field(data)
    // 将修改后的 JSON 对象转换回字符串
    const modifiedJsonString = JSON.stringify(data)
    // 使用 crypto-js 计算 MD5 值
    const md5 = CryptoJS.MD5(modifiedJsonString).toString()
    return md5
  } catch (error) {
    return ''
  }
}

function removeMD5Field(obj: any): void {
  for (const key in obj) {
    if (key === 'md5') {
      // 移除 md5 字段
      delete obj[key]
      return
    }
  }
}

// 解析浏览器导出的HTML书签文件
// 注意：浏览器导出的 Netscape Bookmark HTML 结构通常是：
// <DT><H3>文件夹</H3>
// <DL><p> ...children... </DL><p>
// 因此不能简单依赖“当前文件夹”状态机，否则同级文件夹容易被错误嵌套。
export function parseBrowserBookmarkHTML(htmlContent: string): BookmarkTreeNode[] {
  const parser = new DOMParser()
  const doc = parser.parseFromString(htmlContent, 'text/html')
  const firstDl = doc.querySelector('dl')

  if (!firstDl)
    return []

  return parseDLChildren(firstDl)
}

function parseDLChildren(dlElement: Element): BookmarkTreeNode[] {
  const result: BookmarkTreeNode[] = []
  const children = Array.from(dlElement.children)

  for (let i = 0; i < children.length; i++) {
    const element = children[i]
    const tagName = element.tagName.toLowerCase()
    if (tagName === 'p')
      continue

    if (tagName !== 'dt')
      continue

    const directChildren = Array.from(element.children)
    const firstChild = directChildren.find(child => {
      const tag = child.tagName.toLowerCase()
      return tag === 'h3' || tag === 'a'
    })

    if (!firstChild)
      continue

    // 文件夹节点：其子节点在紧随其后的同级 DL 中
    if (firstChild.tagName.toLowerCase() === 'h3') {
      const folder: BookmarkFolder = {
        id: generateUniqueId(),
        title: firstChild.textContent?.trim() || '未命名文件夹',
        children: [],
        isFolder: true,
      }

      // 浏览器实际解析 Netscape bookmark HTML 时，嵌套的 <DL> 可能：
      // 1) 作为当前 <DT> 的直接子元素；
      // 2) 作为当前 <DT> 的下一个同级元素。
      // 两种都兼容，优先取直接子元素。
      let nestedDl = directChildren.find(child => child.tagName.toLowerCase() === 'dl') || null

      if (!nestedDl) {
        let nextElement = element.nextElementSibling
        while (nextElement && nextElement.tagName.toLowerCase() === 'p')
          nextElement = nextElement.nextElementSibling

        if (nextElement && nextElement.tagName.toLowerCase() === 'dl')
          nestedDl = nextElement
      }

      if (nestedDl)
        folder.children = parseDLChildren(nestedDl)

      result.push(folder)
      continue
    }

    // 书签节点
    const bookmarkElement = firstChild as HTMLAnchorElement
    const bookmarkItem: BookmarkItem = {
      id: generateUniqueId(),
      title: bookmarkElement.textContent?.trim() || '未命名书签',
      url: bookmarkElement.getAttribute('href') || '',
      isFolder: false,
      icon: bookmarkElement.getAttribute('icon') || undefined,
    }

    result.push(bookmarkItem)
  }

  return result
}

// 生成唯一ID的工具函数
export function generateUniqueId(): number {
  return Date.now() + Math.floor(Math.random() * 1000)
}

// 扁平化书签树
export function flattenBookmarkTree(bookmarks: BookmarkTreeNode[]): BookmarkTreeNode[] {
  const result: BookmarkTreeNode[] = []
  
  function traverse(nodes: (BookmarkItem | BookmarkFolder)[], parentId?: number) {
    for (const node of nodes) {
      if (node.isFolder) {
        const folder = node as BookmarkFolder
        // 包含文件夹本身
        result.push({
          ...folder,
          folderId: parentId
        })
        // 继续遍历子节点
        if (folder.children) {
          traverse(folder.children, folder.id)
        }
      } else {
        const bookmark = node as BookmarkItem
        result.push({
          ...bookmark,
          folderId: parentId
        })
      }
    }
  }
  
  traverse(bookmarks)
  return result
}
