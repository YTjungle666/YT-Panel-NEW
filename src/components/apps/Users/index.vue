<script lang="ts" setup>
import { h, onMounted, reactive, ref } from 'vue'
import { NAlert, NButton, NCard, NCheckbox, NDataTable, NDropdown, NTag, NText, useDialog, useMessage } from 'naive-ui'
import type { DataTableColumns, PaginationProps } from 'naive-ui'
import EditUser from './EditUser/index.vue'
import { deletes as usersDeletes, getList as usersGetList } from '@/api/panel/users'
import { getSystemSetting, setSystemSettings } from '@/api/system/systemSetting'
import { SvgIcon } from '@/components/common'
import { useAuthStore } from '@/store'
import { t } from '@/locales'
import { AdminAuthRole } from '@/enums/admin'

const SECURITY_PASSWORD_POLICY_KEY = 'security_password_policy'
const message = useMessage()
const authStore = useAuthStore()
const tableIsLoading = ref<boolean>(false)
const editUserDialogShow = ref<boolean>(false)
const keyWord = ref<string>()
const editUserUserInfo = ref<User.Info>()
const dialog = useDialog()
const allowWeakPassword = ref(false)
const savingPasswordPolicy = ref(false)

const createColumns = ({
  update,
}: {
  update: (row: User.Info) => void
}): DataTableColumns<User.Info> => {
  return [
    {
      title: t('common.username'),
      key: 'username',
      render(row: User.Info) {
        const renderTags = []
        if (row.username === authStore.userInfo?.username) {
          renderTags.push(h(NTag, { type: 'success', bordered: false, size: 'small' }, { default: () => t('adminSettingUsers.currentUseUsername') }))
        }

        if (renderTags.length === 0) {
          return row.username
        }

        return h('div', { class: 'flex items-center space-x-2' }, [
          h('span', row.username),
          ...renderTags
        ])
      },
    },
    {
      title: t('common.nikeName'),
      key: 'name',
    },
    {
      title: t('adminSettingUsers.role'),
      key: 'role',
      render(row) {
        switch (row.role) {
          case AdminAuthRole.admin:
            return h(NTag, { type: 'info' }, t('common.role.admin'))
          case AdminAuthRole.regularUser:
            return h(NTag, t('common.role.regularUser'))
          default:
            return '-'
        }
      },
    },
    {
      title: t('common.action'),
      key: '',
      render(row) {
        const btn = h(
          NButton,
          {
            strong: true,
            tertiary: true,
            size: 'small',
          },
          {
            default() {
              return h(
                SvgIcon, {
                  icon: 'mingcute:more-1-fill',
                },
              )
            },
          },
        )

        return h(NDropdown, {
          trigger: 'click',
          onSelect(key: string | number) {
            switch (key) {
              case 'update':
                update(row)
                break
              case 'delete':
                dialog.warning({
                  title: t('common.warning'),
                  content: t('adminSettingUsers.deletePromptContent', { name: row.name, username: row.username }),
                  positiveText: t('common.confirm'),
                  negativeText: t('common.cancel'),
                  onPositiveClick: () => {
                    deletes([row.id as number])
                  },
                })
                break

              default:
                break
            }
          },
          options: [
            {
              label: t('common.edit'),
              key: 'update',
            },
            {
              label: t('common.delete'),
              key: 'delete',
            },
          ],
        }, { default: () => btn })
      },
    },
  ]
}

const userList = ref<User.Info[]>()

const columns = createColumns({
  update(row: User.Info) {
    editUserUserInfo.value = row
    editUserDialogShow.value = true
  },
})
const pagination = reactive({
  page: 1,
  showSizePicker: true,
  pageSizes: [10, 30, 50, 100, 200],
  pageSize: 10,
  itemCount: 0,
  onChange: (page: number) => {
    pagination.page = page
    getList(null)
  },
  onUpdatePageSize: (pageSize: number) => {
    pagination.pageSize = pageSize
    pagination.page = 1
    getList(null)
  },
  prefix(item: PaginationProps) {
    return t('adminSettingUsers.userCountText', { count: item.itemCount })
  },
})

function handlePageChange(page: number) {
  getList(page)
}

// 添加
function handleAdd() {
  editUserDialogShow.value = true
  editUserUserInfo.value = {}
}

function handelDone() {
  editUserDialogShow.value = false
  message.success(t('common.success'))
  getList(null)
}

async function getList(page: number | null) {
  tableIsLoading.value = true
  const req: AdminUserManage.GetListRequest = {
    page: page || pagination.page,
    limit: pagination.pageSize,
  }
  if (keyWord.value !== '')
    req.keyWord = keyWord.value

  const { data } = await usersGetList<Common.ListResponse<User.Info[]>>(req)
  pagination.itemCount = data.count
  if (data.list)
    userList.value = data.list
  tableIsLoading.value = false
}

async function deletes(ids: number[]) {
  const { code } = await usersDeletes(ids)
  if (code === 0) {
    message.success(t('common.deleteSuccess'))
    getList(null)
  }
}

async function loadPasswordPolicy() {
  try {
    const { data } = await getSystemSetting<{ configValue: string }>(SECURITY_PASSWORD_POLICY_KEY)
    const parsed = data?.configValue ? JSON.parse(data.configValue) : {}
    allowWeakPassword.value = !!parsed.allowWeakPassword
  }
  catch {
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
    message.success(t('common.saveSuccess'))
  }
  finally {
    savingPasswordPolicy.value = false
  }
}

onMounted(() => {
  loadPasswordPolicy()
  getList(null)
})
</script>

<template>
  <div class="overflow-auto pt-2">
    <NAlert type="info" :bordered="false">
      {{ $t('adminSettingUsers.alertText') }}
    </NAlert>
    <NCard class="my-[10px]" :bordered="false" embedded>
      <div class="mb-4">
        <div class="font-medium">
          {{ t('apps.settings.passwordSecurity') }}
        </div>
        <NText depth="3">
          {{ t('apps.settings.passwordSecurityHint') }}
        </NText>
      </div>
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <div class="font-medium">
            {{ t('apps.settings.allowWeakPassword') }}
          </div>
          <NText depth="3">
            {{ t('apps.settings.allowWeakPasswordHint') }}
          </NText>
        </div>
        <div class="flex items-center gap-3">
          <NCheckbox v-model:checked="allowWeakPassword">
            {{ t('apps.settings.allowWeakPassword') }}
          </NCheckbox>
          <NButton type="primary" size="small" :loading="savingPasswordPolicy" @click="handleSavePasswordPolicy">
            {{ $t('common.save') }}
          </NButton>
        </div>
      </div>
    </NCard>
    <div class="my-[10px]">
      <NButton type="primary" size="small" ghost @click="handleAdd">
        {{ $t('common.add') }}
      </NButton>
    </div>

    <NDataTable
      :columns="columns"
      :data="userList"
      :pagination="pagination"
      :bordered="false"
      :loading="tableIsLoading"
      :remote="true"

      @update:page="handlePageChange"
    />
    <EditUser v-model:visible="editUserDialogShow" :user-info="editUserUserInfo" @done="handelDone" />
  </div>
</template>
