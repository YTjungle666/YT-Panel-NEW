<script setup lang="ts">
import { defineEmits, onBeforeUnmount, onMounted, ref, computed, nextTick, watch } from 'vue'
import { NAvatar, NCheckbox, useMessage } from 'naive-ui'
import { SvgIcon } from '@/components/common'
import { useModuleConfig } from '@/store/modules'
import { useAuthStore } from '@/store'
import { VisitMode } from '@/enums/auth'
import { ss } from '@/utils/storage/local'
import { t } from '@/locales'

import SvgSrcBing from '@/assets/search_engine_svg/bing.svg'
import SvgSrcGoogle from '@/assets/search_engine_svg/google.svg'
import { openUrlWithoutReferer } from '@/utils/cmn'

const props = withDefaults(defineProps<{
  background?: string
  textColor?: string
  searchItems?: Panel.ItemInfo[]
}>(), {
  background: '#2a2a2a6b',
  textColor: 'white',
  searchItems: () => [],
})

const emits = defineEmits(['itemSearch', 'itemSelect'])

interface State {
  currentSearchEngine: DeskModule.SearchBox.SearchEngine
  searchEngineList: DeskModule.SearchBox.SearchEngine[]
  newWindowOpen: boolean
  searchBookmarks: boolean
}

interface SuggestionItem {
  value: string
  type?: 'bookmark' | 'item' | 'search'
  isBookmark?: boolean
  url?: string
  item?: Panel.ItemInfo
  [key: string]: any // 其他可能的属性
}

interface Bookmark {
  id: number
  title: string
  url: string
  folderId: string | null
  iconJson?: string
  sort?: number
}

interface TreeItem {
  key: string | number;
  label: string;
  isLeaf: boolean;
  bookmark?: Bookmark;
  children?: TreeItem[];
}

const moduleConfigName = 'deskModuleSearchBox'
const moduleConfig = useModuleConfig()
const authStore = useAuthStore()
const ms = useMessage()
const searchTerm = ref('')
const isFocused = ref(false)
const searchSelectListShow = ref(false)
const suggestionsVisible = ref(false)
const dropdownPosition = ref<'bottom' | 'top'>('bottom')
const searchInputRef = ref<HTMLInputElement | null>(null)
const dropdownRef = ref<HTMLDivElement | null>(null)
const suggestionOptions = ref<SuggestionItem[]>([])

// 书签缓存键名
const BOOKMARKS_CACHE_KEY = 'bookmarksTreeCache'
// 搜索引擎列表缓存键名
const SEARCH_ENGINE_LIST_CACHE_KEY = 'searchEngineListCache'

// 将服务器返回的树形结构转换为前端组件需要的格式
function convertServerTreeToFrontendTree(serverTree: any[]): TreeItem[] {
  // 先对顶层节点按sort字段排序
  const sortedServerTree = [...serverTree].sort((a, b) => (a.sort || 0) - (b.sort || 0));
  const result = sortedServerTree.map(node => {
    // 处理两种可能的节点结构：
    // 1. 服务器原始数据格式 (id, title, isFolder, url, iconJson)
    // 2. 前端节点格式 (key, label, isFolder, bookmark)
    const isFrontendFormat = node.hasOwnProperty('key') && node.hasOwnProperty('label');

    // 提取基本属性
    const nodeId = isFrontendFormat ? node.key : node.id;
    const title = isFrontendFormat ? node.label : node.title;
    const isFolder = isFrontendFormat ? (node.isFolder ? 1 : 0) : node.isFolder;
    const url = isFrontendFormat ? (node.bookmark?.url || '') : node.url;
    const iconJson = isFrontendFormat ? (node.bookmark?.iconJson || '') : node.iconJson;
    const parentId = isFrontendFormat ? (node.rawNode?.parentId || node.ParentId || '0') : (node.parentId || node.ParentId || '0');

    // 提取排序字段
    const sortOrder = node.sort || 0;

    // 处理bookmark对象
    let bookmarkObj = undefined;
    if (isFolder !== 1 && url) {
      // 确保folderId是字符串类型
      const folderId = parentId !== undefined ? String(parentId) : null;
      bookmarkObj = {
        id: nodeId,
        title: title,
        url: url,
        folderId: folderId,
        iconJson: iconJson,
        sort: sortOrder
      };
    }

    const frontendNode: TreeItem = {
        key: nodeId,
        label: title || '未命名',
        isLeaf: isFolder !== 1,
        bookmark: bookmarkObj
    };

    // 递归处理子节点
    if (node.children && node.children.length > 0) {
      // 对子节点先按sort字段排序再递归转换
      const sortedChildren = [...node.children].sort((a, b) => (a.sort || 0) - (b.sort || 0));
      frontendNode.children = convertServerTreeToFrontendTree(sortedChildren);
    }

    return frontendNode;
  });

  return result;
}

