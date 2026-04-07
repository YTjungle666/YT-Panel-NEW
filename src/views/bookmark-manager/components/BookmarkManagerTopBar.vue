<script setup lang="ts">
defineProps<{
  isMobile: boolean
  isDropdownOpen: boolean
}>()

const emit = defineEmits<{
  (e: 'toggle-panel'): void
  (e: 'go-home'): void
  (e: 'update:isDropdownOpen', value: boolean): void
  (e: 'create-bookmark'): void
  (e: 'create-folder'): void
  (e: 'import'): void
  (e: 'export'): void
}>()
</script>

<template>
  <div class="px-4 py-2.5 border-b flex items-center justify-between bg-gray-50 dark:bg-gray-800 border-gray-200 dark:border-gray-700 relative">
    <div
      v-if="isMobile"
      class="flex items-center justify-center w-10 h-10 rounded-full bg-transparent text-gray-700 dark:text-white cursor-pointer transition-all hover:bg-gray-100 dark:hover:bg-gray-700"
      @click="emit('toggle-panel')"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
      </svg>
    </div>

    <div
      class="flex items-center justify-center w-10 h-10 rounded-full bg-transparent text-gray-700 dark:text-white cursor-pointer transition-all hover:bg-gray-100 dark:hover:bg-gray-700"
      @click="emit('go-home')"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
      </svg>
    </div>

    <h1 class="text-xl font-bold text-gray-800 dark:text-white flex-1 text-center">{{ $t('bookmarkManager.management') }}</h1>

    <div class="relative custom-dropdown">
      <div
        class="flex items-center justify-center w-10 h-10 rounded-full bg-transparent text-gray-700 dark:text-white cursor-pointer transition-all hover:bg-gray-100 dark:hover:bg-gray-700 mr-2"
        @click="emit('update:isDropdownOpen', !isDropdownOpen)"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
        </svg>
      </div>
      <div
        v-if="isDropdownOpen"
        class="absolute right-0 mt-2 w-48 bg-white dark:bg-gray-800 text-gray-700 dark:text-white rounded-md shadow-lg py-1 z-[100000]"
      >
        <button
          class="block px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 w-full text-left"
          @click.stop="emit('create-bookmark'); emit('update:isDropdownOpen', false)"
        >
          {{ $t('bookmarkManager.addBookmark') }}
        </button>
        <button
          class="block px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 w-full text-left"
          @click.stop="emit('create-folder'); emit('update:isDropdownOpen', false)"
        >
          {{ $t('bookmarkManager.addFolder') }}
        </button>
        <div class="border-t border-gray-200 dark:border-gray-700 my-1" />
        <button
          class="block px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 w-full text-left"
          @click.stop="emit('import'); emit('update:isDropdownOpen', false)"
        >
          {{ $t('bookmarkManager.importBookmarks') }}
        </button>
        <button
          class="block px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 w-full text-left"
          @click.stop="emit('export'); emit('update:isDropdownOpen', false)"
        >
          {{ $t('bookmarkManager.exportBookmarks') }}
        </button>
      </div>
    </div>
  </div>
</template>
