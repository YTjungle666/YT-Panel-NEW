import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useDialog, useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { useDebounceFn, useDraggable, useStorage } from '@vueuse/core'
import {
  deleteNotepad,
  getNotepadList,
  saveNotepadContent,
  type NotepadInfo,
} from '@/api/panel/notepad'
import { VisitMode } from '@/enums/auth'
import { useAuthStore } from '@/store/modules/auth'
import { logError } from '@/utils/logger'
import { sanitizeNotepadHtml } from '@/utils/sanitize'

export interface NotePadPanelProps {
  visible: boolean
}

export type NotePadPanelEmit = (e: 'update:visible', visible: boolean) => void

export function useNotePadPanel(props: NotePadPanelProps, emit: NotePadPanelEmit) {
  const { t } = useI18n()
  const message = useMessage()
  const dialog = useDialog()
  const authStore = useAuthStore()
  const editorRef = ref<HTMLDivElement | null>(null)
  const notepadRef = ref<HTMLElement | null>(null)
  const headerRef = ref<HTMLElement | null>(null)
  const currentNote = useStorage<Partial<NotepadInfo>>('yt-panel-notepad-current', { id: 0, title: '', content: '' })
  const noteList = useStorage<NotepadInfo[]>('yt-panel-notepad-list', [])
  const showList = ref(false)
  const noteWidth = 350
  const winW = ref(window.innerWidth)
  const winH = ref(window.innerHeight)

  const onResize = () => {
    winW.value = window.innerWidth
    winH.value = window.innerHeight
  }

  const { x, y } = useDraggable(notepadRef, {
    initialValue: { x: Math.max(0, window.innerWidth - noteWidth - 20), y: 80 },
    handle: headerRef,
  })

  const clampedX = computed(() => Math.max(0, Math.min(x.value, winW.value - noteWidth)))
  const clampedY = computed(() => Math.max(0, Math.min(y.value, winH.value - 100)))

  const loadList = async () => {
    if (!authStore.userInfo?.id || authStore.visitMode !== VisitMode.VISIT_MODE_LOGIN)
      return

    try {
      const res = await getNotepadList()
      if (res.code === 0)
        noteList.value = res.data || []
    } catch (error) {
      logError('Load list error', error)
    }
  }

  function getNextDefaultNoteTitle() {
    const existingTitles = new Set(
      (noteList.value || [])
        .map(note => (note.title || '').trim())
        .filter(Boolean),
    )

    let nextIndex = (noteList.value?.length || 0) + 1
    while (existingTitles.has(`便签${nextIndex}`))
      nextIndex += 1

    return `便签${nextIndex}`
  }

  const generateTitle = (textContent?: string) => {
    if (editorRef.value) {
      const h1 = editorRef.value.querySelector('h1')
      if (h1 && h1.innerText.trim())
        return h1.innerText.trim()
    }

    const text = textContent !== undefined ? textContent : (editorRef.value?.innerText.trim() || '')
    if (text)
      return text.substring(0, 5)

    if (currentNote.value.title?.trim())
      return currentNote.value.title.trim()

    if (currentNote.value.id)
      return `便签${currentNote.value.id}`

    return getNextDefaultNoteTitle()
  }

  const handleSave = async () => {
    if (!editorRef.value)
      return

    try {
      const content = sanitizeNotepadHtml(editorRef.value.innerHTML)
      const text = editorRef.value.innerText.trim()
      const title = generateTitle(text)
      const saveId = currentNote.value.id || 0

      const res = await saveNotepadContent({
        id: saveId,
        title,
        content,
      })

      if (res.code === 0) {
        if (currentNote.value.id === saveId)
          currentNote.value = res.data

        await loadList()
      }
    } catch (error) {
      logError('Save notepad error', error)
    }
  }

  const saveContent = useDebounceFn(handleSave, 1000)

  const handleInput = () => {
    if (!editorRef.value)
      return

    currentNote.value.title = generateTitle(editorRef.value.innerText.trim())
    saveContent()
  }

  const selectNote = (note: NotepadInfo) => {
    currentNote.value = { ...note }
    if (editorRef.value)
      editorRef.value.innerHTML = sanitizeNotepadHtml(note.content || '')

    showList.value = false
  }

  const createNew = () => {
    currentNote.value = { id: 0, title: getNextDefaultNoteTitle(), content: '' }
    if (editorRef.value) {
      editorRef.value.innerHTML = ''
      editorRef.value.focus()
    }
    showList.value = false
  }

  const refreshData = async () => {
    await loadList()
  }

  const deleteNote = async (note: NotepadInfo) => {
    dialog.warning({
      title: t('common.warning'),
      content: t('common.deleteConfirmByName', { name: note.title || '便签' }),
      positiveText: t('common.confirm'),
      negativeText: t('common.cancel'),
      onPositiveClick: async () => {
        try {
          const res = await deleteNotepad({ id: note.id })
          if (res.code === 0) {
            message.success(t('common.deleteSuccess'))
            await loadList()
            if (currentNote.value.id === note.id) {
              if (noteList.value.length > 0)
                selectNote(noteList.value[0])
              else
                createNew()
            }
          }
        } catch {
          message.error(t('common.deleteFail'))
        }
      },
    })
  }

  const downloadFile = async (url: string, filename: string) => {
    try {
      const response = await fetch(url)
      if (!response.ok)
        throw new Error('Network response was not ok')

      const blob = await response.blob()
      const urlCreator = window.URL || window.webkitURL
      const objectUrl = urlCreator.createObjectURL(blob)

      const link = document.createElement('a')
      link.href = objectUrl
      link.download = filename
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)

      setTimeout(() => {
        urlCreator.revokeObjectURL(objectUrl)
      }, 100)
    } catch {
      message.error(t('notepad.saveFailed'))
    }
  }

  const handleEditorClick = (event: MouseEvent) => {
    const target = event.target as HTMLElement | null
    const link = target?.closest('.file-attachment') as HTMLAnchorElement | null
    if (!link)
      return

    event.preventDefault()
    const url = link.getAttribute('href')
    const filename = link.getAttribute('data-filename')

    if (url && filename)
      void downloadFile(url, filename)
  }

  const execCommand = (command: string, value?: string) => {
    if (command === 'removeFormat') {
      document.execCommand('removeFormat', false, value)
      document.execCommand('formatBlock', false, 'div')
    } else {
      document.execCommand(command, false, value)

      if (command === 'formatBlock' && value === 'PRE') {
        const selection = window.getSelection()
        if (selection && selection.rangeCount > 0) {
          const range = selection.getRangeAt(0)
          let node = range.commonAncestorContainer
          if (node.nodeType === Node.TEXT_NODE && node.parentNode)
            node = node.parentNode

          let el = node as HTMLElement
          while (el && el.tagName !== 'PRE' && el !== editorRef.value) {
            if (!el.parentElement)
              break
            el = el.parentElement
          }

          if (el && el.tagName === 'PRE' && !el.nextElementSibling) {
            const div = document.createElement('div')
            div.appendChild(document.createElement('br'))
            el.parentNode?.insertBefore(div, el.nextSibling)
          }
        }
      }
    }

    handleInput()
  }

  const initData = async () => {
    await loadList()

    if (noteList.value.length === 0) {
      if (currentNote.value.id !== 0)
        createNew()
      return
    }

    if (currentNote.value.id === 0 && !currentNote.value.content && !currentNote.value.title) {
      selectNote(noteList.value[0])
      return
    }

    const exist = noteList.value.find(note => note.id === currentNote.value.id)
    if (exist) {
      currentNote.value = { ...exist }
      if (editorRef.value)
        editorRef.value.innerHTML = sanitizeNotepadHtml(exist.content || '')
    } else {
      selectNote(noteList.value[0])
    }
  }

  const close = () => {
    emit('update:visible', false)
  }

  onMounted(async () => {
    window.addEventListener('resize', onResize)
    if (noteList.value.length === 0)
      await loadList()
  })

  onUnmounted(() => {
    window.removeEventListener('resize', onResize)
  })

  watch(() => props.visible, (visible) => {
    if (visible) {
      x.value = Math.max(0, Math.min(x.value, winW.value - noteWidth))
      y.value = Math.max(0, Math.min(y.value, winH.value - 100))
      void initData()
    } else {
      void handleSave()
    }
  })

  return {
    VisitMode,
    authStore,
    clampedX,
    clampedY,
    close,
    createNew,
    currentNote,
    deleteNote,
    editorRef,
    execCommand,
    handleEditorClick,
    handleInput,
    headerRef,
    noteList,
    notepadRef,
    refreshData,
    showList,
    selectNote,
    t,
  }
}
