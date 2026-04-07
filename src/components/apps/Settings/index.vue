<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { NButton, NCard, NInput, NSpace, NSwitch, NText, useMessage } from 'naive-ui'
import { t } from '@/locales'
import { getLanPingUrl, setLanPingUrl, testLanPingUrl } from '@/utils/network'
import { getSystemSetting, setSystemSettings } from '@/api/system/systemSetting'
import { useAuthStore } from '@/store'

const SECURITY_PASSWORD_POLICY_KEY = 'security_password_policy'

const ms = useMessage()
const authStore = useAuthStore()
const pingUrl = ref(getLanPingUrl())
const testing = ref(false)
const savingPasswordPolicy = ref(false)
const allowWeakPassword = ref(false)

const disabled = computed(() => testing.value)
const isAdmin = computed(() => authStore.userInfo?.role === 1)

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

async function loadPasswordPolicy() {
  if (!isAdmin.value)
    return

  try {
    const { data } = await getSystemSetting<{ configValue: string }>(SECURITY_PASSWORD_POLICY_KEY)
    const parsed = data?.configValue ? JSON.parse(data.configValue) : {}
    allowWeakPassword.value = !!parsed.allowWeakPassword
  } catch {
    allowWeakPassword.value = false
  }
}

async function handleSavePasswordPolicy() {
  savingPasswordPolicy.value = true
  try {
    await setSystemSettings({
      [SECURITY_PASSWORD_POLICY_KEY]: {
        allowWeakPassword: allowWeakPassword.value,
      },
    })
    ms.success(t('common.saveSuccess'))
  } finally {
    savingPasswordPolicy.value = false
  }
}

onMounted(() => {
  loadPasswordPolicy()
})
</script>

<template>
  <NSpace vertical size="large">
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

    <NCard v-if="isAdmin" :bordered="false" embedded>
      <NSpace vertical size="large">
        <div>
          <div class="text-base font-medium mb-2">
            {{ t('apps.settings.passwordSecurity') }}
          </div>
          <NText depth="3">
            {{ t('apps.settings.passwordSecurityHint') }}
          </NText>
        </div>

        <div class="flex items-center justify-between gap-4 rounded-xl bg-slate-50 px-4 py-3 dark:bg-zinc-800/80">
          <div>
            <div class="font-medium">
              {{ t('apps.settings.allowWeakPassword') }}
            </div>
            <div class="text-sm text-slate-500 dark:text-slate-400">
              {{ t('apps.settings.allowWeakPasswordHint') }}
            </div>
          </div>
          <NSwitch v-model:value="allowWeakPassword" />
        </div>

        <NSpace>
          <NButton type="primary" :loading="savingPasswordPolicy" @click="handleSavePasswordPolicy">
            {{ t('common.save') }}
          </NButton>
        </NSpace>
      </NSpace>
    </NCard>
  </NSpace>
</template>
