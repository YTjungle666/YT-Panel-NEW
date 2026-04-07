<script setup lang="ts">
import { computed } from 'vue'
import { NInput } from 'naive-ui'

const props = defineProps<{
  currentPath: Array<{ id: string, name: string }>
  searchQuery: string
  filteredBookmarks: any[]
  dragIndicatorTop: number | null
  dragOverTarget: string | number | null
  dragInsertPosition: 'before' | 'after' | null
  selectedBookmarkId: string
  selectedFolder: string
  focusedItemId: string
  isMobile: boolean
  getBookmarkDisplayIcon: (iconJson?: string | null) => string
  handleBookmarkIconError: (event: Event) => void
  handleSearch: () => void
  handleBreadcrumbClick: (id: string) => void
  openContextMenu: (event: MouseEvent, item: any) => void
  openFolder: (id: string | number) => void
  openBookmark: (bookmark: any) => void
  handleDragStart: (event: DragEvent, item: any) => void
  handleDragEnd: (event: DragEvent) => void
  handleDragOver: (event: DragEvent, item?: any) => void
  handleDragLeave: (event: DragEvent) => void
  handleDrop: (event: DragEvent, item: any) => void
  handleContainerDragOver: (event: DragEvent) => void
  resetDragState: () => void
  setFocusedItemId: (id: string) => void
}>()

const searchValue = computed({
  get: () => props.searchQuery,
  set: (value: string) => {
    emit('update:searchQuery', value)
  },
})

const emit = defineEmits<{
  (e: 'update:searchQuery', value: string): void
}>()
</script>

<template>
  <div class="flex-1 flex flex-col overflow-hidden">
    <div class="sticky top-0 z-10 p-2 border-b flex flex-col bg-white dark:bg-gray-800">
      <div class="flex items-center text-sm mb-2 text-gray-600 dark:text-gray-400">
        <span
          v-for="(crumb, index) in props.currentPath"
          :key="index"
          class="cursor-pointer hover:text-blue-600"
          @click="props.handleBreadcrumbClick(crumb.id)"
        >
          {{ crumb.name }}
          <span v-if="index < props.currentPath.length - 1" class="mx-1">/</span>
        </span>
      </div>
      <div class="flex-1 rounded-full overflow-hidden border-2 border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-800">
        <NInput
          v-model:value="searchValue"
          :placeholder="$t('bookmarkManager.searchPlaceholder')"
          clearable
          class="w-full bg-transparent text-gray-800 dark:text-white bookmark-search-input"
          @input="props.handleSearch"
        />
      </div>
    </div>

    <div class="flex-1 relative overflow-auto bg-white dark:bg-gray-800" @dragover.prevent="props.handleContainerDragOver">
      <div v-if="props.filteredBookmarks.length === 0" class="text-center py-8 text-gray-400 dark:text-gray-500">
        {{ $t('bookmarkManager.noData') }}
      </div>

      <div v-else class="py-2" @dragover.prevent="props.handleContainerDragOver">
        <div
          v-if="props.dragIndicatorTop !== null"
          class="absolute left-4 right-4 z-20 flex items-center pointer-events-none transition-all duration-75"
          :style="{ top: `${props.dragIndicatorTop}px`, transform: 'translateY(-50%)' }"
        >
          <div class="w-full h-[2px] bg-blue-500" />
        </div>

        <template v-for="item in props.filteredBookmarks" :key="item.id">
          <div
            class="relative py-[2px]"
            :draggable="true"
            @dragstart="props.handleDragStart($event, item)"
            @dragend="props.handleDragEnd($event); props.resetDragState()"
            @dragover="props.handleDragOver($event, item)"
            @dragleave="props.handleDragLeave($event)"
            @drop="props.handleDrop($event, item); props.resetDragState()"
          >
            <div
              :class="[
                'flex items-center px-4 py-2 cursor-pointer transition-colors group',
                props.dragOverTarget === item.id && item.isFolder && props.dragInsertPosition === null
                  ? 'bg-blue-50 dark:bg-blue-900'
                  : props.selectedBookmarkId === String(item.id) || (item.isFolder && props.selectedFolder === String(item.id))
                    ? 'bg-gray-100 dark:bg-gray-700'
                    : 'hover:bg-gray-50 dark:hover:bg-gray-700',
              ]"
              @contextmenu.prevent="!props.isMobile ? props.openContextMenu($event, item) : null"
              @click="props.setFocusedItemId(String(item.id))"
              @dblclick="item.isFolder ? props.openFolder(item.id) : props.openBookmark(item)"
            >
              <div class="flex-shrink-0 w-4 h-4 flex items-center justify-center mr-3">
                <span
                  v-if="item.isFolder"
                  class="text-gray-400 dark:text-gray-500 group-hover:text-[#4285F4] dark:group-hover:text-[#4285F4] transition-colors"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" width="24" height="24" fill="currentColor" viewBox="0 0 24 24">
                    <path d="M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm0 12H4V8h16v10z" />
                  </svg>
                </span>
                <img
                  v-else
                  :src="props.getBookmarkDisplayIcon(item.iconJson)"
                  alt=""
                  class="w-4 h-4 rounded"
                  @error="props.handleBookmarkIconError"
                >
              </div>

              <div class="flex-1 min-w-0">
                <div class="text-sm text-gray-900 dark:text-white truncate">
                  <span>{{ item.title }}</span>
                  <span
                    v-if="!item.isFolder && item.url && props.focusedItemId === String(item.id)"
                    class="text-gray-500 dark:text-gray-400 transition-opacity ml-2"
                  >
                    {{ item.url }}
                  </span>
                  <span
                    v-else-if="item.isFolder && props.focusedItemId === String(item.id)"
                    class="text-gray-400 dark:text-gray-500 italic transition-opacity ml-2"
                  >
                    {{ $t('bookmarkManager.folder') || '文件夹' }}
                  </span>
                </div>
              </div>

              <div
                class="flex-shrink-0 w-6 h-6 flex items-center justify-center ml-2 cursor-pointer text-gray-700 dark:text-white"
                @click.stop="props.openContextMenu($event, item)"
              >
                <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
                </svg>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
