const BASE = ''

function getApiKey(): string | null {
  return localStorage.getItem('tf_api_key')
}

export function setApiKey(key: string) {
  localStorage.setItem('tf_api_key', key)
}

export function clearApiKey() {
  localStorage.removeItem('tf_api_key')
}

export function hasApiKey(): boolean {
  return !!getApiKey()
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const key = getApiKey()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(key ? { 'X-Api-Key': key } : {}),
  }

  const res = await fetch(`${BASE}${path}`, { ...options, headers })

  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(body.error || `HTTP ${res.status}`)
  }

  return res.json()
}

export interface StatusResponse {
  status: string
  version: string
  user_count: number
  event_count: number
}

export interface Summary {
  total_seconds: number
  from: string
  to: string
  projects: CategorySummary[]
  languages: CategorySummary[]
  days: DaySummary[]
}

export interface CategorySummary {
  name: string
  total_seconds: number
  percent: number
}

export interface DaySummary {
  date: string
  total_seconds: number
}

export interface Session {
  start: string
  end: string
  duration_seconds: number
  project: string | null
  event_count: number
}

export interface HourlyActivity {
  hour: number
  total_seconds: number
  event_count: number
}

export const api = {
  status: () => request<StatusResponse>('/api/v1/status'),
  summary: (from?: string, to?: string) => {
    const params = new URLSearchParams()
    if (from) params.set('from', from)
    if (to) params.set('to', to)
    const qs = params.toString()
    return request<Summary>(`/api/v1/reports/summary${qs ? '?' + qs : ''}`)
  },
  sessions: (from?: string, to?: string) => {
    const params = new URLSearchParams()
    if (from) params.set('from', from)
    if (to) params.set('to', to)
    const qs = params.toString()
    return request<Session[]>(`/api/v1/reports/sessions${qs ? '?' + qs : ''}`)
  },
  activity: (from?: string, to?: string) => {
    const params = new URLSearchParams()
    if (from) params.set('from', from)
    if (to) params.set('to', to)
    const qs = params.toString()
    return request<HourlyActivity[]>(`/api/v1/reports/activity${qs ? '?' + qs : ''}`)
  },
  me: () => request<{ id: string; username: string; display_name: string | null }>('/api/v1/me'),
}
