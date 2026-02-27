<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const currentPath = computed(() => route.path)

const navItems = [
  { path: '/', label: 'Dashboard', icon: 'grid' },
  { path: '/settings', label: 'Settings', icon: 'settings' },
]
</script>

<template>
  <div class="tf-app">
    <!-- Sidebar -->
    <aside class="tf-sidebar">
      <!-- Logo -->
      <div class="tf-sidebar-header">
        <div class="tf-logo">
          <img src="/logo.png" alt="TimeForged" class="tf-logo-img" />
        </div>
      </div>

      <!-- Nav -->
      <nav class="tf-nav">
        <router-link
          v-for="item in navItems"
          :key="item.path"
          :to="item.path"
          class="tf-nav-item"
          :class="{ 'tf-nav-active': currentPath === item.path }"
        >
          <span class="tf-nav-indicator" v-if="currentPath === item.path"></span>
          <svg v-if="item.icon === 'grid'" class="tf-nav-icon" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="7" height="7" rx="1.5"/>
            <rect x="14" y="3" width="7" height="7" rx="1.5"/>
            <rect x="3" y="14" width="7" height="7" rx="1.5"/>
            <rect x="14" y="14" width="7" height="7" rx="1.5"/>
          </svg>
          <svg v-if="item.icon === 'settings'" class="tf-nav-icon" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
          </svg>
          <span class="tf-nav-label">{{ item.label }}</span>
        </router-link>
      </nav>

      <!-- Footer -->
      <div class="tf-sidebar-footer">
        <span class="tf-version">v0.1.0</span>
      </div>
    </aside>

    <!-- Main -->
    <main class="tf-main">
      <router-view />
    </main>
  </div>
</template>

<style>
@import "tailwindcss";

/* ── Design Tokens ── */
:root {
  --tf-bg-deep: #09090b;
  --tf-bg-surface: #0f0f12;
  --tf-bg-raised: #16161a;
  --tf-bg-hover: #1c1c22;
  --tf-border: #1e1e26;
  --tf-border-subtle: #161619;
  --tf-text-primary: #f0f0f3;
  --tf-text-secondary: #8b8b9e;
  --tf-text-tertiary: #55556a;
  --tf-accent: #e8933a;
  --tf-accent-dim: #d4802e;
  --tf-accent-glow: rgba(232, 147, 58, 0.12);
  --tf-accent-glow-strong: rgba(232, 147, 58, 0.25);
  --tf-font-ui: 'DM Sans', -apple-system, sans-serif;
  --tf-font-mono: 'JetBrains Mono', 'SF Mono', monospace;
  --tf-radius: 12px;
  --tf-radius-sm: 8px;
  --tf-sidebar-w: 210px;
  --tf-transition: 180ms cubic-bezier(0.4, 0, 0.2, 1);
}

*, *::before, *::after { box-sizing: border-box; }

body {
  font-family: var(--tf-font-ui);
  background: var(--tf-bg-deep);
  color: var(--tf-text-primary);
  -webkit-font-smoothing: antialiased;
  margin: 0;
}

/* ── App Shell ── */
.tf-app {
  display: flex;
  height: 100vh;
  overflow: hidden;
  background: var(--tf-bg-deep);
}

/* ── Sidebar ── */
.tf-sidebar {
  width: var(--tf-sidebar-w);
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--tf-bg-surface);
  border-right: 1px solid var(--tf-border);
  position: relative;
}

.tf-sidebar::after {
  content: '';
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 1px;
  background: linear-gradient(180deg, transparent, var(--tf-accent-glow) 40%, var(--tf-accent-glow) 60%, transparent);
  pointer-events: none;
}

/* ── Logo ── */
.tf-sidebar-header {
  padding: 24px 18px 20px;
  border-bottom: 1px solid var(--tf-border);
}

.tf-logo {
  display: flex;
  align-items: center;
  justify-content: center;
}

.tf-logo-img {
  height: 150px;
  width: auto;
  object-fit: contain;
  filter: brightness(0.95);
  transition: filter var(--tf-transition);
}

.tf-logo:hover .tf-logo-img {
  filter: brightness(1.05);
}

/* ── Navigation ── */
.tf-nav {
  flex: 1;
  padding: 14px 10px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.tf-nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border-radius: var(--tf-radius-sm);
  font-size: 13px;
  font-weight: 450;
  color: var(--tf-text-secondary);
  text-decoration: none;
  transition: all var(--tf-transition);
  position: relative;
}

.tf-nav-item:hover {
  color: var(--tf-text-primary);
  background: var(--tf-bg-hover);
}

.tf-nav-active {
  color: var(--tf-text-primary) !important;
  background: var(--tf-accent-glow) !important;
}

.tf-nav-icon {
  opacity: 0.55;
  transition: all var(--tf-transition);
  flex-shrink: 0;
}

.tf-nav-active .tf-nav-icon {
  opacity: 1;
  color: var(--tf-accent);
}

.tf-nav-indicator {
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 16px;
  background: var(--tf-accent);
  border-radius: 0 3px 3px 0;
}

.tf-nav-label {
  line-height: 1;
}

/* ── Footer ── */
.tf-sidebar-footer {
  padding: 14px 18px;
  border-top: 1px solid var(--tf-border);
}

.tf-version {
  font-family: var(--tf-font-mono);
  font-size: 10px;
  letter-spacing: 0.03em;
  color: var(--tf-text-tertiary);
  background: var(--tf-bg-raised);
  padding: 3px 8px;
  border-radius: 5px;
}

/* ── Main ── */
.tf-main {
  flex: 1;
  overflow-y: auto;
  background: var(--tf-bg-deep);
}

.tf-main::-webkit-scrollbar { width: 6px; }
.tf-main::-webkit-scrollbar-track { background: transparent; }
.tf-main::-webkit-scrollbar-thumb { background: var(--tf-border); border-radius: 3px; }
.tf-main::-webkit-scrollbar-thumb:hover { background: #2a2a35; }

/* ── Shared Card ── */
.tf-card {
  background: var(--tf-bg-surface);
  border: 1px solid var(--tf-border);
  border-radius: var(--tf-radius);
  position: relative;
  overflow: hidden;
}

.tf-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(255,255,255,0.04) 50%, transparent);
  pointer-events: none;
}

.tf-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  border-bottom: 1px solid var(--tf-border);
}

.tf-card-title {
  font-size: 13px;
  font-weight: 550;
  color: var(--tf-text-secondary);
  letter-spacing: 0.01em;
}

.tf-card-body {
  padding: 20px;
}

/* ── Animations ── */
@keyframes tf-fade-up {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}

.tf-animate {
  animation: tf-fade-up 0.4s cubic-bezier(0.16, 1, 0.3, 1) both;
}

@keyframes tf-shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

.tf-loading-shimmer {
  background: linear-gradient(90deg, var(--tf-bg-raised) 25%, var(--tf-bg-hover) 50%, var(--tf-bg-raised) 75%);
  background-size: 200% 100%;
  animation: tf-shimmer 1.5s ease-in-out infinite;
  border-radius: var(--tf-radius-sm);
}
</style>
