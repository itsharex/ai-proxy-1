import { defineStore } from 'pinia'
import { api } from '../api'
import type { Provider } from '../types'

export const useProvidersStore = defineStore('providers', {
  state: () => ({
    providers: [] as Provider[],
    loading: false,
  }),
  actions: {
    async fetchProviders() {
      this.loading = true
      try {
        this.providers = await api<Provider[]>('/api/providers')
      } finally {
        this.loading = false
      }
    },
  },
})
