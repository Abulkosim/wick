<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watchEffect } from 'vue'
import AppIcon from './components/AppIcon.vue'

const query = new URLSearchParams(location.search)
const demo = query.get('demo') === '1'
const desktop = typeof window.__TAURI__?.core.invoke === 'function'
const media = window.matchMedia('(prefers-color-scheme: dark)')
const systemDark = ref(media.matches)
const theme = query.get('theme')
const dark = computed(() => theme === 'dark' || (theme !== 'light' && systemDark.value))

watchEffect(() => {
  document.documentElement.dataset.theme = dark.value ? 'dark' : 'light'
  document.documentElement.style.colorScheme = dark.value ? 'dark' : 'light'
})

function syncSystemTheme(event: MediaQueryListEvent): void { systemDark.value = event.matches }
onMounted(() => media.addEventListener('change', syncSystemTheme))
onUnmounted(() => media.removeEventListener('change', syncSystemTheme))
</script>

<template>
  <div :class="desktop && !demo ? 'desktop-stage' : 'preview-stage'">
    <div class="app-shell">
      <div class="popover-arrow" aria-hidden="true"/>
      <div class="popover">
        <header class="app-header">
          <span class="wordmark"><AppIcon name="wick" :size="17"/>wick</span>
        </header>
        <main class="popover-body"><p class="muted">Your Mac sleeps as usual.</p></main>
        <div v-if="demo" class="preview-ribbon"><span class="preview-dot"/>Interface preview<span>Sample data</span></div>
      </div>
    </div>
  </div>
</template>
