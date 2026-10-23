<script setup lang="ts">
defineProps<{ name: 'wick' | 'settings' | 'plus' | 'back' | 'chevron' | 'check' | 'close' | 'alert' | 'sun' | 'moon' | 'system' | 'display' | 'battery' | 'bolt' | 'clock' | 'terminal' | 'app' | 'lid' | 'power' | 'external'; size?: number }>()
</script>

<template>
  <svg :width="size ?? 18" :height="size ?? 18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <template v-if="name === 'wick'"><path d="M12 3.5a8.5 8.5 0 1 1-7.4 4.3" stroke-width="2.6"/><circle cx="12" cy="12" r="2.2" fill="currentColor" stroke="none"/></template>
    <template v-else-if="name === 'settings'"><path d="m9 3-.7 2.2-2 .9-2.1-.5L2.7 8.2l1.5 1.7v2.3l-1.5 1.7 1.5 2.6 2.1-.5 2 .9L9 19h3l.7-2.1 2-.9 2.1.5 1.5-2.6-1.5-1.7V9.9l1.5-1.7-1.5-2.6-2.1.5-2-.9L12 3z" transform="translate(1.5 1)"/><circle cx="12" cy="12" r="3"/></template>
    <path v-else-if="name === 'plus'" d="M12 5v14M5 12h14"/>
    <path v-else-if="name === 'back'" d="m14 6-6 6 6 6M8 12h12"/>
    <path v-else-if="name === 'chevron'" d="m8 10 4 4 4-4"/>
    <path v-else-if="name === 'check'" d="m5 12 4 4L19 6"/>
    <path v-else-if="name === 'close'" d="m6 6 12 12M6 18 18 6"/>
    <template v-else-if="name === 'alert'"><circle cx="12" cy="12" r="9"/><path d="M12 7v6m0 4h.01"/></template>
    <template v-else-if="name === 'sun'"><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1 1m12 12 1 1M5 19l1-1M18 6l1-1"/></template>
    <path v-else-if="name === 'moon'" d="M20 14a8 8 0 0 1-10-10A9 9 0 1 0 20 14Z"/>
    <template v-else-if="name === 'system'"><rect x="3" y="4" width="18" height="13" rx="2"/><path d="M8 21h8m-4-4v4"/></template>
    <template v-else-if="name === 'display'"><rect x="3" y="4" width="18" height="12" rx="2"/><path d="M9 20h6"/></template>
    <template v-else-if="name === 'battery'"><rect x="2.5" y="7" width="17" height="10" rx="2.5"/><path d="M22 11v2"/></template>
    <path v-else-if="name === 'bolt'" d="M13 2 4 14h7l-1 8 9-12h-7z"/>
    <template v-else-if="name === 'clock'"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></template>
    <template v-else-if="name === 'terminal'"><rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="m7.5 9.5 3 2.5-3 2.5M13 15h3.5"/></template>
    <template v-else-if="name === 'app'"><rect x="4" y="4" width="16" height="16" rx="4.5"/></template>
    <template v-else-if="name === 'lid'"><path d="M4 17h16l-1.5-11h-13z"/><path d="M2 20h20"/></template>
    <template v-else-if="name === 'power'"><path d="M12 3v8"/><path d="M6.4 6.6a8 8 0 1 0 11.2 0"/></template>
    <template v-else-if="name === 'external'"><path d="M14 3h7v7M21 3l-11 11M10 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-5"/></template>
  </svg>
</template>