// 构建书签树
function buildBookmarkTree(bookmarks: any[]): TreeItem[] {
  // 首先分离文件夹和书签
  const folders = bookmarks.filter(b => {
    return (b.isFolder === 1 || (b.isFolder && typeof b.isFolder === 'boolean'));
  });
  const items = bookmarks.filter(b => {
    return (b.isFolder === 0 || (!b.isFolder && typeof b.isFolder === 'boolean'));
  });

  // 构建文件夹树
  const rootFolders: TreeItem[] = []
  const folderMap = new Map<string, TreeItem>() // 使用字符串键

  // 先创建所有文件夹节点
  folders.forEach(folder => {
    // 处理两种可能的文件夹结构
    const isFrontendFormat = folder.hasOwnProperty('key') && folder.hasOwnProperty('label');
    const folderId = isFrontendFormat ? folder.key : folder.id;
    const folderTitle = isFrontendFormat ? folder.label : folder.title;
    const folderNode: TreeItem = {
      key: folderId,
      label: folderTitle,
      children: [],
      isLeaf: false
    };
    // 使用id作为map的键
    folderMap.set(folderId.toString(), folderNode);
    // 同时也将文件夹名称作为键，以便处理嵌套关系
    folderMap.set(folderTitle, folderNode);
  });

  // 将文件夹添加到其父文件夹中
  folders.forEach(folder => {
    const folderNode = folderMap.get(folder.id.toString())
    // 检查是否有ParentUrl并且不是根节点(0)
    if (folder.ParentUrl && folder.ParentUrl !== '0' && folder.ParentUrl !== 0) {
      // 尝试用不同的方式查找父文件夹
      let parentFolder = folderMap.get(folder.ParentUrl.toString())

      if (!parentFolder) {
        // 如果找不到，尝试用文件夹标题匹配
        parentFolder = folderMap.get(folder.ParentUrl)
      }

      if (parentFolder) {
        parentFolder.children?.push(folderNode!)
        return
      }
    }
    // 如果没有父文件夹或父文件夹不存在，则作为根文件夹
    rootFolders.push(folderNode!)
  })

  // 将书签项添加到对应的文件夹中
  items.forEach(item => {
    // 处理两种可能的书签结构
    const isFrontendFormat = item.hasOwnProperty('key') && item.hasOwnProperty('label');
    // 提取书签基本信息
    const bookmarkId = isFrontendFormat ? item.key : item.id;
    const bookmarkTitle = isFrontendFormat ? item.label : (item.title || '未命名');
    const bookmarkUrl = isFrontendFormat ? (item.bookmark?.url || '') : (item.url || '');
    const bookmarkIconJson = isFrontendFormat ? (item.bookmark?.iconJson || '') : (item.iconJson || '');
    // 确保folderId是字符串类型
    const folderId = isFrontendFormat ? (item.rawNode?.parentId || item.ParentId || '0') : (item.parentId || item.ParentId || '0');
    const stringFolderId = String(folderId);

    let targetFolder;

    if (stringFolderId === '0' || stringFolderId === 'null' || stringFolderId === 'undefined') {
      // 根目录的书签，创建一个"未分类"文件夹
      targetFolder = folderMap.get('未分类');
      if (!targetFolder) {
        targetFolder = {
          key: '未分类',
          label: '未分类',
          children: [],
          isLeaf: false
        };
        folderMap.set('未分类', targetFolder);
        rootFolders.push(targetFolder);
      }
    } else {
      // 查找对应的文件夹
      targetFolder = folderMap.get(stringFolderId);
    }

    if (targetFolder) {
      // 创建书签节点
      const bookmarkNode: TreeItem = {
        key: bookmarkId,
        label: bookmarkTitle,
        isLeaf: true,
        bookmark: {
          id: bookmarkId,
          title: bookmarkTitle,
          url: bookmarkUrl,
          folderId: stringFolderId,
          iconJson: bookmarkIconJson
        }
      };
      targetFolder.children?.push(bookmarkNode);
    } else {
      // 如果找不到对应的文件夹，直接添加到根目录
      const bookmarkNode: TreeItem = {
        key: bookmarkId,
        label: bookmarkTitle,
        isLeaf: true,
        bookmark: {
          id: bookmarkId,
          title: bookmarkTitle,
          url: bookmarkUrl,
          folderId: stringFolderId,
          iconJson: bookmarkIconJson
        }
      };
      rootFolders.push(bookmarkNode);
    }
  });

  return rootFolders;
}

// 搜索书签
function searchBookmarks(keyword: string): SuggestionItem[] {
  const results: SuggestionItem[] = []
  const lowerCaseKeyword = keyword.toLowerCase()

  // 从localStorage获取已有的书签数据
  const cachedData = ss.get(BOOKMARKS_CACHE_KEY)
  if (!cachedData) {
    return results
  }

  let bookmarksTree: TreeItem[] = []

  // 处理缓存的数据格式，转换为树形结构
  if (Array.isArray(cachedData)) {
    // 检查是否已经是树形结构（直接包含children字段）
    if (cachedData.length > 0 && 'children' in cachedData[0]) {
      bookmarksTree = convertServerTreeToFrontendTree(cachedData)
    } else if (cachedData[0]?.hasOwnProperty('id') || cachedData[0]?.hasOwnProperty('key')) {
      // 如果是书签数组，构建树形结构
      bookmarksTree = buildBookmarkTree(cachedData)
    }
  } else if (cachedData && typeof cachedData === 'object') {
    // 如果是对象，检查是否有list字段
    if (Array.isArray(cachedData.list)) {
      // 处理list字段中的书签数据
      if (cachedData.list.length > 0 && 'children' in cachedData.list[0]) {
        bookmarksTree = convertServerTreeToFrontendTree(cachedData.list)
      } else {
        bookmarksTree = buildBookmarkTree(cachedData.list)
      }
    } else if (Array.isArray(cachedData.data)) {
      // 处理data字段中的书签数据
      if (cachedData.data.length > 0 && 'children' in cachedData.data[0]) {
        bookmarksTree = convertServerTreeToFrontendTree(cachedData.data)
      } else {
        bookmarksTree = buildBookmarkTree(cachedData.data)
      }
    }
  }

  // 递归搜索书签
  function traverse(node: TreeItem) {
    if (node.isLeaf && node.bookmark) {
      const title = node.bookmark.title.toLowerCase()
      const url = node.bookmark.url.toLowerCase()

      if (title.includes(lowerCaseKeyword) || url.includes(lowerCaseKeyword)) {
        results.push({
          value: node.bookmark.title,
          type: 'bookmark',
          isBookmark: true,
          url: node.bookmark.url
        })
      }
    }

    if (node.children && node.children.length > 0) {
      for (const child of node.children) {
        traverse(child)
      }
    }
  }

  for (const node of bookmarksTree) {
    traverse(node)
  }

  return results
}

