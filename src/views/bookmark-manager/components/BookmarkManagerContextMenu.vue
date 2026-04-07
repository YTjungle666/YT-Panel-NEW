<script setup lang="ts">
defineProps<{
  show: boolean
  style: Record<string, string>
  currentBookmark: any
  onEdit: () => void
  onDelete: () => void
}>()
</script>

<template>
  <div
    v-if="show"
    :style="style"
    class="fixed bg-white dark:bg-gray-800 text-gray-700 dark:text-white shadow-lg py-1 z-50 w-40 context-menu border border-gray-200 dark:border-gray-700"
    @contextmenu.prevent.stop
  >
    <div v-if="!currentBookmark?.isFolder" class="px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer" @click="onEdit">
      {{ $t('bookmarkManager.edit') }}
    </div>
    <div class="px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer" @click="onDelete">
      {{ $t('bookmarkManager.delete') }}
    </div>
  </div>
</template>
