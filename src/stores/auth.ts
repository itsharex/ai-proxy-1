import { defineStore } from 'pinia'
import { api, ApiError, clearStoredToken } from '../api'
import { isTauri } from '../utils/env'

export interface AuthUser {
  id: string
  username: string
  role: string
}

interface LoginResponse {
  token: string
  expires_in: number
  user: AuthUser
}

const TOKEN_KEY = 'ai_proxy_token'

export const useAuthStore = defineStore('auth', {
  state: () => ({
    user: null as AuthUser | null,
    ready: false,
  }),
  getters: {
    isAuthenticated(): boolean {
      if (isTauri) return true
      return !!localStorage.getItem(TOKEN_KEY)
    },
  },
  actions: {
    markReady() {
      this.ready = true
    },

    async login(username: string, password: string) {
      const data = await api<LoginResponse>('/api/auth/login', {
        method: 'POST',
        body: JSON.stringify({ username, password }),
      })
      if (!isTauri) {
        localStorage.setItem(TOKEN_KEY, data.token)
      }
      this.user = data.user
      return data
    },

    async logout() {
      if (!isTauri && this.isAuthenticated) {
        try {
          await api('/api/auth/logout', { method: 'POST' })
        } catch {
          // ignore — token will be cleared locally anyway
        }
      }
      clearStoredToken()
      this.user = null
    },

    async fetchMe() {
      if (isTauri) {
        this.markReady()
        return null
      }
      if (!localStorage.getItem(TOKEN_KEY)) {
        this.markReady()
        return null
      }
      try {
        const user = await api<AuthUser>('/api/auth/me')
        this.user = user
        this.markReady()
        return user
      } catch (err) {
        if (err instanceof ApiError && err.status === 401) {
          clearStoredToken()
          this.user = null
        }
        this.markReady()
        return null
      }
    },
  },
})
