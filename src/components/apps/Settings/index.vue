<script setup lang="ts">
import { computed, ref } from 'vue'
import { NButton, NCard, NInput, NSpace, NText, useMessage } from 'naive-ui'
import { t } from '@/locales'
import { getLanPingUrl, setLanPingUrl, testLanPingUrl } from '@/utils/network'

const ms = useMessage()
const pingUrl = ref(getLanPingUrl())
const testing = ref(false)

const disabled = computed(() => testing.value)

function handleSave() {
  setLanPingUrl(pingUrl.value)
  ms.success(t('common.saveSuccess'))
}

async function handleTest() {
  if (!pingUrl.value.trim()) {
    ms.warning(t('apps.settings.pleaseEnterUrl'))
    return
  }

  testing.value = true
  try {
    const ok = await testLanPingUrl(pingUrl.value)
    if (ok)
      ms.success(t('apps.settings.connectionSuccess'))
    else
      ms.error(t('apps.settings.connectionFailed'))
  } finally {
    testing.value = false
  }
}
</script>

<template>
  <NCard :bordered="false" embedded>
    <NSpace vertical size="large">
      <div>
        <div class="text-base font-medium mb-2">
          {{ t('apps.settings.networkDetection') }}
        </div>
        <NText depth="3">
          {{ t('apps.settings.pingUrlHint') }}
        </NText>
      </div>

      <NInput
        v-model:value="pingUrl"
        :placeholder="t('apps.settings.pingUrlPlaceholder')"
      />

      <NSpace>
        <NButton type="primary" :disabled="disabled" @click="handleSave">
          {{ t('common.save') }}
        </NButton>
        <NButton :loading="testing" :disabled="disabled" @click="handleTest">
          {{ testing ? t('apps.settings.testing') : t('apps.settings.testConnection') }}
        </NButton>
      </NSpace>
    </NSpace>
  </NCard>
</template>
