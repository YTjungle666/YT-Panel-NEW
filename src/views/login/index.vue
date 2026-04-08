<script setup lang="ts">
import { NButton, NCard, NForm, NFormItem, NGradientText, NInput, useMessage } from 'naive-ui'
import { onMounted, ref } from 'vue'
import { login } from '@/api'
import { getLoginConfig } from '@/api/openness'
import { useAuthStore } from '@/store'
import { SvgIcon } from '@/components/common'
import { router } from '@/router'
import { t } from '@/locales'
import { VisitMode } from '@/enums/auth'
import { clearAppScopedStorage } from '@/store/modules/auth/helper'
import { getList as getGroupList } from '@/api/panel/itemIconGroup'
import { ss } from '@/utils/storage/local'
import { logError } from '@/utils/logger'

const authStore = useAuthStore()
const ms = useMessage()
const loading = ref(false)
const GROUP_LIST_CACHE_KEY = 'groupListCache'
const isShowRegister = ref(false)

const form = ref<Login.LoginReqest>({
  username: '',
  password: '',
})

const loadLoginConfig = async () => {
  try {
    const res = await getLoginConfig<Openness.open.LoginVcodeResponse>()
    if (res.code !== 0)
      return

    const registerConfig = res.data?.register as Openness.open.LoginConfigRegister | boolean | undefined
    isShowRegister.value = typeof registerConfig === 'boolean'
      ? registerConfig
      : !!registerConfig?.openRegister
  }
  catch (error) {
    logError('获取登录配置失败', error)
  }
}

const loginPost = async () => {
  loading.value = true
  try {
    const res = await login<Login.LoginResponse>(form.value)
    if (res.code === 0) {
      clearAppScopedStorage()

      authStore.setUserInfo(res.data)
      authStore.setVisitMode(VisitMode.VISIT_MODE_LOGIN)

      if (!res.data.mustChangePassword) {
        const groupListRes = await getGroupList() as any
        if (groupListRes.code === 0 && groupListRes.data)
          ss.set(GROUP_LIST_CACHE_KEY, groupListRes.data.list || [])
      }

      if (!res.data.mustChangePassword)
        ms.success(`Hi ${res.data.name},${t('login.welcomeMessage')}`)
      router.push({ path: '/home' })
      return
    }
  }
  finally {
    loading.value = false
  }
}

function handleSubmit() {
  loginPost()
}

onMounted(() => {
  loadLoginConfig()
})
</script>

<template>
  <div class="login-container">
    <NCard class="login-card" style="border-radius: 20px;">
      <div class="login-title  ">
        <NGradientText :size="30" type="success" class="!font-bold">
          {{ $t('common.appName') }}
        </NGradientText>
      </div>
      <NForm :model="form" label-width="100px" @keydown.enter="handleSubmit">
        <NFormItem>
          <NInput v-model:value="form.username" :placeholder="$t('login.usernamePlaceholder')">
            <template #prefix>
              <SvgIcon icon="ph:user-bold" />
            </template>
          </NInput>
        </NFormItem>

        <NFormItem>
          <NInput v-model:value="form.password" type="password" :placeholder="$t('login.passwordPlaceholder')">
            <template #prefix>
              <SvgIcon icon="mdi:password-outline" />
            </template>
          </NInput>
        </NFormItem>

        <NFormItem style="margin-top: 10px">
          <NButton type="primary" block :loading="loading" @click="handleSubmit">
            {{ $t('login.loginButton') }}
          </NButton>
        </NFormItem>

        <div class="flex justify-end gap-2">
          <NButton v-if="isShowRegister" quaternary type="info" class="flex" @click="$router.push({ path: '/register' })">
            {{ $t('login.registerButton') }}
          </NButton>
        </div>
      </NForm>
    </NCard>
  </div>
</template>

<style>
.login-container {
  padding: 20px;
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100vh;
  background-color: #f2f6ff;
}

.dark .login-container {
  background-color: rgb(43, 43, 43);
}

@media (min-width: 600px) {
  .login-card {
    width: auto;
    margin: 0px 10px;
  }
  .login-button {
    width: 100%;
  }
}

.login-card {
  margin: 20px;
  min-width: 400px;
}

.login-title {
  text-align: center;
  margin: 20px;
}
</style>
