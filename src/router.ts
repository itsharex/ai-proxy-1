import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  { path: '/', name: 'Dashboard', component: () => import('./views/Dashboard.vue') },
  { path: '/providers', name: 'Providers', component: () => import('./views/Providers.vue') },
  { path: '/models', name: 'Models', component: () => import('./views/Models.vue') },
  { path: '/logs', name: 'Logs', component: () => import('./views/Logs.vue') },
  { path: '/statistics', name: 'Statistics', component: () => import('./views/Statistics.vue') },
  { path: '/test', name: 'Test', component: () => import('./views/Test.vue') },
  { path: '/rules', name: 'Rules', component: () => import('./views/Rules.vue') },
  { path: '/settings', name: 'Settings', component: () => import('./views/Settings.vue') },
]

export default createRouter({ history: createWebHashHistory(), routes })
