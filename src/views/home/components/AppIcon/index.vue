<script setup lang="ts">
import { computed } from 'vue'
import { ItemIcon } from '@/components/common'
import { PanelPanelConfigStyleEnum } from '@/enums'

interface Prop {
  itemInfo?: Panel.ItemInfo
  size?: number // 默认70
  forceBackground?: string // 强制背景色
  iconTextColor?: string
  iconTextInfoHideDescription: boolean
  iconTextIconHideTitle: boolean
  style: PanelPanelConfigStyleEnum
}

const props = withDefaults(defineProps<Prop>(), {
  size: 70,
})

const defaultBackground = '#2a2a2a6b'

const calculateLuminance = (color: string) => {
  const hex = color.replace(/^#/, '')
  const r = parseInt(hex.substring(0, 2), 16)
  const g = parseInt(hex.substring(2, 4), 16)
  const b = parseInt(hex.substring(4, 6), 16)
  return (0.299 * r + 0.587 * g + 0.114 * b) / 255
}

const textColor = computed(() => {
  const luminance = calculateLuminance(props.itemInfo?.icon?.backgroundColor || defaultBackground)
  return luminance > 0.5 ? 'black' : 'white'
})

// 根据面板样式计算ItemIcon的尺寸，使图标内容随容器等比缩放
const itemIconSize = computed(() => {
  if (props.style === PanelPanelConfigStyleEnum.info) {
    // Info模式下，图标容器尺寸为60px/50px/40px（响应式），内部图标占80%
    return Math.round(props.size * 0.8)
  } else {
    // Icon模式下，内部图标占容器的80%
    return Math.round(props.size * 0.8)
  }
})
</script>

<template>
  <div class="app-icon w-full">
    <!-- 详情图标 -->
    <div
      v-if="style === PanelPanelConfigStyleEnum.info"
      class="app-icon-info w-full rounded-2xl  transition-all duration-200 hover:shadow-[0_0_20px_10px_rgba(0,0,0,0.2)] flex"
      :style="{ background: itemInfo?.icon?.backgroundColor || defaultBackground }"
    >
      <!-- 图标 -->
      <div class="app-icon-info-icon">
        <div class="w-full h-full flex items-center justify-center ">
          <ItemIcon :item-icon="itemInfo?.icon" force-background="transparent" :size="itemIconSize" class="overflow-hidden rounded-xl" />
        </div>
      </div>

      <!-- 文字 -->
      <!-- 如果为纯白色，将自动根据背景的明暗计算字体的黑白色 -->
      <div class="text-white flex items-center" :style="{ color: (iconTextColor === '#ffffff') ? textColor : iconTextColor, maxWidth: 'calc(100% - 20px)', padding: '0 10px' }">
        <div class="app-icon-info-text-box w-full">
          <div class="app-icon-info-text-box-title font-semibold w-full">
            {{ itemInfo?.title }}
          </div>
          <div v-if="!iconTextInfoHideDescription" class="app-icon-info-text-box-description">
            {{ itemInfo?.description }}
          </div>
        </div>
      </div>
    </div>

    <!-- 极简(小)图标（APP） -->
    <div v-if="style === PanelPanelConfigStyleEnum.icon" class="app-icon-small">
      <div
        class="app-icon-small-icon overflow-hidden rounded-2xl sunpanel mx-auto rounded-2xl transition-all duration-200 hover:shadow-[0_0_20px_10px_rgba(0,0,0,0.2)]"
        :title="itemInfo?.description"
      >
        <ItemIcon :item-icon="itemInfo?.icon" :size="itemIconSize" />
      </div>
      <div
        v-if="!iconTextIconHideTitle"
        class="app-icon-small-title text-center app-icon-text-shadow cursor-pointer mt-[2px]"
        :style="{ color: iconTextColor }"
      >
        <span>{{ itemInfo?.title }}</span>
      </div>
      <div
        v-if="!iconTextInfoHideDescription && itemInfo?.description"
        class="app-icon-small-description text-center text-xs opacity-80 cursor-pointer mt-[1px]"
        :style="{ color: iconTextColor }"
      >
        {{ itemInfo?.description }}
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 响应式图标块设计 */
.app-icon-info-icon {
  width: min(70px, 100%);
  height: min(70px, 100%);
  min-width: min(70px, 100%);
  aspect-ratio: 1 / 1;
}

.app-icon-small-icon {
  width: min(70px, 100%);
  height: min(70px, 100%);
  aspect-ratio: 1 / 1;
}

/* 确保item-icon完全填充容器 */
:deep(.item-icon) {
  width: 100% !important;
  height: 100% !important;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 针对不同屏幕尺寸的等比缩小 */
@media (max-width: 1024px) {
  .app-icon-info-icon {
    width: min(55px, 100%);
    height: min(55px, 100%);
    min-width: min(55px, 100%);
  }
  
  .app-icon-small-icon {
    width: min(55px, 100%);
    height: min(55px, 100%);
  }
}

@media (max-width: 768px) {
  .app-icon-info-icon {
    width: min(45px, 100%);
    height: min(45px, 100%);
    min-width: min(45px, 100%);
  }
  
  .app-icon-small-icon {
    width: min(45px, 100%);
    height: min(45px, 100%);
  }
  
  .app-icon-info-text-box-title {
    font-size: 0.85rem !important;
    word-wrap: break-word;
    overflow-wrap: break-word;
    white-space: normal;
  }
  
  .app-icon-info-text-box-description {
    font-size: 0.7rem !important;
    word-wrap: break-word;
    overflow-wrap: break-word;
    white-space: normal;
  }
  
  .app-icon-small-title {
    font-size: 0.75rem !important;
  }
}

@media (max-width: 480px) {
  .app-icon-info-icon {
    width: min(40px, 100%);
    height: min(40px, 100%);
    min-width: min(40px, 100%);
  }
  
  .app-icon-small-icon {
    width: min(40px, 100%);
    height: min(40px, 100%);
  }
  
  .app-icon-info-text-box-title {
    font-size: 0.8rem !important;
  }
  
  .app-icon-info-text-box-description {
    font-size: 0.65rem !important;
  }
  
  .app-icon-small-title {
    font-size: 0.7rem !important;
  }
}

@media (max-width: 360px) {
  .app-icon-info-icon {
    width: min(35px, 100%);
    height: min(35px, 100%);
    min-width: min(35px, 100%);
  }
  
  .app-icon-small-icon {
    width: min(35px, 100%);
    height: min(35px, 100%);
  }
  
  .app-icon-info-text-box-title {
    font-size: 0.75rem !important;
  }
  
  .app-icon-info-text-box-description {
    font-size: 0.6rem !important;
  }
  
  .app-icon-small-title {
    font-size: 0.65rem !important;
  }
}
</style>
