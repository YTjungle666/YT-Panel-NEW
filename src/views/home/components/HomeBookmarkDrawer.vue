<script setup lang="ts">
import { NButton, NDrawer, NDrawerContent, NTree } from 'naive-ui'

const props = defineProps<{
  show: boolean
  isMobile: boolean
  treeData: any[]
  renderTreeLabel: ((payload: { option: any }) => any) | undefined
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'navigate'): void
  (e: 'select', keys: (string | number)[]): void
}>()
</script>

<template>
  <NDrawer
    :show="props.show"
    placement="left"
    :width="props.isMobile ? '80%' : 300"
    style="overflow-y: auto;"
    @update:show="(value) => emit('update:show', value)"
  >
    <NDrawerContent style="min-height: 100vh;">
      <template #header>
        <div class="flex items-center justify-between w-full">
          <span class="text-lg font-medium">{{ $t('bookmarkManager.bookmarkList') }}</span>
          <NButton type="info" size="small" round @click="emit('navigate')">
            {{ $t('bookmarkManager.management') }}
          </NButton>
        </div>
      </template>
      <NTree
        :data="props.treeData"
        block-line
        expand-on-click
        :default-expanded-keys="props.treeData.length > 0 ? [props.treeData[0].key] : []"
        :render-label="props.renderTreeLabel"
        @update:selected-keys="(keys) => emit('select', keys)"
      />
    </NDrawerContent>
  </NDrawer>
</template>
