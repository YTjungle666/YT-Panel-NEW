<script setup lang="ts">
import type { FormInst, FormRules } from 'naive-ui'
import { NAlert, NButton, NCard, NDivider, NForm, NFormItem, NInput, useDialog, useMessage } from 'naive-ui'
import { ref } from 'vue'
import { logout } from '@/api'
import { updatePassword } from '@/api/system/user'
import { useAppStore, useAuthStore, usePanelState, useUserStore } from '@/store'
import { clearAppScopedStorage } from '@/store/modules/auth/helper'
import { t } from '@/locales'
import { router } from '@/router'

const userStore = useUserStore()
const authStore = useAuthStore()
const appStore = useAppStore()
const panelState = usePanelState()
const ms = useMessage()
const dialog = useDialog()
const formRef = ref<FormInst | null>(null)

const formState = ref({
  loading: false,
  form: {
    password: '',
    oldPassword: '',
    confirmPassword: '',
  },
})

const formRules: FormRules = {
  oldPassword: {
    required: true,
    trigger: 'blur',
    message: t('common.inputPlaceholder'),
  },
  password: {
    required: true,
    trigger: 'blur',
    min: 8,
    max: 64,
    message: t('settingUserInfo.passwordLimit'),
  },
  confirmPassword: {
    required: true,
    trigger: 'blur',
    min: 8,
    max: 64,
    message: t('settingUserInfo.passwordLimit'),
  },
}

function resetPasswordForm() {
  formState.value.form = {
    password: '',
    oldPassword: '',
    confirmPassword: '',
  }
}

async function resetClientSession(successMessage: string) {
  userStore.resetUserInfo()
  authStore.removeToken()
  panelState.removeState()
  appStore.removeToken()
  clearAppScopedStorage()
  ms.success(successMessage)
  await router.replace({ path: '/login' })
}

async function logoutApi() {
  try {
    await logout()
  }
  finally {
    await resetClientSession(t('settingUserInfo.logoutSuccess'))
  }
}

function handleUpdatePassword(e: MouseEvent) {
  e.preventDefault()
  formRef.value?.validate((errors) => {
    if (errors)
      return

    if (formState.value.form.password !== formState.value.form.confirmPassword) {
      ms.error(t('settingUserInfo.confirmPasswordInconsistentMsg'))
      return
    }

    formState.value.loading = true
    updatePassword(formState.value.form.oldPassword, formState.value.form.password)
      .then(async ({ code, msg }) => {
        if (code !== 0) {
          ms.error(msg || t('common.failed'))
          return
        }

        resetPasswordForm()
        await resetClientSession(t('settingUserInfo.passwordUpdateSuccess'))
      })
      .catch(() => {
        ms.error(t('common.serverError'))
      })
      .finally(() => {
        formState.value.loading = false
      })
  })
}

function handleLogout() {
  dialog.warning({
    title: t('common.warning'),
    content: t('settingUserInfo.confirmLogoutText'),
    positiveText: t('common.confirm'),
    negativeText: t('common.cancel'),
    onPositiveClick: () => {
      void logoutApi()
    },
  })
}
</script>

<template>
  <div class="bg-slate-200 dark:bg-zinc-900 p-2 h-full">
    <NCard style="border-radius:10px" size="small">
      <NAlert type="warning" class="mb-4">
        {{ $t('settingUserInfo.forceChangePasswordNotice') }}
      </NAlert>
      <NForm ref="formRef" :model="formState.form" :rules="formRules">
        <NFormItem path="oldPassword" :label="$t('settingUserInfo.oldPassword')">
          <NInput v-model:value="formState.form.oldPassword" :maxlength="64" type="password" :placeholder="$t('settingUserInfo.oldPassword')" />
        </NFormItem>

        <NFormItem path="password" :label="$t('settingUserInfo.newPassword')">
          <NInput v-model:value="formState.form.password" :maxlength="64" type="password" :placeholder="$t('settingUserInfo.newPassword')" />
        </NFormItem>

        <NFormItem path="confirmPassword" :label="$t('settingUserInfo.confirmPassword')">
          <NInput v-model:value="formState.form.confirmPassword" :maxlength="64" type="password" :placeholder="$t('settingUserInfo.confirmPassword')" />
        </NFormItem>
      </NForm>
      <NDivider style="margin: 10px 0;" dashed />
      <div class="flex justify-end gap-2">
        <NButton size="small" type="error" secondary @click="handleLogout">
          {{ $t('settingUserInfo.logout') }}
        </NButton>
        <NButton type="success" size="small" :loading="formState.loading" @click="handleUpdatePassword">
          {{ $t('common.save') }}
        </NButton>
      </div>
    </NCard>
  </div>
</template>
