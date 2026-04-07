<script setup lang="ts">
import { computed } from 'vue'
import GenericProgress from '../components/GenericProgress/index.vue'
import { correctionNumber, correctionNumberByCardStyle } from './common'
import type { PanelPanelConfigStyleEnum } from '@/enums'
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
const cpuUsage = computed(() => monitorData.value?.cpu?.usages?.[0] || 0)
</script>

<template>
  <GenericProgress
    :progress-color="progressColor"
    :progress-rail-color="progressRailColor"
    :progress-height="5"
    :percentage="correctionNumberByCardStyle(cpuUsage, cardTypeStyle)"
    :card-type-style="cardTypeStyle"
    :info-card-right-text="`${correctionNumber(cpuUsage)}%`"
    info-card-left-text="CPU"
    :text-color="textColor"
    style="width: 100%;"
  />
</template>
