<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import { VueDraggable } from 'vue-draggable-plus'
import { NBackTop, NButton, NDropdown } from 'naive-ui'
import { AppIcon } from './components'
import HomeActionButtons from './components/HomeActionButtons.vue'
import HomeBookmarkDrawer from './components/HomeBookmarkDrawer.vue'
import HomeFooterHtml from './components/HomeFooterHtml.vue'
import HomeWindowModal from './components/HomeWindowModal.vue'
import { useHomePage } from './useHomePage'
import { Clock } from '@/components/deskModule'
import { SvgIcon } from '@/components/common'
import { PanelPanelConfigStyleEnum, PanelStateNetworkModeEnum } from '@/enums'
import { VisitMode } from '@/enums/auth'
import { t } from '@/locales'

const SearchBoxWithSuggestions = defineAsyncComponent(() => import('@/components/deskModule/SearchBoxWithSuggestions/index.vue'))
const SystemMonitor = defineAsyncComponent(() => import('@/components/deskModule/SystemMonitor/index.vue'))
const AppStarter = defineAsyncComponent(() => import('./components/AppStarter/index.vue'))
const EditItem = defineAsyncComponent(() => import('./components/EditItem/index.vue'))
const NotePad = defineAsyncComponent(() => import('./components/NotePad/index.vue'))

const {
	authStore,
	currentAddItenIconGroupId,
	drawerVisible,
	dropdownMenuX,
	dropdownMenuY,
	dropdownShow,
	editItemInfoData,
	editItemInfoShow,
	filterItems,
	getDropdownMenuOptions,
	getScrollListenTarget,
	handWindowIframeIdLoad,
	handleAddItem,
	handleChangeNetwork,
	handleContextMenu,
	handleEditSuccess,
	handleRefreshData,
	handleItemClick,
	handleRightMenuSelect,
	handleSearchItemSelect,
	handleSaveSort,
	handleSetHoverStatus,
	handleSetSortStatus,
	isMobile,
	itemFrontEndSearch,
	navigateToBookmarkManager,
	notepadInstance,
	notepadVisible,
	onClickoutside,
	panelState,
	renderTreeLabel,
	safeFooterHtml,
	scrollContainerRef,
	searchableItems,
	settingModalShow,
	treeData,
	windowIframeIsLoad,
	windowShow,
	windowSrc,
	windowTitle,
	handleTreeSelect,
} = useHomePage()
</script>