function includesKeyword(value: string | undefined | null, keyword: string) {
  return (value || '').toLowerCase().includes(keyword)
}

function searchTextIconItems(keyword: string): SuggestionItem[] {
  const lowerCaseKeyword = keyword.toLowerCase()

  return props.searchItems
    .filter(item => item.icon?.itemType === 1)
    .filter(item => (
      includesKeyword(item.title, lowerCaseKeyword)
      || includesKeyword(item.url, lowerCaseKeyword)
      || includesKeyword(item.description, lowerCaseKeyword)
      || includesKeyword(item.icon?.text, lowerCaseKeyword)
    ))
    .map(item => ({
      value: item.title || item.icon?.text || item.url,
      type: 'item',
      url: item.url,
      item,
    }))
}

function dedupeSuggestions(suggestions: SuggestionItem[]) {
  const seen = new Set<string>()

  return suggestions.filter((suggestion) => {
    const key = `${suggestion.type || 'search'}::${suggestion.url || suggestion.value}`
    if (seen.has(key))
      return false

    seen.add(key)
    return true
  })
}

// 键盘导航相关
const selectedIndex = ref(-1)

// 加载状态
const loadingSuggestions = ref(false)

import { getList, add, update, deletes, updateSort } from '@/api/panel/searchEngine'

// 搜索引擎管理对话框相关状态
const searchEngineDialogVisible = ref(false)
const editingSearchEngine = ref<DeskModule.SearchBox.SearchEngine | null>(null)
const editingSearchEngineIndex = ref(-1)
const searchEngineForm = ref({
  id: 0,
  iconSrc: '',
  title: '',
  url: ''
})
const draggedEngineIndex = ref<number | null>(null)

const defaultSearchEngineList = ref<DeskModule.SearchBox.SearchEngine[]>([])
const LEGACY_BAIDU_URL = 'https://www.baidu.com/s?wd=%s'

function isLegacyDefaultBaiduEngine(engine: DeskModule.SearchBox.SearchEngine) {
  return engine.title === 'Baidu' && engine.url === LEGACY_BAIDU_URL
}

async function removeLegacyDefaultBaiduEngines(engines: DeskModule.SearchBox.SearchEngine[]) {
  const legacyBaiduEngines = engines.filter(engine => engine.id && isLegacyDefaultBaiduEngine(engine))
  if (legacyBaiduEngines.length === 0)
    return engines

  for (const engine of legacyBaiduEngines) {
    if (engine.id)
      await deletes({ id: engine.id })
  }

  const { code, data } = await getList()
  if (code === 0)
    return (data && data.list) || []

  return engines.filter(engine => !isLegacyDefaultBaiduEngine(engine))
}

// 初始化加载搜索引擎列表
const initSearchEngines = async (forceRefresh = false) => {
  try {
    if (forceRefresh) {
      ss.remove(SEARCH_ENGINE_LIST_CACHE_KEY)
    }

    if (!forceRefresh) {
      const cachedData = ss.get(SEARCH_ENGINE_LIST_CACHE_KEY)
      if (cachedData) {
        defaultSearchEngineList.value = cachedData
        // 检查当前选中的搜索引擎是否有效
        checkCurrentEngine()
        return
      }
    }

    const { code, data } = await getList()
    if (code === 0) {
      const loadedEngines = await removeLegacyDefaultBaiduEngines((data && data.list) || [])
      defaultSearchEngineList.value = loadedEngines

      // 保存到缓存
      ss.set(SEARCH_ENGINE_LIST_CACHE_KEY, defaultSearchEngineList.value)

      // 如果列表为空（首次运行），添加默认数据
      if (defaultSearchEngineList.value.length === 0) {
         await createDefaultEngines()
      } else {
         // 检查当前选中的搜索引擎是否有效
         checkCurrentEngine()
      }
    }
  } catch (error) {
    console.error('Failed to load search engines:', error)
  }
}

// 创建默认搜索引擎
const createDefaultEngines = async () => {
  const defaults = [
    {
      iconSrc: SvgSrcGoogle,
      title: 'Google',
      url: 'https://www.google.com/search?q=%s',
    },
    {
      iconSrc: SvgSrcBing,
      title: 'Bing',
      url: 'https://www.bing.com/search?q=%s',
    },
  ]

  for (const engine of defaults) {
    await add(engine)
  }

  // 重新加载列表
  const { code, data } = await getList()
  if (code === 0) {
    defaultSearchEngineList.value = (data && data.list) || []
    // 保存到缓存
    ss.set(SEARCH_ENGINE_LIST_CACHE_KEY, defaultSearchEngineList.value)
    // 设置默认选中第一个
    if (defaultSearchEngineList.value.length > 0) {
       state.value.currentSearchEngine = defaultSearchEngineList.value[0]
       moduleConfig.saveToCloud(moduleConfigName, state.value)
    }
  }
}

