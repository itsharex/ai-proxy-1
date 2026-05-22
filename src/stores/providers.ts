import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { Provider } from '../types'

interface CreateProviderPayload {
  [key: string]: unknown
  name: string
  base_url: string
  auth_type: string
  auth_header: string
  endpoints: { format: string; path: string }[]
}

export const useProvidersStore = defineStore('providers', {
  state: () => ({
    providers: [] as Provider[],
    loading: false,
  }),
  actions: {
    async fetchProviders() {
      this.loading = true
      try {
        this.providers = await invoke<Provider[]>('get_providers')
      } finally {
        this.loading = false
      }
    },
    async createProvider(data: CreateProviderPayload) {
      await invoke('create_provider', data)
      await this.fetchProviders()
    },
    async deleteProvider(id: string) {
      await invoke('delete_provider', { id })
      await this.fetchProviders()
    },
    async createApiKey(providerId: string, label: string, plaintextKey: string) {
      await invoke('create_api_key', { providerId, label, plaintextKey })
      await this.fetchProviders()
    },
    async deleteApiKey(id: string) {
      await invoke('delete_api_key', { id })
      await this.fetchProviders()
    },
  },
})
