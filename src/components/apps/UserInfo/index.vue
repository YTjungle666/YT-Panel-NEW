<script setup lang="ts">
import type { FormInst, FormRules } from 'naive-ui'
import { NAlert, NButton, NCard, NDivider, NForm, NFormItem, NInput, NSelect, useDialog, useMessage } from 'naive-ui'
import { computed, ref } from 'vue'
import { useAppStore, useAuthStore, usePanelState, useUserStore } from '@/store'
import { languageOptions } from '@/utils/defaultData'
import type { Language, Theme } from '@/store/modules/app/helper'
import { logout } from '@/api'
import { RoundCardModal, SvgIcon } from '@/components/common/'
import { updateInfo, updatePassword } from '@/api/system/user'
import { updateLocalUserInfo } from '@/utils/cmn'
import { t } from '@/locales'
import { clearAppScopedStorage } from '@/store/modules/auth/helper'
const userStore = useUserStore()
const authStore = useAuthStore()
const appStore = useAppStore()
const panelState = usePanelState()
const ms = useMessage()
const dialog = useDialog()

const languageValue = ref(appStore.language)
const themeValue = ref(appStore.theme)
const nickName = ref(authStore.userInfo?.name || '')
const isEditNickNameStatus = ref(false)
const formRef = ref<FormInst | null>(null)
const isForcePasswordChange = computed(() => !!authStore.userInfo?.mustChangePassword)
const themeOptions: { label: string; key: string; value: Theme }[] = [
  { label: t('apps.userInfo.themeStyle.dark'), key: 'dark', value: 'dark' },
  { label: t('apps.userInfo.themeStyle.light'), key: 'light', value: 'light' },
  { label: t('apps.userInfo.themeStyle.auto'), key: 'Auto', value: 'auto' },
]
const updatePasswordModalState = ref({
  show: false,
  loading: false,
  form: {
    password: '',
    oldPassword: '',
    confirmPassword: '',
  },
})

