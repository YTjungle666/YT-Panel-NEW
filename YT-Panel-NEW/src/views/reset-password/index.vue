<template>
  <div class="flex justify-center items-center min-h-screen bg-slate-900">
    <NCard class="card login-card" bordered>
      <div class="card-title">
        <div class="text-center text-2xl mb-1">
          <NGradientText :gradient="{ deg: 90, from: '#3b82f6', to: '#8b5cf6' }">
            YT-Panel
          </NGradientText>
        </div>
        <div class="text-center text-sm text-slate-400">
          {{ $t('resetPassword.title') }}
        </div>
      </div>

      <NAlert type="warning" class="mt-6" :show-icon="true">
        <div class="text-sm leading-6">
          <div>{{ $t('resetPassword.statusNotice') }}</div>
          <div class="mt-2">{{ $t('resetPassword.contactAdmin') }}</div>
        </div>
      </NAlert>

      <NForm ref="formRef" :model="form" :rules="rules" class="mt-6">
        <NFormItem path="email">
          <NInput v-model:value="form.email" :placeholder="$t('resetPassword.emailPlaceholder')" type="text">
            <template #prefix>
              <SvgIcon icon="email" class="w-4 h-4 text-slate-500" />
            </template>
          </NInput>
        </NFormItem>

        <NFormItem path="emailVCode">
          <div class="flex w-full gap-2">
            <NInput
              v-model:value="form.emailVCode"
              class="flex-1"
              :placeholder="$t('resetPassword.emailCodePlaceholder')"
              type="text"
            >
              <template #prefix>
                <SvgIcon icon="check" class="w-4 h-4 text-slate-500" />
              </template>
            </NInput>
            <NButton type="default" :loading="sendCodeLoading" @click="handleSendCode">
              {{ $t('resetPassword.sendCode') }}
            </NButton>
          </div>
        </NFormItem>

        <NFormItem path="password">
          <NInput
            v-model:value="form.password"
            :placeholder="$t('resetPassword.newPasswordPlaceholder')"
            type="password"
            show-password-on="click"
          >
            <template #prefix>
              <SvgIcon icon="lock" class="w-4 h-4 text-slate-500" />
            </template>
          </NInput>
        </NFormItem>

        <NFormItem path="confirmPassword">
          <NInput
            v-model:value="form.confirmPassword"
            :placeholder="$t('resetPassword.confirmPasswordPlaceholder')"
            type="password"
            show-password-on="click"
          >
            <template #prefix>
              <SvgIcon icon="lock" class="w-4 h-4 text-slate-500" />
            </template>
          </NInput>
        </NFormItem>

        <NFormItem>
          <NButton type="primary" block :loading="submitLoading" @click="handleReset">
            {{ $t('resetPassword.submit') }}
          </NButton>
        </NFormItem>

        <NFormItem>
          <NButton type="default" block @click="router.push('/login')">
            {{ $t('resetPassword.backToLogin') }}
          </NButton>
        </NFormItem>
      </NForm>
    </NCard>
  </div>
</template>

<script setup lang="ts">
import { NAlert, NButton, NCard, NForm, NFormItem, NGradientText, NInput, useMessage } from 'naive-ui'
import { ref } from 'vue'
import type { FormInst } from 'naive-ui'
import { resetPasswordByVCode, sendResetPasswordVCode } from '@/api/login'
import { SvgIcon } from '@/components/common'
import { router } from '@/router'
import { t } from '@/locales'

const formRef = ref<FormInst | null>(null)
const message = useMessage()
const sendCodeLoading = ref(false)
const submitLoading = ref(false)

const form = ref({
  email: '',
  emailVCode: '',
  password: '',
  confirmPassword: '',
})

const rules = {
  email: [
    { required: true, message: t('resetPassword.emailRequired'), trigger: 'blur' },
    {
      pattern: /^[\w-]+(\.[\w-]+)*@[\w-]+(\.[\w-]+)+$/,
      message: t('resetPassword.emailInvalid'),
      trigger: 'blur',
    },
  ],
  emailVCode: [
    { required: true, message: t('resetPassword.codeRequired'), trigger: 'blur' },
  ],
  password: [
    { required: true, message: t('resetPassword.passwordRequired'), trigger: 'blur' },
    { min: 6, max: 64, message: t('resetPassword.passwordLimit'), trigger: 'blur' },
  ],
  confirmPassword: [
    { required: true, message: t('resetPassword.confirmPasswordRequired'), trigger: 'blur' },
    {
      validator(_rule: any, value: string) {
        if (!value || form.value.password === value)
          return true
        return new Error(t('resetPassword.confirmPasswordMismatch'))
      },
      trigger: 'blur',
    },
  ],
}

const handleSendCode = async () => {
  const email = form.value.email.trim()
  if (!email) {
    message.error(t('resetPassword.emailRequired'))
    return
  }
  if (!/^[\w-]+(\.[\w-]+)*@[\w-]+(\.[\w-]+)+$/.test(email)) {
    message.error(t('resetPassword.emailInvalid'))
    return
  }

  try {
    sendCodeLoading.value = true
    const response = await sendResetPasswordVCode(email, {})
    if (response.code === 0)
      message.success(response.msg || t('common.success'))
    else
      message.error(response.msg || t('resetPassword.sendCodeFailed'))
  }
  catch (error: any) {
    message.error(error?.msg || error?.message || t('resetPassword.sendCodeFailed'))
  }
  finally {
    sendCodeLoading.value = false
  }
}

const handleReset = async () => {
  try {
    await formRef.value?.validate()
    submitLoading.value = true
    const response = await resetPasswordByVCode({
      email: form.value.email,
      emailVCode: form.value.emailVCode,
      password: form.value.password,
    })

    if (response.code === 0) {
      message.success(t('resetPassword.resetSuccess'))
      router.push('/login')
      return
    }

    message.error(response.msg || t('resetPassword.resetFailed'))
  }
  catch (error: any) {
    message.error(error?.msg || error?.message || t('resetPassword.resetFailed'))
  }
  finally {
    submitLoading.value = false
  }
}
</script>