// 检查当前选中的搜索引擎
const checkCurrentEngine = () => {
  if (!state.value.currentSearchEngine || !state.value.currentSearchEngine.url) {
    if (defaultSearchEngineList.value.length > 0) {
      state.value.currentSearchEngine = defaultSearchEngineList.value[0]
    }
    return
  }

  // 确保当前选中的搜索引擎仍存在于列表中
  const match = defaultSearchEngineList.value.find((engine) => {
    if (state.value.currentSearchEngine.id && engine.id)
      return engine.id === state.value.currentSearchEngine.id

    return engine.url === state.value.currentSearchEngine.url
  })
  if (match) {
    state.value.currentSearchEngine = match
    return
  }

  if (defaultSearchEngineList.value.length > 0) {
    state.value.currentSearchEngine = defaultSearchEngineList.value[0]
  }
}

// 打开搜索引擎管理对话框
function openSearchEngineDialog() {
  searchEngineDialogVisible.value = true
}

// 关闭搜索引擎管理对话框
function closeSearchEngineDialog() {
  searchEngineDialogVisible.value = false
  resetSearchEngineForm()
}

// 重置表单
function resetSearchEngineForm() {
  searchEngineForm.value = {
    id: 0,
    iconSrc: '',
    title: '',
    url: ''
  }
  editingSearchEngine.value = null
  editingSearchEngineIndex.value = -1
}

// 开始编辑搜索引擎
function startEditSearchEngine(engine: DeskModule.SearchBox.SearchEngine, index: number) {
  editingSearchEngine.value = engine
  editingSearchEngineIndex.value = index
  searchEngineForm.value = {
    id: engine.id!, // 确保有ID
    iconSrc: engine.iconSrc,
    title: engine.title,
    url: engine.url
  }
}

// 保存搜索引擎
async function saveSearchEngine() {
  if (!searchEngineForm.value.title || !searchEngineForm.value.url) {
    return
  }

  try {
    if (editingSearchEngineIndex.value >= 0) {
      // 编辑现有搜索引擎
      const { code } = await update({
        id: searchEngineForm.value.id,
        title: searchEngineForm.value.title,
        url: searchEngineForm.value.url,
        iconSrc: searchEngineForm.value.iconSrc,
      })
      if (code === 0) {
        ms.success(t('common.saveSuccess') || '保存成功')
        closeSearchEngineDialog()
      } else {
        return // 失败不重置
      }
    } else {
      // 添加新搜索引擎
      const { code } = await add({
        title: searchEngineForm.value.title,
        url: searchEngineForm.value.url,
        iconSrc: searchEngineForm.value.iconSrc,
      })
      if (code === 0) {
        ms.success(t('common.addSuccess') || '添加成功')
        closeSearchEngineDialog()
      } else {
        return
      }
    }
  } catch (error) {
     ms.error(t('common.failed') || '操作失败')
     return
  }

  // 重新加载列表（强制刷新）
  await initSearchEngines(true)
  resetSearchEngineForm()
}

// 删除搜索引擎
async function deleteSearchEngine(index: number) {
  const engine = defaultSearchEngineList.value[index]
  if (!engine.id) return

  try {
    const { code } = await deletes({ id: engine.id })
    if (code === 0) {
       ms.success(t('common.deleteSuccess') || '删除成功')
       // 如果删除的是当前选中的搜索引擎，切换到第一个
        if (state.value.currentSearchEngine?.url === engine.url) {
            // 稍后在initSearchEngines中会处理
        }
        // 重新加载列表（强制刷新）
        await initSearchEngines(true)
    } else {
        ms.error(t('common.deleteFail') || '删除失败')
    }
  } catch (error) {
     ms.error(t('common.deleteFail') || '删除失败')
  }
}

// 拖拽开始
function handleDragStart(index: number) {
  draggedEngineIndex.value = index
}

// 拖拽结束
async function handleDragEnd() {
  draggedEngineIndex.value = null

  // 保存排序
  const items = defaultSearchEngineList.value.map((item, index) => ({
    id: item.id!,
    sort: index + 1
  }))

  try {
     const { code } = await updateSort({ items })
     if (code === 0) {
        // ms.success(t('common.saveSort') || '排序保存成功') // 可选提示
     }
  } catch (error) {
     console.error('Failed to save sort order:', error)
  }
}

// 拖拽经过
function handleDragOver(e: DragEvent, index: number) {
  e.preventDefault()
  if (draggedEngineIndex.value === null || draggedEngineIndex.value === index) {
    return
  }

  const draggedItem = defaultSearchEngineList.value[draggedEngineIndex.value]
  const newList = [...defaultSearchEngineList.value]
  newList.splice(draggedEngineIndex.value, 1)
  newList.splice(index, 0, draggedItem)
  defaultSearchEngineList.value = newList
  draggedEngineIndex.value = index
}

// 处理图标上传
function handleIconUpload(e: Event) {
  const target = e.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file) return

  const reader = new FileReader()
  reader.onload = (event) => {
    searchEngineForm.value.iconSrc = event.target?.result as string
  }
  reader.readAsDataURL(file)
}

