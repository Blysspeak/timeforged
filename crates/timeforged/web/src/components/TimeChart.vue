<script setup lang="ts">
import { computed } from 'vue'
import { Bar } from 'vue-chartjs'
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  BarElement,
  Tooltip,
} from 'chart.js'
import type { DaySummary } from '../api'

ChartJS.register(CategoryScale, LinearScale, BarElement, Tooltip)

const props = defineProps<{ days: DaySummary[] }>()

const chartData = computed(() => {
  const gradient = (ctx: any) => {
    const chart = ctx.chart
    const { ctx: c, chartArea } = chart
    if (!chartArea) return '#e8933a'
    const g = c.createLinearGradient(0, chartArea.bottom, 0, chartArea.top)
    g.addColorStop(0, 'rgba(232, 147, 58, 0.4)')
    g.addColorStop(1, 'rgba(232, 147, 58, 0.9)')
    return g
  }

  return {
    labels: props.days.map((d) => {
      const date = new Date(d.date + 'T00:00:00')
      return date.toLocaleDateString('en', { weekday: 'short', day: 'numeric' })
    }),
    datasets: [
      {
        data: props.days.map((d) => +(d.total_seconds / 3600).toFixed(2)),
        backgroundColor: gradient,
        hoverBackgroundColor: '#f0a54e',
        borderRadius: { topLeft: 6, topRight: 6 },
        maxBarThickness: 56,
        borderSkipped: false,
      },
    ],
  }
})

const chartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  animation: {
    duration: 600,
    easing: 'easeOutQuart' as const,
  },
  plugins: {
    tooltip: {
      backgroundColor: '#1c1c22',
      borderColor: '#2a2a35',
      borderWidth: 1,
      titleColor: '#f0f0f3',
      bodyColor: '#8b8b9e',
      titleFont: { family: "'JetBrains Mono'", size: 12, weight: 600 },
      bodyFont: { family: "'DM Sans'", size: 12 },
      padding: { x: 12, y: 8 },
      cornerRadius: 8,
      displayColors: false,
      callbacks: {
        title: (items: any[]) => items[0]?.label || '',
        label: (ctx: any) => {
          const h = Math.floor(ctx.raw)
          const m = Math.round((ctx.raw - h) * 60)
          return h > 0 ? `${h}h ${m}m` : `${m}m`
        },
      },
    },
  },
  scales: {
    x: {
      ticks: {
        color: '#55556a',
        font: { family: "'DM Sans'", size: 11 },
        padding: 4,
      },
      grid: { display: false },
      border: { display: false },
    },
    y: {
      ticks: {
        color: '#55556a',
        font: { family: "'JetBrains Mono'", size: 10 },
        callback: (v: any) => `${v}h`,
        padding: 8,
      },
      grid: {
        color: 'rgba(255, 255, 255, 0.03)',
        lineWidth: 1,
      },
      border: { display: false, dash: [4, 4] },
    },
  },
}
</script>

<template>
  <div class="tf-chart-wrap">
    <Bar :data="chartData" :options="chartOptions" />
  </div>
</template>

<style scoped>
.tf-chart-wrap {
  height: 240px;
  position: relative;
}
</style>
