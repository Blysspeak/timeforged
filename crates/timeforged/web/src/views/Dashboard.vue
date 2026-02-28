<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { api } from '../api'
import type { Summary, Session } from '../api'
import TimeChart from '../components/TimeChart.vue'
import ProjectList from '../components/ProjectList.vue'
import LanguageList from '../components/LanguageList.vue'
import SessionList from '../components/SessionList.vue'

const summary = ref<Summary | null>(null)
const sessions = ref<Session[]>([])
const loading = ref(true)
const error = ref<string | null>(null)

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

const todayTotal = computed(() => {
  if (!summary.value) return '0m'
  return formatDuration(summary.value.total_seconds)
})

const topProject = computed(() => {
  if (!summary.value || summary.value.projects.length === 0) return '—'
  return summary.value.projects[0].name
})

const topProjectPercent = computed(() => {
  if (!summary.value || summary.value.projects.length === 0) return 0
  return Math.round(summary.value.projects[0].percent)
})

const sessionCount = computed(() => {
  return sessions.value.length
})

const totalEvents = computed(() => {
  return sessions.value.reduce((sum, s) => sum + s.event_count, 0)
})

async function loadData() {
  const isInitial = !summary.value
  if (isInitial) loading.value = true
  error.value = null
  try {
    const now = new Date()
    const weekAgo = new Date(now)
    weekAgo.setDate(weekAgo.getDate() - 7)

    const [summaryData, sessionsData] = await Promise.all([
      api.summary(weekAgo.toISOString(), now.toISOString()),
      api.sessions(weekAgo.toISOString(), now.toISOString()),
    ])

    summary.value = summaryData
    sessions.value = sessionsData.slice(0, 10)
  } catch (e: any) {
    if (isInitial) error.value = e.message || 'Failed to load data'
  } finally {
    loading.value = false
  }
}

let pollTimer: ReturnType<typeof setInterval>

onMounted(() => {
  loadData()
  pollTimer = setInterval(loadData, 30_000)
})

onUnmounted(() => {
  clearInterval(pollTimer)
})
</script>

<template>
  <div class="tf-dashboard">
    <!-- Header -->
    <header class="tf-page-header">
      <div>
        <h2 class="tf-page-title">Dashboard</h2>
        <p class="tf-page-subtitle">Last 7 days overview</p>
      </div>
      <button @click="loadData" class="tf-refresh-btn" :class="{ 'tf-spinning': loading }">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/>
          <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
        </svg>
      </button>
    </header>

    <!-- Error -->
    <div v-if="error" class="tf-error tf-animate">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>
      {{ error }}
    </div>

    <!-- Loading skeleton -->
    <div v-if="loading" class="tf-skeleton-grid">
      <div class="tf-loading-shimmer" style="height:110px"></div>
      <div class="tf-loading-shimmer" style="height:110px"></div>
      <div class="tf-loading-shimmer" style="height:110px"></div>
      <div class="tf-loading-shimmer" style="height:110px"></div>
      <div class="tf-loading-shimmer tf-skeleton-wide" style="height:280px"></div>
    </div>

    <template v-else-if="summary">
      <!-- Stat cards -->
      <div class="tf-stats-grid tf-animate" style="animation-delay: 0.05s">
        <div class="tf-stat-card tf-stat-primary">
          <div class="tf-stat-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
          </div>
          <div class="tf-stat-content">
            <span class="tf-stat-label">Total Time</span>
            <span class="tf-stat-value">{{ todayTotal }}</span>
          </div>
          <div class="tf-stat-glow"></div>
        </div>

        <div class="tf-stat-card">
          <div class="tf-stat-icon tf-stat-icon-teal">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
          </div>
          <div class="tf-stat-content">
            <span class="tf-stat-label">Top Project</span>
            <span class="tf-stat-value tf-stat-value-sm">{{ topProject }}</span>
          </div>
          <span class="tf-stat-badge" v-if="topProjectPercent > 0">{{ topProjectPercent }}%</span>
        </div>

        <div class="tf-stat-card">
          <div class="tf-stat-icon tf-stat-icon-amber">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>
          </div>
          <div class="tf-stat-content">
            <span class="tf-stat-label">Sessions</span>
            <span class="tf-stat-value">{{ sessionCount }}</span>
          </div>
        </div>

        <div class="tf-stat-card">
          <div class="tf-stat-icon tf-stat-icon-rose">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/></svg>
          </div>
          <div class="tf-stat-content">
            <span class="tf-stat-label">Events</span>
            <span class="tf-stat-value">{{ totalEvents }}</span>
          </div>
        </div>
      </div>

      <!-- Chart -->
      <div class="tf-card tf-animate" style="animation-delay: 0.1s">
        <div class="tf-card-header">
          <h3 class="tf-card-title">Activity</h3>
          <span class="tf-card-badge">7 days</span>
        </div>
        <div class="tf-card-body">
          <TimeChart :days="summary.days" />
        </div>
      </div>

      <!-- Projects & Languages -->
      <div class="tf-split-grid tf-animate" style="animation-delay: 0.15s">
        <div class="tf-card">
          <div class="tf-card-header">
            <h3 class="tf-card-title">Projects</h3>
            <span class="tf-card-count">{{ summary.projects.length }}</span>
          </div>
          <div class="tf-card-body tf-card-body-list">
            <ProjectList :projects="summary.projects" />
          </div>
        </div>
        <div class="tf-card">
          <div class="tf-card-header">
            <h3 class="tf-card-title">Languages</h3>
            <span class="tf-card-count">{{ summary.languages.length }}</span>
          </div>
          <div class="tf-card-body tf-card-body-list">
            <LanguageList :languages="summary.languages" />
          </div>
        </div>
      </div>

      <!-- Sessions -->
      <div class="tf-card tf-animate" style="animation-delay: 0.2s">
        <div class="tf-card-header">
          <h3 class="tf-card-title">Recent Sessions</h3>
          <span class="tf-card-count">{{ sessions.length }}</span>
        </div>
        <div class="tf-card-body tf-card-body-flush">
          <SessionList :sessions="sessions" />
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.tf-dashboard {
  padding: 28px 32px 40px;
  max-width: 1200px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* ── Header ── */
.tf-page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.tf-page-title {
  font-family: var(--tf-font-mono);
  font-size: 22px;
  font-weight: 600;
  color: var(--tf-text-primary);
  letter-spacing: -0.03em;
}

.tf-page-subtitle {
  font-size: 13px;
  color: var(--tf-text-tertiary);
  margin-top: 2px;
}

.tf-refresh-btn {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  border: 1px solid var(--tf-border);
  background: var(--tf-bg-surface);
  color: var(--tf-text-secondary);
  cursor: pointer;
  transition: all var(--tf-transition);
}
.tf-refresh-btn:hover {
  color: var(--tf-text-primary);
  border-color: var(--tf-accent);
  background: var(--tf-accent-glow);
}
.tf-spinning svg {
  animation: spin 0.8s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

/* ── Error ── */
.tf-error {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  background: rgba(239, 68, 68, 0.06);
  border: 1px solid rgba(239, 68, 68, 0.15);
  border-radius: var(--tf-radius-sm);
  color: #fca5a5;
  font-size: 13px;
}

/* ── Skeleton ── */
.tf-skeleton-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
}
.tf-skeleton-wide {
  grid-column: 1 / -1;
}

/* ── Stat Cards ── */
.tf-stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 14px;
}