<template>
	<div>
		<!-- 左上角抽屉按钮 - 大众常用样式 -->
		<div class="fixed top-4 left-4 z-50">
			<NButton
				circle
				color="#2a2a2a6b"
				class="w-10 h-10 !p-0 shadow-[0_0_10px_2px_rgba(0,0,0,0.2)] no-focus-outline no-tap-highlight"
				tabindex="-1"
				@click="drawerVisible = !drawerVisible"
			>
				<svg viewBox="0 0 24 24" class="w-6 h-6 text-white" v-if="drawerVisible">
					<path
						d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12
         5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"
						fill="currentColor"
					/>
				</svg>
				<svg viewBox="0 0 24 24" class="w-6 h-6 text-white" v-else>
					<path
						d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z"
						fill="currentColor"
					/>
				</svg>
			</NButton>
		</div>

		<!-- 右上角便签按钮 -->
		<div v-if="authStore.visitMode === VisitMode.VISIT_MODE_LOGIN" class="fixed top-4 right-4 z-50 cursor-pointer transition-opacity hover:opacity-80 no-tap-highlight" @click="notepadVisible = !notepadVisible">
			<SvgIcon icon="note" class="text-white text-[32px] drop-shadow-md" />
		</div>

    <HomeBookmarkDrawer
      v-model:show="drawerVisible"
      :is-mobile="isMobile"
      :tree-data="treeData"
      :render-tree-label="renderTreeLabel"
      @navigate="navigateToBookmarkManager"
      @select="handleTreeSelect"
    />
	</div>
  <div class="w-full h-full sun-main">
    <div
      class="cover wallpaper" :style="{
        filter: `blur(${panelState.panelConfig.backgroundBlur}px)`,
        background: `url(${panelState.panelConfig.backgroundImageSrc}) no-repeat`,
        backgroundSize: 'cover',
        backgroundPosition: 'center',
      }"
    />
    <div class="mask" :style="{ backgroundColor: `rgba(0,0,0,${panelState.panelConfig.backgroundMaskNumber})` }" />
    <div ref="scrollContainerRef" class="absolute w-full h-full overflow-auto">
      <div
        class="p-2.5 mx-auto"
        :style="{
          marginTop: `${panelState.panelConfig.marginTop}%`,
          marginBottom: `${panelState.panelConfig.marginBottom}%`,
          maxWidth: (panelState.panelConfig.maxWidth ?? '1200') + panelState.panelConfig.maxWidthUnit,
        }"
      >
        <!-- 头 -->
        <div class="mx-[auto] w-[80%]">
          <div class="flex mx-[auto] items-center justify-center text-white">
            <div class="logo">
              <span class="text-2xl md:text-6xl font-bold text-shadow">
                {{ panelState.panelConfig.logoText }}
              </span>
            </div>
            <div class="divider text-base lg:text-2xl mx-[10px]">
              |
            </div>
            <div class="text-shadow">
              <Clock :hide-second="!panelState.panelConfig.clockShowSecond" />
            </div>
          </div>
          <div v-if="panelState.panelConfig.searchBoxShow" class="flex mt-[20px] mx-auto sm:w-full lg:w-[80%]">
            <SearchBoxWithSuggestions
              :search-items="searchableItems"
              @itemSearch="itemFrontEndSearch"
              @itemSelect="handleSearchItemSelect"
            />
          </div>
        </div>

        <!-- 应用盒子 -->
        <div :style="{ marginLeft: `${panelState.panelConfig.marginX}px`, marginRight: `${panelState.panelConfig.marginX}px` }">
          <!-- 系统监控状态 -->
          <div
            v-if="panelState.panelConfig.systemMonitorShow && authStore.visitMode === VisitMode.VISIT_MODE_LOGIN"
            class="flex mx-auto"
          >
            <SystemMonitor
              :allow-edit="authStore.visitMode === VisitMode.VISIT_MODE_LOGIN"
              :show-title="panelState.panelConfig.systemMonitorShowTitle"
            />
          </div>

          <!-- 组纵向排列 -->
          <div
            v-for="(itemGroup, itemGroupIndex) in filterItems" :key="itemGroupIndex"
            class="item-list mt-[50px]"
            :class="itemGroup.sortStatus ? 'shadow-2xl border shadow-[0_0_30px_10px_rgba(0,0,0,0.3)]  p-[10px] rounded-2xl' : ''"
            @mouseenter="handleSetHoverStatus(itemGroupIndex, true)"
            @mouseleave="handleSetHoverStatus(itemGroupIndex, false)"
          >
            <!-- 分组标题 -->
            <div class="text-white text-xl font-extrabold mb-[20px] ml-[10px] flex items-center">
              <span class="group-title text-shadow">
                {{ itemGroup.title }}
              </span>
              <div
                v-if="authStore.visitMode === VisitMode.VISIT_MODE_LOGIN && panelState.networkMode === PanelStateNetworkModeEnum.lan"
                class="group-buttons ml-2 delay-100 transition-opacity flex"
              >
                <span class="mr-2 cursor-pointer" :title="t('common.add')" @click="handleAddItem(itemGroup.id)">
                  <SvgIcon class="text-white font-xl" icon="typcn:plus" />
                </span>
                <span class="mr-2 cursor-pointer " :title="t('common.sort')" @click="handleSetSortStatus(itemGroup, !itemGroup.sortStatus)">
                  <SvgIcon class="text-white font-xl" icon="ri:drag-drop-line" />
                </span>
              </div>
            </div>

            <!-- 详情图标 -->
            <div v-if="panelState.panelConfig.iconStyle === PanelPanelConfigStyleEnum.info">
              <div v-if="itemGroup.items">
                <VueDraggable
                  v-model="itemGroup.items" item-key="sort" :animation="300"
                  class="icon-info-box"
                  filter=".not-drag"
                  :disabled="!itemGroup.sortStatus"
                  @end="handleSaveSort(itemGroup)"
                >
                  <div v-for="item, index in itemGroup.items" :key="index" :title="item.description" @contextmenu="(e) => handleContextMenu(e, itemGroupIndex, item)">
                    <AppIcon
                      :class="itemGroup.sortStatus ? 'cursor-move' : 'cursor-pointer'"
                      :item-info="item"
                      :icon-text-color="panelState.panelConfig.iconTextColor"
                      :icon-text-info-hide-description="panelState.panelConfig.iconTextInfoHideDescription || false"
                      :icon-text-icon-hide-title="panelState.panelConfig.iconTextIconHideTitle || false"
                      :style="0"
                      @click="handleItemClick(itemGroupIndex, item)"
                    />
                  </div>

                  <div v-if="itemGroup.items.length === 0 && panelState.networkMode === PanelStateNetworkModeEnum.lan" class="not-drag">
                    <AppIcon
                      :class="itemGroup.sortStatus ? 'cursor-move' : 'cursor-pointer'"
                      :item-info="{ icon: { itemType: 3, text: 'subway:add' }, title: t('common.add'), url: '', openMethod: 0 }"
                      :icon-text-color="panelState.panelConfig.iconTextColor"
                      :icon-text-info-hide-description="panelState.panelConfig.iconTextInfoHideDescription || false"
                      :icon-text-icon-hide-title="panelState.panelConfig.iconTextIconHideTitle || false"
                      :style="0"
                      @click="handleAddItem(itemGroup.id)"
                    />
                  </div>
                </VueDraggable>
              </div>
            </div>

            <!-- APP图标宫型盒子 -->
            <div v-if="panelState.panelConfig.iconStyle === PanelPanelConfigStyleEnum.icon">
              <div v-if="itemGroup.items">
                <VueDraggable
                  v-model="itemGroup.items" item-key="sort" :animation="300"
                  class="icon-small-box"

                  filter=".not-drag"
                  :disabled="!itemGroup.sortStatus"
                  @end="handleSaveSort(itemGroup)"
                >
                  <div v-for="item, index in itemGroup.items" :key="index" :title="item.description" @contextmenu="(e) => handleContextMenu(e, itemGroupIndex, item)">
                    <AppIcon
                      :class="itemGroup.sortStatus ? 'cursor-move' : 'cursor-pointer'"
                      :item-info="item"
                      :icon-text-color="panelState.panelConfig.iconTextColor"
                      :icon-text-info-hide-description="!panelState.panelConfig.iconTextInfoHideDescription"
                      :icon-text-icon-hide-title="panelState.panelConfig.iconTextIconHideTitle || false"
                      :style="1"
                      @click="handleItemClick(itemGroupIndex, item)"
                    />
                  </div>

                  <div v-if="itemGroup.items.length === 0 && panelState.networkMode === PanelStateNetworkModeEnum.lan" class="not-drag">
                    <AppIcon
                      class="cursor-pointer"
                      :item-info="{ icon: { itemType: 3, text: 'subway:add' }, title: $t('common.add'), url: '', openMethod: 0 }"
                      :icon-text-color="panelState.panelConfig.iconTextColor"
                      :icon-text-info-hide-description="!panelState.panelConfig.iconTextInfoHideDescription"
                      :icon-text-icon-hide-title="panelState.panelConfig.iconTextIconHideTitle || false"
                      :style="1"
                      @click="handleAddItem(itemGroup.id)"
                    />
                  </div>
                </VueDraggable>
              </div>
            </div>

            <!-- 编辑栏 -->

          </div>
        </div>
        <HomeFooterHtml :html="safeFooterHtml" />
      </div>
    </div>

    <!-- 右键菜单 -->
    <NDropdown
      placement="bottom-start" trigger="manual" :x="dropdownMenuX" :y="dropdownMenuY"
      :options="getDropdownMenuOptions()" :show="dropdownShow" :on-clickoutside="onClickoutside" @select="handleRightMenuSelect"
    />

    <!-- 悬浮按钮 -->
    <div class="fixed-element shadow-[0_0_10px_2px_rgba(0,0,0,0.2)]">
      <HomeActionButtons
        :visit-mode="authStore.visitMode"
        :network-mode="panelState.networkMode"
        :panel-config="panelState.panelConfig"
        @toggle-settings="settingModalShow = !settingModalShow"
        @refresh="handleRefreshData"
        @change-network="handleChangeNetwork"
      />

      <AppStarter v-model:visible="settingModalShow" />
      <NotePad ref="notepadInstance" v-model:visible="notepadVisible" />
    </div>

    <NBackTop
      :listen-to="getScrollListenTarget"
      :right="10"
      :bottom="10"
      style="background-color:transparent;border: none;box-shadow: none;"
    >
      <div class="shadow-[0_0_10px_2px_rgba(0,0,0,0.2)]">
        <NButton color="#2a2a2a6b">
          <template #icon>
            <SvgIcon class="text-white font-xl" icon="icon-park-outline:to-top" />
          </template>
        </NButton>
      </div>
    </NBackTop>

    <EditItem v-model:visible="editItemInfoShow" :item-info="editItemInfoData" :item-group-id="currentAddItenIconGroupId" @done="handleEditSuccess" />

    <HomeWindowModal
      v-model:show="windowShow"
      :title="windowTitle"
      :loading="windowIframeIsLoad"
      :src="windowSrc"
      @loaded="handWindowIframeIdLoad"
    />
  </div>
