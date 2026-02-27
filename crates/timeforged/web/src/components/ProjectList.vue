<script setup lang="ts">
import type { CategorySummary } from '../api'

const props = defineProps<{ projects: CategorySummary[] }>()

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

const projectColors = [
  '#e8933a', '#2dd4bf', '#8b8b9e', '#fb7185', '#60a5fa',
  '#f0a54e', '#34d399', '#a78bfa', '#f472b6', '#38bdf8',
]

function getColor(index: number): string {
  return projectColors[index % projectColors.length]
}
</script>

<template>
  <div v-if="projects.length === 0" class="tf-empty">No projects tracked yet</div>
  <div v-else class="tf-project-list">
    <div
      v-for="(p, i) in projects"
      :key="p.name"
      class="tf-project-item"
      :style="{ animationDelay: `${i * 40}ms` }"
    >
      <div class="tf-project-row">
        <div class="tf-project-left">
          <span class="tf-project-dot" :style="{ background: getColor(i) }"></span>
          <span class="tf-project-name">{{ p.name }}</span>
        </div>
        <div class="tf-project-meta">
          <span class="tf-project-time">{{ formatDuration(p.total_seconds) }}</span>
          <span class="tf-project-pct">{{ p.percent.toFixed(0) }}%</span>
        </div>
      </div>
      <div class="tf-project-bar-track">
        <div
          class="tf-project-bar-fill"
          :style="{ width: Math.max(p.percent, 1) + '%', background: getColor(i) }"
        ></div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tf-empty {
  padding: 16px 0;
  font-size: 13px;
  color: var(--tf-text-tertiary);
}

.tf-project-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.tf-project-item {
  animation: tf-fade-up 0.35s cubic-bezier(0.16, 1, 0.3, 1) both;
}

.tf-project-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.tf-project-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.tf-project-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.tf-project-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--tf-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tf-project-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.tf-project-time {
  font-family: var(--tf-font-mono);
  font-size: 12px;
  color: var(--tf-text-secondary);
}

.tf-project-pct {
  font-family: var(--tf-font-mono);
  font-size: 11px;
  color: var(--tf-text-tertiary);
  width: 32px;
  text-align: right;
}

.tf-project-bar-track {
  height: 4px;
  background: rgba(255, 255, 255, 0.04);
  border-radius: 4px;
  overflow: hidden;
}

.tf-project-bar-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.6s cubic-bezier(0.16, 1, 0.3, 1);
  opacity: 0.8;
}

@keyframes tf-fade-up {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
