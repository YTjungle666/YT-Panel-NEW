<script setup lang="ts">
import { defineAsyncComponent, shallowRef, watch } from 'vue'
import { NSpin } from 'naive-ui'
import type { Component } from 'vue'

const props = defineProps<{
  componentName: string | null
}>()
const loading = shallowRef(false)
const dynamicComponent = shallowRef<Component | null>(null)

const componentLoaders: Record<string, () => Promise<any>> = {
  About: () => import('../../apps/About/index.vue'),
  ForceChangePassword: () => import('../../apps/ForceChangePassword/index.vue'),
  ImportExport: () => import('../../apps/ImportExport/index.vue'),
  ItemGroupManage: () => import('../../apps/ItemGroupManage/index.vue'),
  Style: () => import('../../apps/Style/index.vue'),
  UploadFileManager: () => import('../../apps/UploadFileManager/index.vue'),
  UserInfo: () => import('../../apps/UserInfo/index.vue'),
  Users: () => import('../../apps/Users/index.vue'),
}

function updateComponent(componentName: string | null) {
  const loader = componentName ? componentLoaders[componentName] : null
  if (!loader) {
    dynamicComponent.value = null
    loading.value = false
    return
  }

  loading.value = true
  dynamicComponent.value = defineAsyncComponent(async () => {
    try {
      return await loader()
    }
    catch {
      dynamicComponent.value = null
      return { default: null }
    }
    finally {
      loading.value = false
    }
  })
}

watch(() => props.componentName, (componentName) => {
  updateComponent(componentName)
}, { immediate: true })
</script>

<template>
  <div class="h-full">
    <NSpin :show="loading" style="height: 100%;" content-style="height: 100%;" :delay="500" description="loading...">
      <component :is="dynamicComponent" v-if="dynamicComponent" />
      <div
        v-else-if="!dynamicComponent"
      />
    </NSpin>
  </div>
</template>
