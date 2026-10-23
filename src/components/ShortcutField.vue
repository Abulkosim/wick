<script setup lang="ts">
import { nextTick, ref } from 'vue'
import AppIcon from './AppIcon.vue'
import { formatShortcut, recordShortcut } from '../lib/shortcut'

const props = defineProps<{ modelValue: string | null; disabled: boolean; labelledby: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: string | null] }>()
const recording = ref(false)
const error = ref<string | null>(null)
const button = ref<HTMLButtonElement | null>(null)

async function start(): Promise<void> {
  recording.value = true
  error.value = null
  await nextTick()
  button.value?.focus()
}

function stop(): void {
  recording.value = false
}

function onKeydown(event: KeyboardEvent): void {
  if (!recording.value) return
  event.preventDefault()
  event.stopPropagation()
  if (event.code === 'Escape' && !event.metaKey && !event.ctrlKey && !event.altKey) return stop()
  if ((event.code === 'Backspace' || event.code === 'Delete') && !event.metaKey && !event.ctrlKey && !event.altKey) {
    stop()
    emit('update:modelValue', null)
    return
  }
  const result = recordShortcut(event)
  if (result.kind === 'pending') return
  if (result.kind === 'error') { error.value = result.message; return }
  error.value = null
  stop()
  if (result.value !== props.modelValue) emit('update:modelValue', result.value)
}
</script>

<template>
  <div class="shortcut-field">
    <div class="shortcut-controls">
    <button ref="button" class="shortcut-button" :class="{ recording }" type="button" :aria-labelledby="labelledby" :aria-describedby="error ? 'shortcut-error' : undefined" :disabled="disabled" @click="recording ? stop() : start()" @keydown="onKeydown" @blur="stop">
      <span v-if="recording">Type shortcut…</span>
      <kbd v-else-if="modelValue">{{ formatShortcut(modelValue) }}</kbd>
      <span v-else>Record shortcut</span>
    </button>
    <button v-if="modelValue && !recording" class="icon-button shortcut-clear" type="button" aria-label="Remove shortcut" :disabled="disabled" @click="emit('update:modelValue', null)"><AppIcon name="close" :size="13"/></button>
    </div>
    <p v-if="error" id="shortcut-error" class="field-error shortcut-error" role="alert">{{ error }}</p>
  </div>
</template>
