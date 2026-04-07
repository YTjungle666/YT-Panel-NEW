<script setup lang="ts">
import { NButton, NButtonGroup } from 'naive-ui'
import { SvgIcon } from '@/components/common'
import { PanelStateNetworkModeEnum } from '@/enums'
import { VisitMode } from '@/enums/auth'

defineProps<{
  visitMode: number
  networkMode: number
  panelConfig: {
    netModeChangeButtonShow?: boolean
  }
}>()

const emit = defineEmits<{
  (e: 'toggle-settings'): void
  (e: 'refresh'): void
  (e: 'login'): void
  (e: 'change-network', mode: PanelStateNetworkModeEnum): void
}>()
</script>

<template>
  <NButtonGroup vertical>
    <NButton
      v-if="networkMode === PanelStateNetworkModeEnum.lan && panelConfig.netModeChangeButtonShow && visitMode === VisitMode.VISIT_MODE_LOGIN"
      color="#2a2a2a6b"
      :title="$t('panelHome.changeToWanModel')"
      @click="emit('change-network', PanelStateNetworkModeEnum.wan)"
    >
      <template #icon>
        <SvgIcon class="text-white font-xl" icon="material-symbols:lan-outline-rounded" />
      </template>
    </NButton>

    <NButton
      v-if="networkMode === PanelStateNetworkModeEnum.wan && panelConfig.netModeChangeButtonShow && visitMode === VisitMode.VISIT_MODE_LOGIN"
      color="#2a2a2a6b"
      :title="$t('panelHome.changeToLanModel')"
      @click="emit('change-network', PanelStateNetworkModeEnum.lan)"
    >
      <template #icon>
        <SvgIcon class="text-white font-xl" icon="mdi:wan" />
      </template>
    </NButton>

    <NButton v-if="visitMode === VisitMode.VISIT_MODE_LOGIN" color="#2a2a2a6b" @click="emit('toggle-settings')">
      <template #icon>
        <SvgIcon class="text-white font-xl" icon="majesticons-applications" />
      </template>
    </NButton>

    <NButton color="#2a2a2a6b" :title="$t('panelHome.refreshData')" @click="emit('refresh')">
      <template #icon>
        <SvgIcon class="text-white font-xl" icon="shuaxin" />
      </template>
    </NButton>

    <NButton
      v-if="visitMode === VisitMode.VISIT_MODE_PUBLIC"
      color="#2a2a2a6b"
      :title="$t('panelHome.goToLogin')"
      @click="emit('login')"
    >
      <template #icon>
        <SvgIcon class="text-white font-xl" icon="material-symbols:account-circle" />
      </template>
    </NButton>
  </NButtonGroup>
</template>
