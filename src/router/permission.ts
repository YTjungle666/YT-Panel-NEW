import type { Router } from 'vue-router'
import { useAuthStore } from '@/store/modules/auth'
import { VisitMode } from '@/enums/auth'

export function setupPageGuard(router: Router) {
  router.beforeEach(async (to, from, next) => {
    const authStore = useAuthStore()

    // If not logged in and trying to access protected routes, redirect to login
    const publicRoutes = ['/login', '/register', '/404', '/500']
    const isPublicRoute = publicRoutes.includes(to.path)

    const isLoggedIn = authStore.visitMode === VisitMode.VISIT_MODE_LOGIN && !!authStore.userInfo?.id

    // If accessing protected route and not logged in, redirect to login
    if (!isPublicRoute && !isLoggedIn) {
      next({ name: 'login' })
      return
    }

    if (isLoggedIn && authStore.userInfo?.mustChangePassword && to.path !== '/home' && to.path !== '/login') {
      next({ path: '/home' })
      return
    }

    next()
  })
}
