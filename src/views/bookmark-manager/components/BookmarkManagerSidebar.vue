<script setup lang="ts">
import { NTree } from 'naive-ui'

defineProps<{
  isMobile: boolean
  showLeftPanel: boolean
  isPanelExpanded: boolean
  leftPanelWidth: number
  bookmarkTree: any[]
  defaultExpandedKeys: string[]
  selectedKeysRef: (string | number)[]
  renderTreeLabel: ((payload: { option: any }) => any) | undefined
  renderExpandIcon: ((payload: { option: any }) => any) | undefined
  onCollapsePanel: () => void
  onStartResize: (event: MouseEvent) => void
  onSelect: (keys: (string | number)[]) => void
  onExpand: (node: any) => void
}>()
</script>

<template>
  <div
    v-if="isMobile && showLeftPanel"
    class="fixed inset-0 bg-black bg-opacity-30 z-40"
    @click="onCollapsePanel"
  />

  <div
    v-show="showLeftPanel"
    :class="[
      isMobile ? 'fixed top-0 left-0 h-full bg-white dark:bg-gray-800 z-50 rounded-r-lg shadow-lg overflow-auto transition-all duration-300 ease-in-out' : 'h-full bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 overflow-auto',
      isMobile && isPanelExpanded ? 'w-3/4' : isMobile ? 'w-12' : '',
    ]"
    :style="{ width: !isMobile ? `${leftPanelWidth}px` : '' }"
  >
    <NTree
      :data="bookmarkTree"
      :default-expanded-keys="defaultExpandedKeys"
      :selected-keys="selectedKeysRef"
      block-line
      :render-label="renderTreeLabel"
      :render-expand-icon="renderExpandIcon"
      @update:selected-keys="onSelect"
      @expand="onExpand"
    />
  </div>

  <div
    v-if="!isMobile"
    class="w-1 bg-gray-200 cursor-col-resize hover:bg-blue-300 flex items-center justify-center"
    :style="{ height: '100%', userSelect: 'none' }"
    @mousedown="onStartResize"
  >
    <div class="w-px h-12 bg-gray-400" />
  </div>
</template>
