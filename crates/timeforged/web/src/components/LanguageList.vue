<script setup lang="ts">
import type { CategorySummary } from '../api'

const props = defineProps<{ languages: CategorySummary[] }>()

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

interface LangMeta {
  icon: string
  color: string
}

const langMap: Record<string, LangMeta> = {
  'rust':       { icon: 'devicon-rust-original', color: '#dea584' },
  'typescript': { icon: 'devicon-typescript-plain', color: '#3178c6' },
  'javascript': { icon: 'devicon-javascript-plain', color: '#f7df1e' },
  'python':     { icon: 'devicon-python-plain', color: '#3776ab' },
  'go':         { icon: 'devicon-go-plain', color: '#00add8' },
  'java':       { icon: 'devicon-java-plain', color: '#ed8b00' },
  'c':          { icon: 'devicon-c-plain', color: '#a8b9cc' },
  'c++':        { icon: 'devicon-cplusplus-plain', color: '#00599c' },
  'c#':         { icon: 'devicon-csharp-plain', color: '#68217a' },
  'php':        { icon: 'devicon-php-plain', color: '#777bb4' },
  'ruby':       { icon: 'devicon-ruby-plain', color: '#cc342d' },
  'swift':      { icon: 'devicon-swift-plain', color: '#fa7343' },
  'kotlin':     { icon: 'devicon-kotlin-plain', color: '#7f52ff' },
  'dart':       { icon: 'devicon-dart-plain', color: '#0175c2' },
  'lua':        { icon: 'devicon-lua-plain', color: '#2c2d72' },
  'html':       { icon: 'devicon-html5-plain', color: '#e34f26' },
  'css':        { icon: 'devicon-css3-plain', color: '#1572b6' },
  'sass':       { icon: 'devicon-sass-original', color: '#cc6699' },
  'vue':        { icon: 'devicon-vuejs-plain', color: '#4fc08d' },
  'react':      { icon: 'devicon-react-original', color: '#61dafb' },
  'svelte':     { icon: 'devicon-svelte-plain', color: '#ff3e00' },
  'docker':     { icon: 'devicon-docker-plain', color: '#2496ed' },
  'bash':       { icon: 'devicon-bash-plain', color: '#4eaa25' },
  'shell':      { icon: 'devicon-bash-plain', color: '#4eaa25' },
  'sql':        { icon: 'devicon-azuresqldatabase-plain', color: '#e38c00' },
  'markdown':   { icon: 'devicon-markdown-original', color: '#83888d' },
  'json':       { icon: 'devicon-json-plain', color: '#5b5b5b' },
  'yaml':       { icon: 'devicon-yaml-plain', color: '#cb171e' },
  'toml':       { icon: 'devicon-tomcat-line-wordmark', color: '#9c4121' },
  'xml':        { icon: 'devicon-xml-plain', color: '#005fad' },
  'graphql':    { icon: 'devicon-graphql-plain', color: '#e10098' },
  'r':          { icon: 'devicon-r-original', color: '#276dc3' },
  'scala':      { icon: 'devicon-scala-plain', color: '#dc322f' },
  'elixir':     { icon: 'devicon-elixir-plain', color: '#6e4a7e' },
  'haskell':    { icon: 'devicon-haskell-plain', color: '#5d4f85' },
  'zig':        { icon: 'devicon-zig-original', color: '#f7a41d' },
  'nix':        { icon: 'devicon-nixos-plain', color: '#5277c3' },
}

const defaultColors = [
  '#8b5cf6', '#2dd4bf', '#fbbf24', '#fb7185', '#60a5fa',
  '#a78bfa', '#34d399', '#f59e0b', '#f472b6', '#38bdf8',
]

function getLang(name: string): LangMeta {
  const key = name.toLowerCase()
  if (langMap[key]) return langMap[key]
  return { icon: '', color: '' }
}

function getFallbackColor(index: number): string {
  return defaultColors[index % defaultColors.length]
}
</script>

<template>
  <div v-if="languages.length === 0" class="tf-empty">No languages tracked yet</div>
  <div v-else class="tf-lang-list">
    <div
      v-for="(l, i) in languages"
      :key="l.name"
      class="tf-lang-item"
      :style="{ animationDelay: `${i * 40}ms` }"
    >
      <div class="tf-lang-row">
        <div class="tf-lang-left">
          <span
            v-if="getLang(l.name).icon"
            class="tf-lang-icon"
            :style="{ color: getLang(l.name).color }"
          >
            <i :class="getLang(l.name).icon"></i>
          </span>
          <span v-else class="tf-lang-dot" :style="{ background: getFallbackColor(i) }"></span>
          <span class="tf-lang-name">{{ l.name }}</span>
        </div>
        <div class="tf-lang-meta">
          <span class="tf-lang-time">{{ formatDuration(l.total_seconds) }}</span>
          <span class="tf-lang-pct">{{ l.percent.toFixed(0) }}%</span>
        </div>
      </div>
      <div class="tf-lang-bar-track">
        <div
          class="tf-lang-bar-fill"
          :style="{
            width: Math.max(l.percent, 1) + '%',
            background: getLang(l.name).color || getFallbackColor(i)
          }"
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

.tf-lang-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.tf-lang-item {
  animation: tf-fade-up 0.35s cubic-bezier(0.16, 1, 0.3, 1) both;
}

.tf-lang-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.tf-lang-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.tf-lang-icon {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 16px;
}

.tf-lang-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.tf-lang-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--tf-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tf-lang-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.tf-lang-time {
  font-family: var(--tf-font-mono);
  font-size: 12px;
  color: var(--tf-text-secondary);
}

.tf-lang-pct {
  font-family: var(--tf-font-mono);
  font-size: 11px;
  color: var(--tf-text-tertiary);
  width: 32px;
  text-align: right;
}

.tf-lang-bar-track {
  height: 4px;
  background: rgba(255, 255, 255, 0.04);
  border-radius: 4px;
  overflow: hidden;
}

.tf-lang-bar-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.6s cubic-bezier(0.16, 1, 0.3, 1);
  opacity: 0.75;
}

@keyframes tf-fade-up {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
