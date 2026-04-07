<script setup lang="ts">
import { computed } from 'vue'
import GenericProgress from '../components/GenericProgress/index.vue'
import { correctionNumberByCardStyle } from './common'
import type { PanelPanelConfigStyleEnum } from '@/enums'
import { bytesToSize } from '@/utils/cmn'
import { useSharedSystemMonitor } from '../useSharedSystemMonitor'

interface Prop {
  cardTypeStyle: PanelPanelConfigStyleEnum
  refreshInterval: number
  textColor: string
  progressColor: string
  progressRailColor: string
}

const props = defineProps<Prop>()
const { monitorData } = useSharedSystemMonitor(props.refreshInterval)
const memoryState = computed(() => monitorData.value?.memory || null)

function formatMemorySize(v: number): string {
  return bytesToSize(v)
}
</script>

<template>
  <GenericProgress
    :progress-color="progressColor"
    :progress-rail-color="progressRailColor"
    :progress-height="5"
    :percentage="correctionNumberByCardStyle(memoryState?.usedPercent || 0, cardTypeStyle)"
    :card-type-style="cardTypeStyle"
    :info-card-right-text="`${formatMemorySize(memoryState?.used || 0)}/${formatMemorySize((memoryState?.total || 0) - (memoryState?.used || 0) || 0)}`"
    info-card-left-text="RAM"
    :text-color="textColor"
  />
</template>