</template>

<style>
body,
html {
  overflow: hidden;
  background-color: rgb(54, 54, 54);
}
</style>

<style scoped>
.mask {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
}

.sun-main {
  user-select: none;
}

.cover {
  position: absolute;
  width: 100%;
  height: 100%;
  overflow: hidden;
  /* background: url(@/assets/start_sky.jpg) no-repeat; */

  transform: scale(1.05);
}

.text-shadow {
  text-shadow: 2px 2px 50px rgb(0, 0, 0);
}

.app-icon-text-shadow {
  text-shadow: 2px 2px 5px rgb(0, 0, 0);
}

.fixed-element {
  position: fixed;
  /* 将元素固定在屏幕上 */
  right: 10px;
  /* 距离屏幕顶部的距离 */
  bottom: 50px;
  /* 距离屏幕左侧的距离 */
}

.icon-info-box {
  width: 100%;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(200px, 100%), 1fr));
  gap: 18px;

}

.icon-small-box {
  width: 100%;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(100px, 100%), 1fr));
  gap: 18px;

}

/* 响应式图标块布局 */
@media (max-width: 1024px) {
  .icon-info-box {
    grid-template-columns: repeat(auto-fill, minmax(min(160px, 100%), 1fr));
    gap: 14px;
  }

  .icon-small-box {
    grid-template-columns: repeat(auto-fill, minmax(min(85px, 100%), 1fr));
    gap: 14px;
  }
}