const defaultState: State = {
  currentSearchEngine: defaultSearchEngineList.value[0],
  searchEngineList: defaultSearchEngineList.value,
  newWindowOpen: true,
  searchBookmarks: true
}

const state = ref<State>({ ...defaultState })

// 过滤后的提示词选项
const filteredSuggestions = computed(() => {
  return suggestionOptions.value.slice(0, 8)
})

const suggestionCache = new Map<string, SuggestionItem[]>()
let latestSuggestionRequestId = 0
let suggestionDebounceTimer: ReturnType<typeof setTimeout> | null = null
let blurHideTimer: ReturnType<typeof setTimeout> | null = null

// 监听搜索词变化，获取动态提示词
watch(searchTerm, (newVal) => {
  selectedIndex.value = -1

  if (suggestionDebounceTimer) {
    clearTimeout(suggestionDebounceTimer)
    suggestionDebounceTimer = null
  }

  if (newVal) {
    suggestionDebounceTimer = setTimeout(() => {
      suggestionDebounceTimer = null
      void fetchSuggestions(newVal)
    }, 250)
  } else {
    suggestionOptions.value = []
  }
})

// 获取搜索建议
const fetchSuggestions = async (keyword: string) => {
  const trimmedKeyword = keyword.trim()
  if (!trimmedKeyword) return

  const requestId = ++latestSuggestionRequestId
  const cacheKey = `${state.value.currentSearchEngine.title}:${trimmedKeyword}`
  const cachedSuggestions = suggestionCache.get(cacheKey)
  if (cachedSuggestions) {
    suggestionOptions.value = cachedSuggestions
    return
  }

  loadingSuggestions.value = true
  try {
    const bookmarkSuggestions = state.value.searchBookmarks ? searchBookmarks(trimmedKeyword) : []
    const itemSuggestions = state.value.searchBookmarks ? searchTextIconItems(trimmedKeyword) : []

    let searchEngineSuggestions: SuggestionItem[] = []
    if (trimmedKeyword.length >= 2) {
      searchEngineSuggestions = getDefaultSuggestions(trimmedKeyword)
    }

    const allSuggestions: SuggestionItem[] = dedupeSuggestions([
      ...bookmarkSuggestions,
      ...itemSuggestions,
      ...searchEngineSuggestions,
    ])

    if (requestId !== latestSuggestionRequestId)
      return

    suggestionOptions.value = allSuggestions
    suggestionCache.set(cacheKey, allSuggestions)
  } catch {
    const defaultSuggestions = getDefaultSuggestions(trimmedKeyword)
    const bookmarkSuggestions = state.value.searchBookmarks ? searchBookmarks(trimmedKeyword) : []
    const itemSuggestions = state.value.searchBookmarks ? searchTextIconItems(trimmedKeyword) : []
    const allSuggestions: SuggestionItem[] = dedupeSuggestions([
      ...bookmarkSuggestions,
      ...itemSuggestions,
      ...defaultSuggestions,
    ])

    if (requestId !== latestSuggestionRequestId)
      return

    suggestionOptions.value = allSuggestions
  } finally {
    if (requestId === latestSuggestionRequestId)
      loadingSuggestions.value = false
  }
}


// 默认建议词（当API不可用时使用）
const getDefaultSuggestions = (keyword: string): SuggestionItem[] => {
  const defaults = [
    '天气预报',
    '最新新闻',
    '股票行情',
    '电影推荐',
    '菜谱大全',
    '旅游攻略',
    '学习资料',
    '技术文档'
  ]

  // 根据关键词过滤默认建议
  return defaults
    .filter(item => item.includes(keyword))
    .map(item => ({ value: item, type: 'search' }))
}

const onFocus = (): void => {
  isFocused.value = true
  suggestionsVisible.value = true
  nextTick(() => {
    calculateDropdownPosition()
  })

  // 获取初始建议词
  if (searchTerm.value) {
    fetchSuggestions(searchTerm.value)
  }
}

const onBlur = (): void => {
  // 添加延迟以允许点击下拉项
  if (blurHideTimer)
    clearTimeout(blurHideTimer)

  blurHideTimer = setTimeout(() => {
    isFocused.value = false
    suggestionsVisible.value = false
    blurHideTimer = null
  }, 200)
}

// 计算下拉框位置
const calculateDropdownPosition = () => {
  if (!searchInputRef.value) return

  const inputRect = searchInputRef.value.getBoundingClientRect()
  const viewportHeight = window.innerHeight
  const spaceBelow = viewportHeight - inputRect.bottom
  const dropdownHeight = 200 // 预估高度

  // 如果下方空间不足且上方空间足够，则显示在上方
  if (spaceBelow < dropdownHeight && inputRect.top > dropdownHeight) {
    dropdownPosition.value = 'top'
  } else {
    dropdownPosition.value = 'bottom'
  }
}

function handleEngineClick() {
  // 访客模式不允许修改
  if (authStore.visitMode === VisitMode.VISIT_MODE_PUBLIC)
    return
  searchSelectListShow.value = !searchSelectListShow.value
}

function handleEngineUpdate(engine: DeskModule.SearchBox.SearchEngine) {
  state.value.currentSearchEngine = engine
  moduleConfig.saveToCloud(moduleConfigName, state.value)
  searchSelectListShow.value = false

  // 更换搜索引擎后重新获取建议词
  if (searchTerm.value) {
    fetchSuggestions(searchTerm.value)
  }
}