.tf-stat-card {
  background: var(--tf-bg-surface);
  border: 1px solid var(--tf-border);
  border-radius: var(--tf-radius);
  padding: 18px;
  display: flex;
  align-items: flex-start;
  gap: 14px;
  position: relative;
  overflow: hidden;
  transition: border-color var(--tf-transition);
}
.tf-stat-card:hover {
  border-color: #2a2a35;
}

.tf-stat-primary {
  border-color: rgba(232, 147, 58, 0.2);
  background: linear-gradient(135deg, rgba(232, 147, 58, 0.06) 0%, var(--tf-bg-surface) 60%);
}

.tf-stat-glow {
  position: absolute;
  top: -20px;
  right: -20px;
  width: 80px;
  height: 80px;
  background: radial-gradient(circle, rgba(232, 147, 58, 0.15) 0%, transparent 70%);
  pointer-events: none;
}

.tf-stat-icon {
  width: 40px;
  height: 40px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  background: var(--tf-accent-glow);
  color: var(--tf-accent);
  border: 1px solid rgba(232, 147, 58, 0.12);
}

.tf-stat-icon-teal {
  background: rgba(45, 212, 191, 0.08);
  color: #2dd4bf;
  border-color: rgba(45, 212, 191, 0.12);
}
.tf-stat-icon-amber {
  background: rgba(251, 191, 36, 0.08);
  color: #fbbf24;
  border-color: rgba(251, 191, 36, 0.12);
}
.tf-stat-icon-rose {
  background: rgba(251, 113, 133, 0.08);
  color: #fb7185;
  border-color: rgba(251, 113, 133, 0.12);
}

.tf-stat-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.tf-stat-label {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--tf-text-tertiary);
  font-weight: 500;
}

.tf-stat-value {
  font-family: var(--tf-font-mono);
  font-size: 22px;
  font-weight: 600;
  color: var(--tf-text-primary);
  letter-spacing: -0.03em;
  line-height: 1.1;
}

.tf-stat-value-sm {
  font-size: 17px;
  font-family: var(--tf-font-ui);
  font-weight: 600;
  letter-spacing: -0.01em;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tf-stat-badge {
  position: absolute;
  top: 14px;
  right: 14px;
  font-family: var(--tf-font-mono);
  font-size: 11px;
  font-weight: 500;
  color: #2dd4bf;
  background: rgba(45, 212, 191, 0.1);
  padding: 2px 8px;
  border-radius: 6px;
}

/* ── Card ── */
.tf-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--tf-border);
}

.tf-card-title {
  font-size: 13px;
  font-weight: 550;
  color: var(--tf-text-secondary);
  letter-spacing: 0.01em;
}

.tf-card-badge {
  font-family: var(--tf-font-mono);
  font-size: 11px;
  color: var(--tf-text-tertiary);
  background: var(--tf-bg-raised);
  padding: 3px 10px;
  border-radius: 6px;
  border: 1px solid var(--tf-border);
}

.tf-card-count {
  font-family: var(--tf-font-mono);
  font-size: 12px;
  color: var(--tf-text-tertiary);
}

.tf-card-body {
  padding: 20px;
}

.tf-card-body-list {
  padding: 8px 20px 16px;
}

.tf-card-body-flush {
  padding: 0;
}

/* ── Split Grid ── */
.tf-split-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}

@media (max-width: 1024px) {
  .tf-stats-grid { grid-template-columns: repeat(2, 1fr); }
  .tf-split-grid { grid-template-columns: 1fr; }
}
@media (max-width: 640px) {
  .tf-stats-grid { grid-template-columns: 1fr; }
  .tf-dashboard { padding: 20px 16px; }
}
</style>
