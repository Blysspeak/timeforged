#!/usr/bin/env node
/**
 * Заполняет БД историей работы за прошедший период.
 *
 * Зачем. Демон не работал семь месяцев, а работа шла. Git знает только моменты
 * коммитов — он не знает часов. Поэтому часы задаются владельцем (он источник
 * истины о собственном дне), а распределение по проектам, языкам и часам суток
 * берётся из реальной git-истории, снятой `analyze-git.mjs`.
 *
 * Как события превращаются во время. Демон режет поток на сессии по паузе
 * `idle_timeout` (по умолчанию 300 с) и считает длительность от первого события
 * сессии до последнего. Значит внутри рабочего блока события должны идти чаще
 * пяти минут, а перерыв между блоками — быть заведомо длиннее. Отсюда шаг
 * 2-4 минуты внутри блока и пауза 12-45 минут между блоками.
 *
 *   node seed-history.mjs --activity /tmp/git-activity.json --dry
 *   node seed-history.mjs --activity /tmp/git-activity.json --apply
 */

import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(name) ? args[args.indexOf(name) + 1] : def);
const APPLY = args.includes('--apply');
const ACTIVITY = opt('--activity', '/tmp/git-activity.json');
const DB = opt('--db', `${process.env.HOME}/.local/share/timeforged/timeforged.db`);
const HEAVY_SHARE = Number(opt('--heavy-share', '0.4'));   // доля длинных дней
const LIGHT_HOURS = [4, 5];                                 // обычный день
const HEAVY_HOURS = [10, 12];                               // длинный день
const IDLE_TIMEOUT = 300;                                   // как в конфиге демона
const SEED = Number(opt('--seed', '20260909'));

/** Детерминированный генератор: один и тот же прогон даёт одну и ту же историю. */
let state = SEED >>> 0;
const rnd = () => {
  state ^= state << 13; state >>>= 0;
  state ^= state >> 17;
  state ^= state << 5; state >>>= 0;
  return state / 0x100000000;
};
const pick = (arr) => arr[Math.floor(rnd() * arr.length)];
const between = (a, b) => a + rnd() * (b - a);

const activity = JSON.parse(readFileSync(ACTIVITY, 'utf8'));
const sql = (q) => execFileSync('sqlite3', [DB, q], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }).trim();

const userId = sql('select id from users order by rowid limit 1;');
if (!userId) throw new Error('в БД нет пользователя — запусти демон хотя бы раз');

// ── Профиль проектов ───────────────────────────────────────────────────
// Вес проекта = его доля коммитов. Для дня без коммитов берётся общий
// профиль: человек работал и в такой день, просто ничего не закоммитил.
const ranking = activity.ranking.filter((r) => r.commits > 0);
const totalCommits = ranking.reduce((s, r) => s + r.commits, 0);
const globalProfile = ranking.map((r) => ({
  repo: r.repo,
  weight: r.commits / totalCommits,
  langs: Object.entries(r.langs).sort((a, b) => b[1] - a[1]).slice(0, 6),
}));

const langsOf = (repo) => {
  const found = globalProfile.find((p) => p.repo === repo);
  return found?.langs?.length ? found.langs : [['Markdown', 1]];
};

/** Взвешенный выбор проекта. */
const chooseProject = (weights) => {
  const total = weights.reduce((s, w) => s + w.weight, 0);
  let x = rnd() * total;
  for (const w of weights) { x -= w.weight; if (x <= 0) return w.repo; }
  return weights.at(-1).repo;
};

/** Взвешенный выбор языка внутри проекта — по числу изменённых строк. */
const chooseLang = (repo) => {
  const langs = langsOf(repo);
  const total = langs.reduce((s, [, n]) => s + n, 0);
  let x = rnd() * total;
  for (const [l, n] of langs) { x -= n; if (x <= 0) return l; }
  return langs[0][0];
};

// ── Часы суток ─────────────────────────────────────────────────────────
// Гистограмма реальных коммитов задаёт, когда человек за машиной. Ровный
// случайный разброс по суткам дал бы работу в 5 утра там, где её не бывает.
const hourHist = activity.hourHist.map((n) => n + 1); // +1: ни один час не невозможен
const totalHourWeight = hourHist.reduce((s, n) => s + n, 0);
const chooseStartHour = () => {
  let x = rnd() * totalHourWeight;
  for (let h = 0; h < 24; h++) { x -= hourHist[h]; if (x <= 0) return h; }
  return 20;
};

// ── Период ─────────────────────────────────────────────────────────────
const dayKeys = Object.keys(activity.days).sort();
const firstDay = new Date(`${dayKeys[0]}T00:00:00Z`);
const today = new Date();
today.setUTCHours(0, 0, 0, 0);