function getSearchOpenTarget() {
  return state.value.newWindowOpen ? '_blank' : '_self'
}

function handleSearchClick() {
  if (!searchTerm.value.trim())
    return
  const url = state.value.currentSearchEngine.url
  const keyword = searchTerm
  // 如果网址中存在 %s，则直接替换为关键字
  const fullUrl = replaceOrAppendKeywordToUrl(url, keyword.value)
  handleClearSearchTerm()
  openUrlWithoutReferer(fullUrl, getSearchOpenTarget())
}

function handleSuggestionSelect(suggestion: SuggestionItem) {
  if (suggestion.type === 'item' && suggestion.item) {
    emits('itemSelect', suggestion.item)
    handleClearSearchTerm()
    return
  }

  if (suggestion.isBookmark && suggestion.url) {
    // 如果是书签项，直接打开书签URL
    openUrlWithoutReferer(suggestion.url, getSearchOpenTarget())
    // 清空搜索框并关闭建议列表
    handleClearSearchTerm()
  } else {
    // 否则执行正常搜索
    searchTerm.value = suggestion.value
    suggestionsVisible.value = false
    // 触发搜索
    nextTick(() => {
      handleSearchClick()
    })
  }
}

function replaceOrAppendKeywordToUrl(url: string, keyword: string) {
  // 如果网址中存在 %s，则直接替换为关键字
  if (url.includes('%s'))
    return url.replace('%s', encodeURIComponent(keyword))

  // 如果网址中不存在 %s，则将关键字追加到末尾
  return url + (keyword ? `${encodeURIComponent(keyword)}` : '')
}

const handleItemSearch = () => {
  emits('itemSearch', searchTerm.value)
  // 输入时也显示建议
  suggestionsVisible.value = true
}

// 处理键盘事件
const handleKeyDown = (e: KeyboardEvent) => {
  // 解决输入法回车问题：如果正在合成（选词），则不处理回车
  if (e.isComposing)
    return

  // 如果按下的是回车键
  if (e.key === 'Enter') {
    // 如果输入框为空，则不执行任何逻辑，且阻止冒泡和默认行为
    if (!searchTerm.value.trim()) {
      e.preventDefault()
      e.stopPropagation()
      return
    }

    // 如果当前有选中的建议项，先处理建议项选择（在下面的逻辑中处理）
    // 如果没有建议项或者没有选中，则执行搜索
    if (!suggestionsVisible.value || filteredSuggestions.value.length === 0 || selectedIndex.value < 0) {
      e.preventDefault()
      handleSearchClick()
      return
    }
  }

  // 只有在提示框可见且有提示词时才处理键盘事件（后续的上下箭头等）
  if (!suggestionsVisible.value || filteredSuggestions.value.length === 0)
    return

  // 下箭头：选中下一项
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    selectedIndex.value = (selectedIndex.value + 1) % filteredSuggestions.value.length
  }
  // 上箭头：选中上一项
  else if (e.key === 'ArrowUp') {
    e.preventDefault()
    selectedIndex.value = (selectedIndex.value - 1 + filteredSuggestions.value.length) % filteredSuggestions.value.length
  }
  // 回车：搜索选中项
  else if (e.key === 'Enter') {
    e.preventDefault()
    if (selectedIndex.value >= 0 && filteredSuggestions.value.length > 0) {
      const selectedItem = filteredSuggestions.value[selectedIndex.value]
      handleSuggestionSelect(selectedItem)
    } else {
      handleSearchClick()
    }
  }
  // ESC：关闭提示框
  else if (e.key === 'Escape') {
    suggestionsVisible.value = false
    selectedIndex.value = -1
  }
}

function handleClearSearchTerm() {
  searchTerm.value = ''
  emits('itemSearch', searchTerm.value)
  suggestionsVisible.value = false
  suggestionOptions.value = []
  selectedIndex.value = -1
}

onMounted(() => {
  moduleConfig.getValueByNameFromCloud<State>('deskModuleSearchBox').then(({ code, data }) => {
    if (code === 0)
      state.value = { ...defaultState, ...(data || {}) }
    else
      state.value = { ...defaultState }

    // 加载搜索引擎列表
    initSearchEngines()
  })
})

onBeforeUnmount(() => {
  latestSuggestionRequestId += 1
  loadingSuggestions.value = false

  if (suggestionDebounceTimer) {
    clearTimeout(suggestionDebounceTimer)
    suggestionDebounceTimer = null
  }

  if (blurHideTimer) {
    clearTimeout(blurHideTimer)
    blurHideTimer = null
  }
})
</script>

