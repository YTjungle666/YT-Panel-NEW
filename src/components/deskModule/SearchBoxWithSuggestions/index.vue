<script lang="ts">
import { defineComponent, type PropType } from 'vue'
import { NAvatar, NCheckbox } from 'naive-ui'
import { SvgIcon } from '@/components/common'
import { useSearchBoxWithSuggestions } from './useSearchBoxWithSuggestions'

export default defineComponent({
  name: 'SearchBoxWithSuggestions',
  components: {
    NAvatar,
    NCheckbox,
    SvgIcon,
  },
  props: {
    background: {
      type: String,
      default: '#2a2a2a6b',
    },
    textColor: {
      type: String,
      default: 'white',
    },
    searchItems: {
      type: Array as PropType<Panel.ItemInfo[]>,
      default: () => [],
    },
  },
  emits: ['itemSearch', 'itemSelect'],
  setup(props, { emit }) {
    return useSearchBoxWithSuggestions(props, emit as any)
  },
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
