<script setup lang="ts">
defineProps<{
  html: string
}>()
</script>

<template>
  <div v-if="html" class="mt-5 footer" v-html="html" />
</template>