/* 响应式图标块布局 - 继续使用grid布局，但减小最小宽度 */
@media (max-width: 768px) {
  .icon-info-box {
    grid-template-columns: repeat(auto-fill, minmax(min(100px, 100%), 1fr));
    gap: 12px;
  }

  .icon-small-box {
    grid-template-columns: repeat(auto-fill, minmax(min(60px, 100%), 1fr));
    gap: 12px;
  }
}

@media (max-width: 480px) {
  .icon-info-box {
    grid-template-columns: repeat(auto-fill, minmax(min(100px, 100%), 1fr));
    gap: 10px;
  }

  .icon-small-box {
    grid-template-columns: repeat(auto-fill, minmax(min(60px, 100%), 1fr));
    gap: 10px;
  }
}

@media (max-width: 360px) {
  .icon-info-box {
    grid-template-columns: repeat(auto-fill, minmax(min(100px, 100%), 1fr));
    gap: 8px;
  }

  .icon-small-box {
    grid-template-columns: repeat(auto-fill, minmax(min(60px, 100%), 1fr));
    gap: 8px;
  }
}



/* 优化条状按钮阴影 */
/* 优化条状按钮阴影 - 已移除，避免污染全局 .fixed 类 */


:deep(.no-focus-outline:focus) {
  box-shadow: none !important;
}

.no-tap-highlight {
  -webkit-tap-highlight-color: transparent !important;
  outline: none !important;
}
</style>
