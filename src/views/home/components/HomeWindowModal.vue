<script setup lang="ts">
import { NModal, NSkeleton, NSpin } from 'naive-ui'

const props = defineProps<{
  show: boolean
  title: string
  loading: boolean
  src: string
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'loaded', payload: Event): void
}>()
</script>

<template>
  <NModal
    :show="props.show"
    :mask-closable="false"
    preset="card"
    style="max-width: 1000px;height: 600px;border-radius: 1rem;"
    :bordered="true"
    size="small"
    role="dialog"
    aria-modal="true"
    @update:show="(value) => emit('update:show', value)"
  >
    <template #header>
      <div class="flex items-center">
        <span class="mr-[20px]">
          {{ props.title }}
        </span>
        <NSpin v-if="props.loading" size="small" />
      </div>
    </template>
    <div class="w-full h-full rounded-2xl overflow-hidden border dark:border-zinc-700">
      <div v-if="props.loading" class="flex flex-col p-5">
        <NSkeleton height="50px" width="100%" class="rounded-lg" />
        <NSkeleton height="180px" width="100%" class="mt-[20px] rounded-lg" />
        <NSkeleton height="180px" width="100%" class="mt-[20px] rounded-lg" />
      </div>
      <iframe
        v-show="!props.loading"
        id="windowIframeId"
        :src="props.src"
        class="w-full h-full"
        frameborder="0"
        @load="(event) => emit('loaded', event)"
      />
    </div>
  </NModal>
</template>
