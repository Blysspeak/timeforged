<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '../api'
import type { StatusResponse } from '../api'

const status = ref<StatusResponse | null>(null)
const statusError = ref<string | null>(null)

async function loadStatus() {
  statusError.value = null
  try {
    status.value = await api.status()
  } catch (e: any) {
    statusError.value = e.message
  }
}

onMounted(loadStatus)
</script>

<template>
  <div class="tf-settings">
    <header class="tf-page-header">
      <div>
        <h2 class="tf-page-title">Settings</h2>
        <p class="tf-page-subtitle">Daemon configuration</p>
      </div>
    </header>

    <!-- Daemon Status Card -->
    <div class="tf-card tf-animate" style="animation-delay: 0.05s">
      <div class="tf-card-header">
        <h3 class="tf-card-title">Daemon Status</h3>
        <div v-if="status" class="tf-key-status">
          <span class="tf-status-dot tf-status-ok"></span>
          <span class="tf-status-text">Online</span>
        </div>
        <div v-else-if="statusError" class="tf-key-status">
          <span class="tf-status-dot tf-status-err"></span>
          <span class="tf-status-text">Offline</span>
        </div>
      </div>
      <div class="tf-card-body">
        <div v-if="statusError" class="tf-status-error">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
          Cannot reach daemon: {{ statusError }}
        </div>

        <div v-else-if="status" class="tf-status-grid">
          <div class="tf-status-item">
            <span class="tf-status-label">Version</span>
            <span class="tf-status-value tf-mono">{{ status.version }}</span>
          </div>
          <div class="tf-status-item">
            <span class="tf-status-label">Status</span>
            <span class="tf-status-value tf-status-ok-text">{{ status.status }}</span>
          </div>
          <div class="tf-status-item">
            <span class="tf-status-label">Users</span>
            <span class="tf-status-value tf-mono">{{ status.user_count }}</span>
          </div>
          <div class="tf-status-item">
            <span class="tf-status-label">Total Events</span>
            <span class="tf-status-value tf-mono">{{ status.event_count }}</span>
          </div>
        </div>

        <div v-else class="tf-status-loading">
          <div class="tf-loading-shimmer" style="height: 16px; width: 120px;"></div>
        </div>
      </div>
    </div>

    <!-- About Card -->
    <div class="tf-card tf-animate" style="animation-delay: 0.1s">
      <div class="tf-card-header">
        <h3 class="tf-card-title">About</h3>
      </div>
      <div class="tf-card-body">
        <div class="tf-about">
          <p class="tf-about-text">
            TimeForged is a self-hosted time tracking daemon. It captures editor activity,
            computes sessions, and generates reports — all running locally on your machine.
          </p>
          <div class="tf-about-links">
            <span class="tf-about-item">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"/><circle cx="12" cy="10" r="3"/></svg>
              127.0.0.1:6175
            </span>
            <span class="tf-about-item">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="2" width="20" height="8" rx="2" ry="2"/><rect x="2" y="14" width="20" height="8" rx="2" ry="2"/><line x1="6" y1="6" x2="6.01" y2="6"/><line x1="6" y1="18" x2="6.01" y2="18"/></svg>
              SQLite storage
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tf-settings {
  padding: 28px 32px 40px;
  max-width: 640px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.tf-page-header { margin-bottom: 4px; }
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

.tf-key-status {
  display: flex;
  align-items: center;
  gap: 6px;
}
.tf-status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
}
.tf-status-ok { background: #34d399; box-shadow: 0 0 6px rgba(52, 211, 153, 0.4); }
.tf-status-err { background: #fb7185; box-shadow: 0 0 6px rgba(251, 113, 133, 0.4); }
.tf-status-text { font-size: 12px; color: var(--tf-text-tertiary); }

.tf-status-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1px;
  background: var(--tf-border);
  border-radius: var(--tf-radius-sm);
  overflow: hidden;
}
.tf-status-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 14px 16px;
  background: var(--tf-bg-surface);
}
.tf-status-label {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--tf-text-tertiary);
  font-weight: 500;
}
.tf-status-value {
  font-size: 15px;
  font-weight: 600;
  color: var(--tf-text-primary);
}
.tf-mono { font-family: var(--tf-font-mono); }
.tf-status-ok-text { color: #34d399; }

.tf-status-error {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  background: rgba(251, 113, 133, 0.06);
  border: 1px solid rgba(251, 113, 133, 0.15);
  border-radius: var(--tf-radius-sm);
  color: #fca5a5;
  font-size: 13px;
}

.tf-status-loading { padding: 8px 0; }

/* About */
.tf-about {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.tf-about-text {
  font-size: 13px;
  color: var(--tf-text-secondary);
  line-height: 1.6;
}
.tf-about-links {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
}
.tf-about-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-family: var(--tf-font-mono);
  color: var(--tf-text-tertiary);
}
</style>
