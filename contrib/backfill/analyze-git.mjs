#!/usr/bin/env node
/**
 * Снимает реальную картину работы из git-истории всех репозиториев в каталоге.
 *
 * Нужен, чтобы засев БД опирался на то, что человек действительно делал:
 * какие проекты, в какие дни, на каких языках. Число коммитов за день — не
 * часы, но оно верно расставляет пропорции между проектами и отмечает дни,
 * когда работы не было вовсе.
 *
 *   node analyze-git.mjs [корень] [--since 2026-02-09] [--json файл]
 */

import { execFileSync } from 'node:child_process';
import { readdirSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const args = process.argv.slice(2);
const ROOT = args.find((a) => !a.startsWith('--')) ?? `${process.env.HOME}/workSpace/project`;
const SINCE = args.includes('--since') ? args[args.indexOf('--since') + 1] : '7 months ago';
const JSON_OUT = args.includes('--json') ? args[args.indexOf('--json') + 1] : null;

const git = (cwd, ...a) => {
  try {
    return execFileSync('git', ['-C', cwd, ...a], { encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 });
  } catch {
    return '';
  }
};

// Автор берётся из глобального конфига, а не задаётся руками: в репозиториях
// встречаются коммиты с разных адресов одного человека, и почта в конфиге —
// единственный ориентир, который не надо угадывать.
const selfEmail = execFileSync('git', ['config', '--global', 'user.email'], { encoding: 'utf8' }).trim();
const selfName = execFileSync('git', ['config', '--global', 'user.name'], { encoding: 'utf8' }).trim();

const LANG_BY_EXT = {
  rs: 'Rust', ts: 'TypeScript', tsx: 'TypeScript', js: 'JavaScript', mjs: 'JavaScript',
  cjs: 'JavaScript', jsx: 'JavaScript', py: 'Python', sh: 'Shell', vue: 'Vue',
  toml: 'TOML', json: 'JSON', md: 'Markdown', sql: 'SQL', css: 'CSS', scss: 'CSS',
  html: 'HTML', go: 'Go', java: 'Java', yml: 'YAML', yaml: 'YAML', ps1: 'PowerShell',
  kt: 'Kotlin', rb: 'Ruby', php: 'PHP', c: 'C', h: 'C', cpp: 'C++', cs: 'C#',
};

/**
 * Каталоги-клоны одного репозитория (preview, hotfix, worktrees) держат ту же
 * историю, и без дедупликации проект считается столько раз, сколько у него
 * копий на диске — boostix давал ровно тройной вес. Ключ — адрес origin;
 * побеждает каталог с самым коротким именем, он же обычно основной.
 */
const repos = (() => {
  const byOrigin = new Map();
  const loose = [];
  for (const d of readdirSync(ROOT, { withFileTypes: true })) {
    if (!d.isDirectory() || !existsSync(join(ROOT, d.name, '.git'))) continue;
    const origin = git(join(ROOT, d.name), 'remote', 'get-url', 'origin').trim();
    if (!origin) { loose.push(d.name); continue; }
    const prev = byOrigin.get(origin);
    if (!prev || d.name.length < prev.length) byOrigin.set(origin, d.name);
  }
  return [...byOrigin.values(), ...loose].sort();
})();

/** день → { проект → { коммиты, файлы, языки, ветки, часы } } */
const days = new Map();
const perProject = new Map();

for (const repo of repos) {
  const cwd = join(ROOT, repo);
  // %x1f/%x1e — разделители полей и записей: сообщения коммитов содержат
  // любые печатные символы, а эти два не встречаются в тексте.
  const raw = git(cwd, 'log', '--all', `--since=${SINCE}`,
    `--author=${selfEmail}`, '--no-merges',
    '--pretty=format:%x1e%H%x1f%aI%x1f%aE', '--numstat');
  if (!raw.trim()) continue;

  for (const rec of raw.split('\x1e').slice(1)) {
    const [head, ...statLines] = rec.split('\n');
    const [, iso] = head.split('\x1f');
    if (!iso) continue;
    const day = iso.slice(0, 10);
    const hour = Number(iso.slice(11, 13));

    if (!days.has(day)) days.set(day, new Map());
    const byProject = days.get(day);
    if (!byProject.has(repo)) byProject.set(repo, { commits: 0, files: 0, langs: {}, hours: [] });
    const e = byProject.get(repo);
    e.commits += 1;
    e.hours.push(hour);

    for (const line of statLines) {
      const parts = line.split('\t');
      if (parts.length !== 3) continue;
      const path = parts[2];
      if (/node_modules|\/target\/|\/dist\/|package-lock|Cargo\.lock/.test(path)) continue;
      e.files += 1;
      const lang = LANG_BY_EXT[path.split('.').pop()?.toLowerCase()];
      if (lang) e.langs[lang] = (e.langs[lang] ?? 0) + 1;
    }

    const p = perProject.get(repo) ?? { commits: 0, files: 0, days: new Set(), langs: {} };
    p.commits += 1;
    p.files += e.files;
    p.days.add(day);
    for (const [l, n] of Object.entries(e.langs)) p.langs[l] = (p.langs[l] ?? 0) + n;
    perProject.set(repo, p);
  }
}

const sortedDays = [...days.keys()].sort();
const ranking = [...perProject.entries()]
  .map(([repo, p]) => ({ repo, commits: p.commits, days: p.days.size, langs: p.langs }))
  .sort((a, b) => b.commits - a.commits);

console.log(`Автор: ${selfName} <${selfEmail}>`);
console.log(`Репозиториев просмотрено: ${repos.length}, с коммитами за период: ${perProject.size}`);
console.log(`Период: ${SINCE} → сегодня. Дней с активностью: ${sortedDays.length}`);
console.log(`Первый день: ${sortedDays[0]}, последний: ${sortedDays.at(-1)}\n`);

console.log('Где больше всего работы (по коммитам):');
for (const r of ranking.slice(0, 20)) {
  const top = Object.entries(r.langs).sort((a, b) => b[1] - a[1]).slice(0, 3).map(([l, n]) => `${l} ${n}`).join(', ');
  console.log(`  ${String(r.commits).padStart(5)} коммитов · ${String(r.days).padStart(3)} дней · ${r.repo.padEnd(24)} ${top}`);
}

const total = ranking.reduce((s, r) => s + r.commits, 0);
console.log(`\nВсего коммитов: ${total}`);

// Распределение по часам суток — понадобится, чтобы засев ложился в те же
// часы, когда человек действительно коммитил, а не в равномерное окно.
const hourHist = new Array(24).fill(0);
for (const byProject of days.values()) {
  for (const e of byProject.values()) for (const h of e.hours) hourHist[h] += 1;
}
console.log('\nЧасы активности (UTC, по коммитам):');
const maxH = Math.max(...hourHist);
hourHist.forEach((n, h) => {
  if (n === 0) return;
  console.log(`  ${String(h).padStart(2, '0')}:00 ${'█'.repeat(Math.round((n / maxH) * 40))} ${n}`);
});

if (JSON_OUT) {
  const out = {
    author: { name: selfName, email: selfEmail },
    since: SINCE,
    days: Object.fromEntries([...days].map(([d, m]) => [d, Object.fromEntries(
      [...m].map(([repo, e]) => [repo, { commits: e.commits, files: e.files, langs: e.langs, hours: e.hours }]),
    )])),
    ranking: ranking.map((r) => ({ ...r })),
    hourHist,
  };
  writeFileSync(JSON_OUT, JSON.stringify(out, null, 1));
  console.log(`\nСырьё: ${JSON_OUT}`);
}