const updatePasswordModalFormRules: FormRules = {
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

async function logoutApi() {
  await logout()
  userStore.resetUserInfo()
  authStore.removeToken()
  panelState.removeState()
  appStore.removeToken()
  clearAppScopedStorage()
  ms.success(t('settingUserInfo.logoutSuccess'))
  location.reload()
}

function handleSaveInfo() {
  updateInfo(nickName.value).then(({ code, msg }) => {
    if (code === 0) {
      updateLocalUserInfo()
      isEditNickNameStatus.value = false
    }
    else {
      ms.error(`${t('common.editFail')}:${msg}`)
    }
  })
}

function resetPasswordForm() {
  updatePasswordModalState.value.form = {
    password: '',
    oldPassword: '',
    confirmPassword: '',
  }
}

async function refreshUserAuthInfo() {
  const authInfo = await updateLocalUserInfo()
  if (authInfo?.user) {
    authStore.setUserInfo(authInfo.user)
  }
}

function openPasswordModal() {
  updatePasswordModalState.value.show = true
}

function handleUpdatePassword(e: MouseEvent) {
  e.preventDefault()
  formRef.value?.validate((errors) => {
    if (errors)
      return

    if (updatePasswordModalState.value.form.password !== updatePasswordModalState.value.form.confirmPassword) {
      ms.error(t('settingUserInfo.confirmPasswordInconsistentMsg'))
      return
    }
    updatePasswordModalState.value.loading = true
    updatePassword(updatePasswordModalState.value.form.oldPassword, updatePasswordModalState.value.form.password).then(async ({ code, msg }) => {
      if (code === 0) {
        updatePasswordModalState.value.show = false
        resetPasswordForm()
        await refreshUserAuthInfo()
        ms.success(t('settingUserInfo.passwordUpdateSuccess'))
      }
      else {
        ms.error(msg || t('common.failed'))
      }
    }).finally(() => {
      updatePasswordModalState.value.loading = false
    }).catch(() => {
      ms.error(t('common.serverError'))
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
      logoutApi()
    },
  })
}

function handleChangeLanuage(value: Language) {
  languageValue.value = value
  appStore.setLanguage(value)
  location.reload()
}

function handleChangeTheme(value: Theme) {
  themeValue.value = value
  appStore.setTheme(value)
}
</script>

<template>
  <div class="bg-slate-200 dark:bg-zinc-900 p-2 h-full">
    <template v-if="isForcePasswordChange">
      <NCard style="border-radius:10px" size="small">
        <NAlert type="warning" class="mb-4">
          {{ $t('settingUserInfo.forceChangePasswordNotice') }}
        </NAlert>
        <NForm ref="formRef" :model="updatePasswordModalState.form" :rules="updatePasswordModalFormRules">
          <NFormItem path="oldPassword" :label="$t('settingUserInfo.oldPassword')">
            <NInput v-model:value="updatePasswordModalState.form.oldPassword" :maxlength="64" type="password" :placeholder="$t('settingUserInfo.oldPassword')" />
          </NFormItem>

          <NFormItem path="password" :label="$t('settingUserInfo.newPassword')">
            <NInput v-model:value="updatePasswordModalState.form.password" :maxlength="64" type="password" :placeholder="$t('settingUserInfo.newPassword')" />
          </NFormItem>

          <NFormItem path="confirmPassword" :label="$t('settingUserInfo.confirmPassword')">
            <NInput v-model:value="updatePasswordModalState.form.confirmPassword" :maxlength="64" type="password" :placeholder="$t('settingUserInfo.confirmPassword')" />
          </NFormItem>
        </NForm>
        <NDivider style="margin: 10px 0;" dashed />
        <div class="flex justify-end gap-2">
          <NButton size="small" type="error" secondary @click="handleLogout">
            {{ $t('settingUserInfo.logout') }}
          </NButton>
          <NButton type="success" size="small" :loading="updatePasswordModalState.loading" @click="handleUpdatePassword">
            {{ $t('common.save') }}
          </NButton>
        </div>
      </NCard>
    </template>

    <template v-else>
      <NCard style="border-radius:10px" size="small">
      <div>
        <div class="text-slate-500 font-bold">
          {{ $t('common.username') }}
        </div>

        <div>
          {{ authStore.userInfo?.username }}
        </div>
      </div>

      <div class="mt-[10px]">
        <div class="text-slate-500 font-bold">
          {{ $t('common.nikeName') }}
        </div>

        <div v-if="!isEditNickNameStatus">
          {{ authStore.userInfo?.name }}

          <NButton size="small" text type="info" @click="isEditNickNameStatus = !isEditNickNameStatus">
            {{ $t('common.edit') }}
          </NButton>
        </div>

        <div v-else class="flex items-center">
          <div class="max-w-[150px]">
            <NInput v-model:value="nickName" type="text" :placeholder="$t('common.inputPlaceholder')" />
          </div>
          <NButton size="small" quaternary type="info" @click="handleSaveInfo">
            {{ $t('common.save') }}
          </NButton>
        </div>
      </div>

      <div class="mt-[10px]">
        <div class="text-slate-500 font-bold">
          {{ $t('common.language') }}
        </div>
        <div class="max-w-[200px]">
          <NSelect v-model:value="languageValue" :options="languageOptions" @update-value="handleChangeLanuage" />
        </div>
      </div>

      <div class="mt-[10px]">
        <div class="text-slate-500 font-bold">
          {{ $t('apps.userInfo.theme') }}
        </div>
        <div class="max-w-[200px]">
          <NSelect v-model:value="themeValue" :options="themeOptions" @update-value="handleChangeTheme" />
        </div>
      </div>

      <NDivider style="margin: 10px 0;" dashed />
      <div>
        <NButton size="small" text type="info" @click="openPasswordModal">
          {{ $t('settingUserInfo.updatePassword') }}
        </NButton>
      </div>
      </NCard>

      <NCard style="border-radius:10px" class="mt-[10px]" size="small">
        <NButton size="small" text type="error" @click="handleLogout">
          <template #icon>
            <SvgIcon icon="tabler:logout" />
          </template>
          {{ $t('settingUserInfo.logout') }}
        </NButton>
      </NCard>
    </template>

    <RoundCardModal v-if="!isForcePasswordChange" v-model:show="updatePasswordModalState.show" size="small" preset="card" style="width: 420px" :mask-closable="true" :closable="true" :title="$t('settingUserInfo.updatePassword')">
      <NForm ref="formRef" :model="updatePasswordModalState.form" :rules="updatePasswordModalFormRules">
        <NFormItem path="oldPassword" :label="$t('settingUserInfo.oldPassword')">
          <NInput v-model:value="updatePasswordModalState.form.oldPassword" :maxlength="64" type="password" :placeholder="$t('settingUserInfo.oldPassword')" />
        </NFormItem>

        <NFormItem path="password" :label="$t('settingUserInfo.newPassword')">
          <NInput v-model:value="updatePasswordModalState.form.password" :maxlength="64" type="password" :placeholder="$t('settingUserInfo.newPassword')" />
        </NFormItem>

        <NFormItem path="confirmPassword" :label="$t('settingUserInfo.confirmPassword')">
          <NInput v-model:value="updatePasswordModalState.form.confirmPassword" :maxlength="64" type="password" :placeholder="$t('settingUserInfo.confirmPassword')" />
        </NFormItem>
      </NForm>

      <template #footer>
        <div class="float-right">
          <NButton type="success" size="small" :loading="updatePasswordModalState.loading" @click="handleUpdatePassword">
            {{ $t('common.save') }}
          </NButton>
        </div>
      </template>
    </RoundCardModal>
  </div>
</template>
