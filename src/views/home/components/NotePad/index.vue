<script lang="ts">
import { defineComponent } from 'vue'
import { SvgIcon, SvgIconOnline } from '@/components/common'
import type { NotePadPanelEmit } from './useNotePadPanel'
import { useNotePadPanel } from './useNotePadPanel'

export default defineComponent({
  name: 'HomeNotePad',
  components: {
    SvgIcon,
    SvgIconOnline,
  },
  props: {
    visible: {
      type: Boolean,
      required: true,
    },
  },
  emits: ['update:visible'],
  setup(props, { emit, expose }) {
    const panel = useNotePadPanel(props, emit as NotePadPanelEmit)
    expose({ refreshData: panel.refreshData })
    return panel
  },
})
</script>

<template>
  <!-- 遮罩层，点击关闭 -->
  <div v-if="authStore.visitMode === VisitMode.VISIT_MODE_LOGIN" v-show="visible" class="fixed inset-0 z-[100] bg-transparent" @click="close"></div>

  <!-- 便签主体 -->
  <transition name="note-fade">
    <div
        v-if="authStore.visitMode === VisitMode.VISIT_MODE_LOGIN"
        v-show="visible"
        ref="notepadRef"
        class="fixed z-[101] w-[350px] h-[45vh] flex flex-col shadow-xl rounded-lg overflow-hidden border border-amber-200"
        :style="{ left: clampedX + 'px', top: clampedY + 'px' }"
        @click.stop
    >
      <!-- 头部 -->
      <div ref="headerRef" class="h-8 bg-[#fef3c7] flex justify-between items-center px-2 border-b border-[#feebc8] select-none cursor-move shrink-0">
         <div class="flex items-center text-amber-800 text-sm font-bold cursor-pointer hover:bg-amber-200 rounded px-1 -ml-1 transition-colors" @click="showList = !showList">
            <SvgIcon icon="note" class="mr-1" />
            <span class="truncate max-w-[120px]" :title="currentNote.title">
                {{ t('notepad.title') }} <span v-if="currentNote.title && currentNote.title !== t('notepad.title')">- {{ currentNote.title }}</span>
            </span>
            <SvgIconOnline icon="mdi:chevron-down" class="ml-1 text-xs opacity-60" />
         </div>
         
         <div class="flex items-center gap-1">
             <!-- New Note Button -->
             <div class="hover:bg-amber-200 rounded p-0.5 cursor-pointer text-amber-900" title="New Note" @click="createNew">
                <SvgIconOnline icon="mdi:plus" />
             </div>
             <!-- Close Button -->
             <div class="hover:bg-amber-200 rounded p-0.5 cursor-pointer text-amber-900" @click="close">
                <SvgIconOnline icon="mdi:close" />
             </div>
         </div>
      </div>

      <!-- 编辑区 & 列表区 -->
      <div class="flex-1 bg-[#fffbeb] relative overflow-hidden flex flex-col">
         
         <!-- 内容区域容器 -->
         <div class="flex-1 relative overflow-hidden">
             <!-- 列表侧边栏 -->
             <transition name="slide-fade">
                <div v-show="showList" class="absolute inset-0 z-10 bg-[#fffbeb]/95 backdrop-blur-sm border-r border-[#feebc8] flex flex-col">
                    <div class="p-2 space-y-1 overflow-y-auto flex-1">
                        <div v-if="noteList.length === 0" class="text-center text-gray-400 text-xs py-4">
                            {{ t('notepad.noData') }}
                        </div>
                        <div 
                            v-for="item in noteList" 
                            :key="item.id"
                            class="group flex justify-between items-center p-2 rounded text-sm text-gray-700 hover:bg-amber-100 cursor-pointer transition-colors"
                            :class="{'bg-amber-200 font-medium text-amber-900': item.id === currentNote.id}"
                            @click="selectNote(item)"
                        >
                            <span class="truncate flex-1">{{ item.title || '便签' }}</span>
                            <div class="opacity-0 group-hover:opacity-100 p-1 hover:bg-red-100 text-red-500 rounded transition-all" @click.stop="deleteNote(item)">
                                <SvgIconOnline icon="mdi:trash-can-outline" />
                            </div>
                        </div>
                    </div>
                </div>
             </transition>

             <!-- ContentEditable Div -->
             <div
                ref="editorRef"
                contenteditable="true"
                class="w-full h-full p-3 outline-none overflow-y-auto text-sm text-gray-800 break-words font-sans leading-relaxed"
                :data-placeholder="t('notepad.placeholder')"
                @input="handleInput"
                @click="handleEditorClick"
                spellcheck="false"
             ></div>
         </div>

         <!-- 底部工具栏 -->
         <div class="h-9 bg-[#fef3c7] border-t border-amber-200 flex items-center px-2 gap-1 overflow-x-auto text-amber-800 shrink-0">
             <div class="p-1 hover:bg-amber-200 rounded cursor-pointer transition-colors" @mousedown.prevent="execCommand('bold')" title="粗体">
                <SvgIconOnline icon="mdi:format-bold" />
             </div>
             <div class="p-1 hover:bg-amber-200 rounded cursor-pointer transition-colors" @mousedown.prevent="execCommand('italic')" title="斜体">
                <SvgIconOnline icon="mdi:format-italic" />
             </div>
             <div class="p-1 hover:bg-amber-200 rounded cursor-pointer transition-colors" @mousedown.prevent="execCommand('strikeThrough')" title="删除线">
                <SvgIconOnline icon="mdi:format-strikethrough-variant" />
             </div>
             <div class="p-1 hover:bg-amber-200 rounded cursor-pointer transition-colors" @mousedown.prevent="execCommand('insertHorizontalRule')" title="分割线">
                <SvgIconOnline icon="mdi:minus" />
             </div>
             <div class="w-[1px] h-4 bg-amber-300 mx-1"></div>
             <div class="p-1 hover:bg-amber-200 rounded cursor-pointer transition-colors" @mousedown.prevent="execCommand('formatBlock', 'H1')" title="标题1">
                <SvgIconOnline icon="mdi:format-header-1" />
             </div>
             <div class="p-1 hover:bg-amber-200 rounded cursor-pointer transition-colors" @mousedown.prevent="execCommand('formatBlock', 'H2')" title="标题2">
                <SvgIconOnline icon="mdi:format-header-2" />
             </div>
             <div class="w-[1px] h-4 bg-amber-300 mx-1"></div>
             <div class="p-1 hover:bg-amber-200 rounded cursor-pointer transition-colors" @mousedown.prevent="execCommand('insertUnorderedList')" title="无序列表">
                <SvgIconOnline icon="mdi:format-list-bulleted" />
             </div>
             <div class="p-1 hover:bg-amber-200 rounded cursor-pointer transition-colors" @mousedown.prevent="execCommand('insertOrderedList')" title="有序列表">
                <SvgIconOnline icon="mdi:format-list-numbered" />
             </div>
             <div class="p-1 hover:bg-amber-200 rounded cursor-pointer transition-colors" @mousedown.prevent="execCommand('formatBlock', 'PRE')" title="代码块">
                <SvgIconOnline icon="mdi:code-tags" />
             </div>
             <div class="w-[1px] h-4 bg-amber-300 mx-1"></div>
             <div class="p-1 hover:bg-amber-200 rounded cursor-pointer transition-colors" @mousedown.prevent="execCommand('removeFormat')" title="清除格式">
                <SvgIconOnline icon="mdi:format-clear" />
             </div>
         </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.note-fade-enter-active,
