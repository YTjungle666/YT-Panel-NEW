<script setup lang="ts">
import { NTreeSelect } from 'naive-ui'

defineProps<{
  show: boolean
  isCreateMode: boolean
  bookmarkType: 'bookmark' | 'folder'
  currentBookmark: any
  currentEditBookmark: {
    title: string
    url: string
    folderId?: string | number
  }
  folderTreeOptions: any[]
  onClose: () => void
  onSave: () => void
}>()
</script>

<template>
  <div v-if="show" class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-800 p-6 rounded-lg w-96">
      <h3 class="text-xl font-bold text-gray-800 dark:text-white mb-4">
        {{ isCreateMode ? (bookmarkType === 'bookmark' ? $t('bookmarkManager.createBookmark') : $t('bookmarkManager.createFolder')) : $t('bookmarkManager.editBookmark') }}
      </h3>
      <div class="mb-4">
        <label class="block mb-2 text-gray-800 dark:text-white">{{ $t('bookmarkManager.title') }}</label>
        <input
          v-model="currentEditBookmark.title"
          class="w-full px-3 py-2 border border-gray-300 rounded-md bg-white dark:bg-gray-700 text-gray-800 dark:text-white"
          :placeholder="$t('bookmarkManager.title')"
        >
      </div>
      <div v-if="(!isCreateMode && !currentBookmark?.isFolder) || (isCreateMode && bookmarkType === 'bookmark')" class="mb-4">
        <label class="block mb-2 text-gray-800 dark:text-white">{{ $t('bookmarkManager.url') }}</label>
        <input
          v-model="currentEditBookmark.url"
          class="w-full px-3 py-2 border border-gray-300 rounded-md bg-white dark:bg-gray-700 text-gray-800 dark:text-white"
          :placeholder="$t('bookmarkManager.enterUrl')"
        >
      </div>
      <div class="mb-4">
        <label class="block mb-2 text-gray-800 dark:text-white">{{ $t('bookmarkManager.folder') }}</label>
        <NTreeSelect
          v-model:value="currentEditBookmark.folderId"
          :options="folderTreeOptions"
          key-field="key"
          label-field="label"
          :placeholder="$t('bookmarkManager.folder')"
          default-expand-all
          class="w-full"
        />
      </div>
      <div class="flex justify-end gap-2">
        <button class="px-4 py-2 border border-gray-300 rounded-md text-gray-800 dark:text-white" @click="onClose">
          {{ $t('bookmarkManager.cancel') }}
        </button>
        <button class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors shadow-md" @click="onSave">
          {{ $t('bookmarkManager.confirm') }}
        </button>
      </div>
    </div>
  </div>
</template>
