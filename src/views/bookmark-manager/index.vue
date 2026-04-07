<template>
	<div
		class="flex flex-col h-screen bg-white dark:bg-gray-800"
		@contextmenu.prevent
	>
		<BookmarkManagerTopBar
			:is-mobile="isMobile"
			:is-dropdown-open="isDropdownOpen"
			@toggle-panel="togglePanel"
			@go-home="goBackToHome"
			@update:is-dropdown-open="isDropdownOpen = $event"
			@create-bookmark="bookmarkType = 'bookmark'; createNewBookmark()"
			@create-folder="bookmarkType = 'folder'; createNewBookmark()"
			@import="triggerImportBookmarks"
			@export="exportBookmarks"
		/>
		<input
			ref="fileInput"
			type="file"
			accept=".html"
			style="display: none;"
			@change="handleFileChange"
		>

		<div class="flex flex-1 overflow-hidden">
			<BookmarkManagerSidebar
				:is-mobile="isMobile"
				:show-left-panel="showLeftPanel"
				:is-panel-expanded="isPanelExpanded"
				:left-panel-width="leftPanelWidth"
				:bookmark-tree="bookmarkTree"
				:default-expanded-keys="defaultExpandedKeys"
				:selected-keys-ref="selectedKeysRef"
				:render-tree-label="renderTreeLabel"
				:render-expand-icon="renderExpandIcon"
				:on-collapse-panel="collapsePanel"
				:on-start-resize="startResize"
				:on-select="handleSelect"
				:on-expand="handleNodeExpand"
			/>

			<BookmarkManagerContent
				v-model:search-query="searchQuery"
				:current-path="currentPath"
				:filtered-bookmarks="filteredBookmarks"
				:drag-indicator-top="dragIndicatorTop"
				:drag-over-target="dragOverTarget"
				:drag-insert-position="dragInsertPosition"
				:selected-bookmark-id="selectedBookmarkId"
				:selected-folder="selectedFolder"
				:focused-item-id="focusedItemId"
				:is-mobile="isMobile"
				:get-bookmark-display-icon="getBookmarkDisplayIcon"
				:handle-bookmark-icon-error="handleBookmarkIconError"
				:handle-search="handleSearch"
				:handle-breadcrumb-click="handleBreadcrumbClick"
				:open-context-menu="openContextMenu"
				:open-folder="openFolder"
				:open-bookmark="openBookmark"
				:handle-drag-start="handleDragStart"
				:handle-drag-end="handleDragEnd"
				:handle-drag-over="handleDragOver"
				:handle-drag-leave="handleDragLeave"
				:handle-drop="handleDrop"
				:handle-container-drag-over="handleContainerDragOver"
				:reset-drag-state="resetDragState"
				:set-focused-item-id="setFocusedItemId"
			/>
		</div>

		<BookmarkManagerEditDialog
			:show="isEditDialogOpen"
			:is-create-mode="isCreateMode"
			:bookmark-type="bookmarkType"
			:current-bookmark="currentBookmark"
			:current-edit-bookmark="currentEditBookmark"
			:folder-tree-options="folderTreeOptions"
			:on-close="closeEditDialog"
			:on-save="saveBookmarkChanges"
		/>

		<BookmarkManagerContextMenu
			:show="isContextMenuOpen"
			:style="contextMenuStyle"
			:current-bookmark="currentBookmark"
			:on-edit="handleEditBookmark"
			:on-delete="handleDeleteBookmark"
		/>
	</div>
</template>

<script setup lang="ts">
import BookmarkManagerContent from './components/BookmarkManagerContent.vue'
import BookmarkManagerContextMenu from './components/BookmarkManagerContextMenu.vue'
import BookmarkManagerEditDialog from './components/BookmarkManagerEditDialog.vue'
import BookmarkManagerSidebar from './components/BookmarkManagerSidebar.vue'
import BookmarkManagerTopBar from './components/BookmarkManagerTopBar.vue'
import { useBookmarkManagerPage } from './useBookmarkManagerPage'

const {
	isMobile,
	isDropdownOpen,
	togglePanel,
	goBackToHome,
	bookmarkType,
	createNewBookmark,
	triggerImportBookmarks,
	exportBookmarks,
	fileInput,
	handleFileChange,
	showLeftPanel,
	isPanelExpanded,
	leftPanelWidth,
	bookmarkTree,
	defaultExpandedKeys,
	selectedKeysRef,
	renderTreeLabel,
	renderExpandIcon,
	collapsePanel,
	startResize,
	handleSelect,
	handleNodeExpand,
	searchQuery,
	currentPath,
	filteredBookmarks,
	dragIndicatorTop,
	dragOverTarget,
	dragInsertPosition,
	selectedBookmarkId,
	selectedFolder,
	focusedItemId,
	getBookmarkDisplayIcon,
	handleBookmarkIconError,
	handleSearch,
	handleBreadcrumbClick,
	openContextMenu,
	openFolder,
	openBookmark,
	handleDragStart,
	handleDragEnd,
	handleDragOver,
	handleDragLeave,
	handleDrop,
	handleContainerDragOver,
	resetDragState,
	setFocusedItemId,
	isEditDialogOpen,
	isCreateMode,
	currentBookmark,
	currentEditBookmark,
	folderTreeOptions,
	closeEditDialog,
	saveBookmarkChanges,
	isContextMenuOpen,
	contextMenuStyle,
	handleEditBookmark,
	handleDeleteBookmark,
} = useBookmarkManagerPage()
</script>

<style scoped>
.context-menu {
    position: fixed !important;
    z-index: 99999 !important;
    border-radius: 0.375rem !important;
    border-right: 1px solid #e5e7eb !important;
}

.dark .context-menu {
  border-right: 1px solid #4a5568 !important;
}


/* 阴影和圆角让左栏浮动更美观 */
.fixed.bg-white {
	border-right: 1px solid #e5e7eb; /* 柔和边框 */
	border-radius: 0 0.75rem 0.75rem 0; /* 左边圆角 */
	box-shadow: 0 4px 12px rgba(0,0,0,0.1);
	transition: width 0.3s ease-in-out, transform 0.3s ease-in-out;
}

    /* 移除输入组件根元素的边框 */
    .bookmark-search-input.n-input {
      border: none !important;
      box-shadow: none !important;
      outline: none !important;
    }
    /* 移除内部输入组件的所有边框、阴影和圆角 */
    .bookmark-search-input.n-input :deep(*) {
      border: none !important;
      outline: none !important;
      box-shadow: none !important;
      border-radius: 0 !important;
    }
    /* 确保正确的布局 */
    .bookmark-search-input.n-input :deep(.n-input__inner),
    .bookmark-search-input.n-input :deep(.n-input__input-wrap) {
      overflow: hidden !important;
      width: 100% !important;
      height: 100% !important;
    }

</style>