.note-fade-leave-active {
  transition: all 0.2s ease;
}

.note-fade-enter-from,
.note-fade-leave-to {
  opacity: 0;
  transform: translateY(-10px) scale(0.95);
}

.slide-fade-enter-active,
.slide-fade-leave-active {
  transition: all 0.2s ease;
}
.slide-fade-enter-from,
.slide-fade-leave-to {
  opacity: 0;
  transform: translateX(-10px);
}

:deep(.file-attachment) {
    display: inline-flex;
    align-items: center;
    background-color: #fff7ed;
    border: 1px solid #fed7aa;
    border-radius: 4px;
    padding: 0 4px;
    margin: 0 2px;
    font-size: 0.85em;
    color: #c2410c;
    cursor: pointer;
    user-select: none;
    transition: all 0.2s;
    text-decoration: none;
}

:deep(.file-attachment:hover) {
    background-color: #ffedd5;
    border-color: #fdba74;
}

:deep(.note-image) {
    max-width: 100%;
    max-height: 150px;
    border-radius: 4px;
    margin: 4px 0;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    display: block;
    cursor: default;
}

/* 编辑器内部样式 */
:deep(h1) {
    font-size: 1.5em;
    font-weight: bold;
    margin: 0.5em 0 0.25em 0;
    line-height: 1.3;
    color: #1f2937;
}
:deep(h2) {
    font-size: 1.25em;
    font-weight: bold;
    margin: 0.5em 0 0.25em 0;
    line-height: 1.4;
    color: #374151;
    border-bottom: 1px solid #e5e7eb;
}
:deep(ul) {
    list-style-type: disc;
    padding-left: 1.5em;
    margin: 0.5em 0;
}
:deep(ol) {
    list-style-type: decimal;
    padding-left: 1.5em;
    margin: 0.5em 0;
}
:deep(li) {
    margin: 0.2em 0;
}
:deep(pre) {
    background-color: #1e293b; /* slate-800 */
    color: #e2e8f0; /* slate-200 */
    padding: 0.75em;
    border-radius: 6px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    font-size: 0.9em;
    line-height: 1.5;
    margin: 0.5em 0;
    white-space: pre-wrap;
    overflow-x: auto;
}
:deep(blockquote) {
    border-left: 4px solid #cbd5e1;
    padding-left: 1em;
    margin: 0.5em 0;
    color: #64748b;
    font-style: italic;
}
:deep(b), :deep(strong) {
    font-weight: bold;
}
:deep(i), :deep(em) {
    font-style: italic;
}
:deep(s), :deep(strike) {
    text-decoration: line-through;
}
:deep(hr) {
    border: 0;
    border-top: 1px solid #78350f; /* Amber-900 like */
    opacity: 0.2;
    margin: 1em 0;
}

/* 占位符效果 */
div[contenteditable]:empty::before {
  content: attr(data-placeholder);
  color: #9ca3af;
  pointer-events: none;
  font-style: italic;
}
</style>
