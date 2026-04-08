import { computed, onMounted, ref } from 'vue'
import { getLoginConfig } from '@/api/openness'
import { logError } from '@/utils/logger'

const allowWeakPassword = ref(false)
const policyLoaded = ref(false)
let loadPasswordPolicyPromise: Promise<void> | null = null

export function usePublicPasswordPolicy() {
  async function loadPasswordPolicy() {
    if (policyLoaded.value)
      return
    if (loadPasswordPolicyPromise)
      return loadPasswordPolicyPromise

    loadPasswordPolicyPromise = (async () => {
      try {
        const res = await getLoginConfig<Openness.open.LoginVcodeResponse>()
        if (res.code === 0)
          allowWeakPassword.value = !!res.data?.passwordPolicy?.allowWeakPassword
      }
      catch (error) {
        logError('获取密码策略失败', error)
        allowWeakPassword.value = false
      }
      finally {
        policyLoaded.value = true
        loadPasswordPolicyPromise = null
      }
    })()

    try {
      await loadPasswordPolicyPromise
    }
    catch {
      // Errors are already handled in the shared loader above.
    }
  }

  const showStrongPasswordHint = computed(() => policyLoaded.value && !allowWeakPassword.value)

  onMounted(() => {
    void loadPasswordPolicy()
  })

  return {
    allowWeakPassword,
    loadPasswordPolicy,
    policyLoaded,
    showStrongPasswordHint,
  }
}
