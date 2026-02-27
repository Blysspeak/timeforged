<script setup lang="ts">
import type { Session } from '../api'

const props = defineProps<{ sessions: Session[] }>()

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

function formatTime(iso: string): string {
  const d = new Date(iso)
  return d.toLocaleString('en', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  })
}

function durationClass(seconds: number): string {
  if (seconds >= 3600) return 'tf-dur-high'
  if (seconds >= 600) return 'tf-dur-mid'
  return 'tf-dur-low'
}
</script>

<template>
  <div v-if="sessions.length === 0" class="tf-session-empty">No sessions recorded</div>
  <div v-else class="tf-session-table">
    <div class="tf-session-thead">
      <span class="tf-session-th tf-col-time">Start</span>
      <span class="tf-session-th tf-col-time">End</span>
      <span class="tf-session-th tf-col-dur">Duration</span>
      <span class="tf-session-th tf-col-proj">Project</span>
      <span class="tf-session-th tf-col-events">Events</span>
    </div>
    <div
      v-for="(s, i) in sessions"
      :key="i"
      class="tf-session-row"
      :style="{ animationDelay: `${i * 30}ms` }"
    >
      <span class="tf-session-td tf-col-time tf-mono">{{ formatTime(s.start) }}</span>
      <span class="tf-session-td tf-col-time tf-mono">{{ formatTime(s.end) }}</span>
      <span class="tf-session-td tf-col-dur">
        <span class="tf-dur-pill" :class="durationClass(s.duration_seconds)">
          {{ formatDuration(s.duration_seconds) }}
        </span>
      </span>
      <span class="tf-session-td tf-col-proj">{{ s.project || '—' }}</span>
      <span class="tf-session-td tf-col-events tf-mono">{{ s.event_count }}</span>
    </div>
  </div>
</template>

<style scoped>
.tf-session-empty {
  padding: 24px 20px;
  font-size: 13px;
  color: var(--tf-text-tertiary);
}

.tf-session-table {
  display: flex;
  flex-direction: column;
}

.tf-session-thead {
  display: flex;
  padding: 10px 20px;
  border-bottom: 1px solid var(--tf-border);
}

.tf-session-th {
  font-size: 11px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--tf-text-tertiary);
}

.tf-session-row {
  display: flex;
  padding: 11px 20px;
  border-bottom: 1px solid var(--tf-border-subtle);
  transition: background var(--tf-transition);
  animation: tf-fade-up 0.3s cubic-bezier(0.16, 1, 0.3, 1) both;
}

.tf-session-row:last-child {
  border-bottom: none;
}

.tf-session-row:hover {
  background: rgba(255, 255, 255, 0.015);
}

.tf-session-td {
  font-size: 13px;
  color: var(--tf-text-secondary);
  display: flex;
  align-items: center;
}

.tf-mono {
  font-family: var(--tf-font-mono);
  font-size: 12px;
}

/* Column widths */
.tf-col-time { flex: 0 0 160px; }
.tf-col-dur { flex: 0 0 100px; }
.tf-col-proj { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.tf-col-events { flex: 0 0 70px; justify-content: flex-end; color: var(--tf-text-tertiary); }

/* Duration pill */
.tf-dur-pill {
  font-family: var(--tf-font-mono);
  font-size: 12px;
  font-weight: 500;
  padding: 2px 8px;
  border-radius: 5px;
  line-height: 1.4;
}

.tf-dur-high {
  color: #e8933a;
  background: rgba(232, 147, 58, 0.1);
}
.tf-dur-mid {
  color: #2dd4bf;
  background: rgba(45, 212, 191, 0.08);
}
.tf-dur-low {
  color: var(--tf-text-tertiary);
  background: rgba(255, 255, 255, 0.04);
}

@keyframes tf-fade-up {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