const existing = new Set(
  sql(`select distinct date(timestamp) from events where user_id='${userId}';`).split('\n').filter(Boolean),
);

const rows = [];
let stats = { days: 0, heavy: 0, light: 0, seconds: 0, skipped: 0 };

for (let d = new Date(firstDay); d < today; d.setUTCDate(d.getUTCDate() + 1)) {
  const day = d.toISOString().slice(0, 10);
  // День, где события уже есть, не трогаем: реальные данные важнее засева.
  if (existing.has(day)) { stats.skipped += 1; continue; }

  const heavy = rnd() < HEAVY_SHARE;
  const targetHours = heavy ? between(HEAVY_HOURS[0], HEAVY_HOURS[1]) : between(LIGHT_HOURS[0], LIGHT_HOURS[1]);
  const targetSeconds = Math.round(targetHours * 3600);

  // Проекты дня: если в этот день были коммиты — они и задают состав.
  const commitsToday = activity.days[day];
  const dayWeights = commitsToday
    ? Object.entries(commitsToday).map(([repo, e]) => ({ repo, weight: e.commits }))
    : globalProfile;

  // Раскладываем целевое время на блоки по 40-110 минут с перерывами.
  let produced = 0;
  let cursor = new Date(`${day}T00:00:00Z`);
  cursor.setUTCHours(chooseStartHour(), Math.floor(rnd() * 60), 0, 0);

  while (produced < targetSeconds) {
    const blockSeconds = Math.min(targetSeconds - produced, Math.round(between(40 * 60, 110 * 60)));
    // Блок короче двух шагов не даст ни одной сессии — доливаем остаток к паузе.
    if (blockSeconds < 300) break;

    const project = chooseProject(dayWeights);
    const blockEnd = new Date(cursor.getTime() + blockSeconds * 1000);
    let t = new Date(cursor);
    while (t < blockEnd) {
      const lang = chooseLang(project);
      rows.push([
        userId,
        t.toISOString().replace('.000Z', 'Z'),
        'file',
        `/home/blyss/workSpace/project/${project}`,
        project,
        lang,
        'main',
        'coding',
        'ULTRA',
        '{"source":"backfill"}',
      ]);
      // Шаг заведомо меньше idle_timeout, иначе сессия распадётся.
      t = new Date(t.getTime() + Math.round(between(120, 240)) * 1000);
    }
    produced += blockSeconds;
    // Перерыв длиннее idle_timeout — так блоки станут отдельными сессиями.
    cursor = new Date(blockEnd.getTime() + Math.round(between(IDLE_TIMEOUT * 2.4, 45 * 60)) * 1000);
  }

  stats.days += 1;
  stats[heavy ? 'heavy' : 'light'] += 1;
  stats.seconds += produced;
}

const esc = (s) => String(s).replace(/'/g, "''");
console.log(`Пользователь: ${userId}`);
console.log(`Период: ${dayKeys[0]} → ${today.toISOString().slice(0, 10)}`);
console.log(`Дней засеяно: ${stats.days} (длинных ${stats.heavy}, обычных ${stats.light}), пропущено с реальными данными: ${stats.skipped}`);
console.log(`Событий: ${rows.length}`);
console.log(`Часов всего: ${(stats.seconds / 3600).toFixed(1)}, в среднем за день: ${(stats.seconds / 3600 / Math.max(stats.days, 1)).toFixed(1)}`);

if (!APPLY) {
  console.log('\n--dry: ничего не записано. Пример событий:');
  for (const r of rows.slice(0, 3)) console.log('  ', r[1], r[4], r[5]);
  process.exit(0);
}

// Пишем пачками: одна транзакция на 5000 строк — иначе командная строка
// sqlite3 упирается в предел длины аргумента.
const CHUNK = 5000;
for (let i = 0; i < rows.length; i += CHUNK) {
  const values = rows.slice(i, i + CHUNK)
    .map((r) => `('${esc(r[0])}','${esc(r[1])}','${esc(r[2])}','${esc(r[3])}','${esc(r[4])}','${esc(r[5])}','${esc(r[6])}','${esc(r[7])}','${esc(r[8])}','${esc(r[9])}')`)
    .join(',');
  execFileSync('sqlite3', [DB], {
    input: `BEGIN;INSERT INTO events (user_id,timestamp,event_type,entity,project,language,branch,activity,machine,metadata) VALUES ${values};COMMIT;`,
    encoding: 'utf8',
  });
  process.stderr.write(`\rзаписано ${Math.min(i + CHUNK, rows.length)}/${rows.length}`);
}
process.stderr.write('\n');
console.log('готово');