<template>
  <div class="search-box w-full" @keydown.esc="handleClearSearchTerm">
    <div class="search-container flex rounded-2xl items-center justify-center text-white w-full relative" :style="{ background, color: textColor }" :class="{ focused: isFocused }">
      <div class="search-box-btn-engine w-[40px] flex justify-center cursor-pointer" @click="handleEngineClick">
        <NAvatar :src="state.currentSearchEngine?.iconSrc || defaultSearchEngineList[0]?.iconSrc" style="background-color: transparent;" :size="20" />
      </div>

      <input
        ref="searchInputRef"
        v-model="searchTerm"
        :placeholder="$t('deskModule.searchBox.inputPlaceholder')"
        @focus="onFocus"
        @blur="onBlur"
        @input="handleItemSearch"
        @keydown="handleKeyDown"
        class="search-input"
      >

      <div v-if="searchTerm !== ''" class="search-box-btn-clear w-[25px] mr-[10px] flex justify-center cursor-pointer" @click="handleClearSearchTerm">
        <SvgIcon style="width: 20px;height: 20px;" icon="line-md:close-small" />
      </div>
      <div class="search-box-btn-search w-[25px] flex justify-center cursor-pointer" @click="handleSearchClick">
        <SvgIcon style="width: 20px;height: 20px;" icon="iconamoon:search-fill" />
      </div>

      <!-- 提示词下拉框 -->
      <div
        v-if="suggestionsVisible && (filteredSuggestions.length > 0 || loadingSuggestions)"
        ref="dropdownRef"
        class="suggestions-dropdown absolute left-0 w-full rounded-xl overflow-hidden z-10 shadow-lg"
        :class="dropdownPosition === 'bottom' ? 'top-full mt-[5px]' : 'bottom-full mb-[5px]'"
        :style="{ background }"
      >
        <!-- 加载状态 -->
        <div v-if="loadingSuggestions" class="suggestion-item px-4 py-2 flex items-center" :style="{ color: textColor }">
          <span class="loading-spinner mr-2"></span>
          {{ $t('deskModule.searchBox.loading') || '加载中...' }}
        </div>

        <!-- 建议列表 -->
      <div
        v-else
        v-for="(suggestion, index) in filteredSuggestions"
        :key="index"
        class="suggestion-item px-4 py-2 cursor-pointer hover:bg-white/10 transition-colors flex items-center justify-between"
        :class="{ 'active': index === selectedIndex }"
        :style="{ color: textColor }"
        @mousedown="handleSuggestionSelect(suggestion)"
        @mouseenter="selectedIndex = index"
      >
        <div class="flex items-center">
          <SvgIcon icon="mdi:magnify" class="mr-2" />
          {{ suggestion.value }}
        </div>
        <div v-if="suggestion.isBookmark" class="ml-2 text-xs opacity-80">
          [{{ $t('deskModule.searchBox.bookmark') || '书签' }}]
        </div>
        <div v-else-if="suggestion.type === 'item'" class="ml-2 text-xs opacity-80">
          [项目]
        </div>
      </div>
      </div>
    </div>

    <!-- 搜索引擎选择 -->
    <div v-if="searchSelectListShow" class="w-full mt-[10px] rounded-xl p-[10px]" :style="{ background }">
      <div class="flex items-center">
        <div class="flex items-center">
          <div
            v-for="(item, index) in defaultSearchEngineList"
            :key="(item as any).id || index"
            :title="item.title"
            class="w-[40px] h-[40px] mr-[10px]  cursor-pointer bg-[#ffffff] flex items-center justify-center rounded-xl"
            @click="handleEngineUpdate(item)"
          >
            <NAvatar :src="item.iconSrc" style="background-color: transparent;" :size="20" />
          </div>
        </div>
      </div>

      <div class="mt-[10px] flex items-center space-x-[20px]">
        <NCheckbox v-model:checked="state.newWindowOpen" @update-checked="moduleConfig.saveToCloud(moduleConfigName, state)">
          <span :style="{ color: textColor }">
            {{ $t('deskModule.searchBox.openWithNewOpen') }}
          </span>
        </NCheckbox>
        <NCheckbox v-model:checked="state.searchBookmarks" @update-checked="moduleConfig.saveToCloud(moduleConfigName, state)">
          <span :style="{ color: textColor }">
            {{ $t('deskModule.searchBox.searchBookmarks')  }}
          </span>
        </NCheckbox>
        <div
          class="flex-shrink-0 flex items-center justify-center w-8 h-8 cursor-pointer hover:bg-white/10 rounded transition-all"
          @click="openSearchEngineDialog"
          :title="$t('deskModule.searchBox.manageSearchEngines')"
        >
          <SvgIcon icon="set" :style="{ width: '20px', height: '20px', color: textColor }" />
        </div>
      </div>
    </div>
  </div>

  <!-- 搜索引擎管理对话框 -->
  <div v-if="searchEngineDialogVisible" class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-[10000]" @click.self="closeSearchEngineDialog">
    <div class="bg-white dark:bg-gray-800 rounded-xl p-6 w-[600px] max-h-[80vh] overflow-y-auto" @click.stop>
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-xl font-bold text-gray-800 dark:text-white">{{ $t('deskModule.searchBox.manageSearchEngines') }}</h3>
        <div class="cursor-pointer text-gray-500 hover:text-gray-700 dark:hover:text-gray-300" @click="closeSearchEngineDialog">
          <SvgIcon icon="line-md:close-small" style="width: 24px; height: 24px;" />
        </div>
      </div>

      <!-- 搜索引擎列表 -->
      <div class="mb-6">
        <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">{{ $t('deskModule.searchBox.searchEngineList') || '搜索引擎列表' }}</h4>
        <div class="space-y-2">
          <div
            v-for="(engine, index) in defaultSearchEngineList"
            :key="index"
            :draggable="true"
            @dragstart="handleDragStart(index)"
            @dragend="handleDragEnd"
            @dragover="handleDragOver($event, index)"
            class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700 rounded-lg cursor-move hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors"
            :class="{ 'opacity-50': draggedEngineIndex === index }"
          >
            <div class="flex items-center space-x-3 flex-1">
              <SvgIcon icon="ri-drag-drop-line" class="text-gray-400" style="width: 20px; height: 20px;" />
              <div class="w-8 h-8 flex items-center justify-center bg-white dark:bg-gray-800 rounded">
                <img v-if="engine.iconSrc" :src="engine.iconSrc" class="w-6 h-6" alt="" />
                <SvgIcon v-else icon="ion-language" class="text-gray-400" style="width: 20px; height: 20px;" />
              </div>
              <div class="flex-1">
                <div class="text-sm font-medium text-gray-800 dark:text-white">{{ engine.title }}</div>
                <div class="text-xs text-gray-500 dark:text-gray-400 truncate">{{ engine.url }}</div>
              </div>
            </div>
            <div class="flex items-center space-x-2">
              <div
                class="cursor-pointer text-blue-500 hover:text-blue-600"
                @click="startEditSearchEngine(engine, index)"
                :title="$t('common.edit') || '编辑'"
              >
                <SvgIcon icon="basil-edit-solid" style="width: 20px; height: 20px;" />
              </div>
              <div
                class="cursor-pointer text-red-500 hover:text-red-600"
                @click="deleteSearchEngine(index)"
                :title="$t('common.delete') || '删除'"
              >
                <SvgIcon icon="material-symbols-delete" style="width: 20px; height: 20px;" />
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 添加/编辑表单 -->
      <div class="border-t border-gray-200 dark:border-gray-700 pt-4">
        <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
          {{ editingSearchEngineIndex >= 0 ? ($t('common.edit') || '编辑') : ($t('common.add') || '添加') }}
          {{ $t('deskModule.searchBox.searchEngine') || '搜索引擎' }}
        </h4>

        <div class="space-y-4">
          <!-- 图标上传 -->
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              {{ $t('deskModule.searchBox.icon') || '图标' }}
            </label>
            <div class="flex items-center space-x-3">
              <div class="w-12 h-12 flex items-center justify-center bg-gray-100 dark:bg-gray-700 rounded border-2 border-dashed border-gray-300 dark:border-gray-600">
                <img v-if="searchEngineForm.iconSrc" :src="searchEngineForm.iconSrc" class="w-10 h-10 object-contain" alt="" />
                <SvgIcon v-else icon="typcn-plus" class="text-gray-400" style="width: 24px; height: 24px;" />
              </div>
              <input
                type="file"
                accept="image/*"
                @change="handleIconUpload"
                class="hidden"
                id="iconUpload"
              />
              <label
                for="iconUpload"
                class="px-4 py-2 bg-blue-500 text-white rounded-lg cursor-pointer hover:bg-blue-600 transition-colors text-sm"
              >
                {{ $t('common.upload') || '上传' }}
              </label>
              <div class="text-xs text-gray-500 dark:text-gray-400">
                {{ $t('deskModule.searchBox.iconTip') || '支持 PNG, JPG, SVG 格式' }}
              </div>
            </div>
          </div>

          <!-- 标题 -->
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              {{ $t('deskModule.searchBox.title') || '标题' }}
            </label>
            <input
              v-model="searchEngineForm.title"
              type="text"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-800 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none"
              :placeholder="$t('deskModule.searchBox.titlePlaceholder') || '例如: Google'"
            />
          </div>

          <!-- URL -->
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              {{ $t('deskModule.searchBox.url') || 'URL' }}
            </label>
            <input
              v-model="searchEngineForm.url"
              type="text"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-800 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none"
              :placeholder="$t('deskModule.searchBox.urlPlaceholder') || '例如: https://www.google.com/search?q=%s'"
            />
            <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
              {{ $t('deskModule.searchBox.urlTip') || '使用 %s 作为搜索关键词的占位符' }}
            </div>
          </div>

          <!-- 按钮 -->
          <div class="flex justify-end space-x-3">
            <button
              v-if="editingSearchEngineIndex >= 0"
              @click="resetSearchEngineForm"
              class="px-4 py-2 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
            >
              {{ $t('common.cancel') || '取消' }}
            </button>
            <button
              @click="saveSearchEngine"
              class="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="!searchEngineForm.title || !searchEngineForm.url"
            >
              {{ editingSearchEngineIndex >= 0 ? ($t('common.save') || '保存') : ($t('common.add') || '添加') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.search-container {
  border: 1px solid #ccc;
  transition: box-shadow 0.5s,backdrop-filter 0.5s;
  padding: 2px 10px;
  backdrop-filter:blur(2px)
}

.focused, .search-container:hover {
  box-shadow: 0px 0px 30px -5px rgba(41, 41, 41, 0.45);
  -webkit-box-shadow: 0px 0px 30px -5px rgba(0, 0, 0, 0.45);
  -moz-box-shadow: 0px 0px 30px -5px rgba(0, 0, 0, 0.45);
  backdrop-filter:blur(5px)
}

.before {
  left: 10px;
}

.after {
  right: 10px;
}

input {
  background-color: transparent;
  box-sizing: border-box;
  width: 100%;
  height: 40px;
  padding: 10px 5px;
  border: none;
  outline: none;
  font-size: 17px;
}

.suggestions-dropdown {
  max-height: 200px;
  overflow-y: auto;
}

.loading-spinner {
  width: 12px;
  height: 12px;
  border: 2px solid transparent;
  border-top: 2px solid currentColor;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

/* 选中项高亮样式 */
.suggestion-item.active {
  background-color: rgba(255, 255, 255, 0.2) !important;
}
</style>
