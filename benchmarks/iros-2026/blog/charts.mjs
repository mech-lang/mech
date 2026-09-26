/** Publication charts computed from the retained ten-sample native campaigns. */
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const COLORS = {
  Mech: ['#936a0e', '#f4c653'],
  Rust: ['#925c44', '#dfad93'],
  Mojo: ['#bc4b08', '#ff8734'],
  Julia: ['#75518e', '#bc87d3'],
  Futhark: ['#a2476c', '#e480aa'],
  Numba: ['#395ea9', '#789ee6'],
  Taichi: ['#2e8270', '#63c5aa'],
  Halide: ['#327f99', '#78c7e1'],
};
const WIDTH = 900;
const LEFT = 205;
const RIGHT = 700;
const MEDIAN_X = 791;
const PLUS_X = 812;
const MAD_X = 875;
const ROW_HEIGHT = 62;
const BAR_HEIGHT = 18;
const BAR_GAP = 4;
const BACKGROUND = '#101719';
const FOREGROUND = '#e9eeef';
const MUTED = '#b0c2c9';
const SOURCE_ROOT = 'https://github.com/mech-lang/mech/blob/iros-workshop-benchmarks/benchmarks/iros-2026/results/';

function esc(value) {
  return String(value).replace(/[&<>"']/g, c => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;',
  }[c]));
}

function load(filename) {
  const content = readFileSync(new URL(`../results/${filename}`, import.meta.url));
  return {
    filename,
    sha256: createHash('sha256').update(content).digest('hex'),
    data: JSON.parse(content),
  };
}

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const middle = sorted.length / 2;
  return (sorted[Math.floor(middle)] + sorted[Math.ceil(middle) - 1]) / 2;
}

function statistics(samples, expectedMedian, expectedMad, identity) {
  if (!Array.isArray(samples) || samples.length !== 10 ||
      samples.some(value => !Number.isFinite(value) || value <= 0)) {
    throw new Error(`${identity}: expected ten finite, positive samples`);
  }
  const center = median(samples);
  const mad = median(samples.map(value => Math.abs(value - center)));
  for (const [actual, expected, name] of [
    [center, expectedMedian, 'median'], [mad, expectedMad, 'MAD'],
  ]) {
    if (!Number.isFinite(expected) || Math.abs(actual - expected) > 1e-9 * Math.max(1, actual)) {
      throw new Error(`${identity}: recomputed ${name} disagrees with the archive`);
    }
  }
  return { median: center, mad, samples: [...samples] };
}

function row(archive, group, key, label, color, detail = '', gpu = false) {
  const entry = group ? archive.data.campaigns[group].rows[key] : archive.data.summary[key];
  if (!entry) throw new Error(`Missing archived row: ${archive.filename}: ${key}`);
  const values = {};
  for (const mode of ['unchecked', 'checked']) {
    const value = entry[mode];
    values[mode] = statistics(
      value?.[group ? 'samples_million_ekf_turns_per_second' : 'samples_million_filter_turns_per_second'],
      value?.[group ? 'median_million_ekf_turns_per_second' : 'median_million_filter_turns_per_second'],
      value?.[group ? 'median_absolute_deviation_million_ekf_turns_per_second' : 'mad_million_filter_turns_per_second'],
      `${archive.filename}: ${key}: ${mode}`,
    );
  }
  return { key, label, color, detail, gpu, values, source: archive.filename };
}

function number(value) {
  return value.toFixed(value < 1 ? 2 : 1);
}

function variation(value) {
  return value < 0.1 ? '<0.1' : value.toFixed(1);
}

function text(x, y, content, options = '') {
  return `<text x="${x}" y="${y}" ${options}>${esc(content)}</text>`;
}

function legend(y, hatchId) {
  return [
    `<g class="legend" aria-label="Legend: dark upper bars are unchecked; light lower bars are checked; diagonal hatching indicates GPU execution.">`,
    `<rect x="205" y="${y - 12}" width="19" height="12" fill="#69767b"/>`,
    text(232, y - 1, 'unchecked (upper)', 'font-size="14"'),
    `<rect x="405" y="${y - 12}" width="19" height="12" fill="#dce5e8"/>`,
    text(432, y - 1, 'checked (lower)', 'font-size="14"'),
    `<rect x="608" y="${y - 12}" width="19" height="12" fill="#dce5e8"/>`,
    `<rect x="608" y="${y - 12}" width="19" height="12" fill="url(#${hatchId})"/>`,
    text(635, y - 1, 'GPU', 'font-size="14"'),
    '</g>',
  ].join('\n');
}

function panel(rows, top, { title = '', log = false, maximum = 200 } = {}, hatchId) {
  const minimum = log ? 0.1 : 0;
  const plotTop = top + (title ? 36 : 0);
  const bottom = plotTop + rows.length * ROW_HEIGHT - 12;
  const ticks = log ? [0.1, 1, 10, 100, 1000] :
    maximum === 500 ? [0, 100, 200, 300, 400, 500] : [0, 50, 100, 150, 200];
  const x = value => LEFT + (RIGHT - LEFT) * (log ?
    Math.log10(value / minimum) / Math.log10(maximum / minimum) : value / maximum);
  const elements = [];
  if (title) elements.push(text(25, top + 16, title, 'class="panel-title"'));
  for (const tick of ticks) {
    const position = x(tick);
    elements.push(`<line x1="${position}" y1="${plotTop - 4}" x2="${position}" y2="${bottom}" stroke="#304047" stroke-width="1"/>`);
    elements.push(text(position, bottom + 23, tick, 'class="tick" text-anchor="middle"'));
  }
  rows.forEach((entry, index) => {
    const topY = plotTop + index * ROW_HEIGHT;
    const midY = topY + BAR_HEIGHT + BAR_GAP / 2;
    elements.push(text(LEFT - 15, midY + (entry.detail ? -3 : 5), entry.label, 'class="row-label" text-anchor="end"'));
    if (entry.detail) {
      elements.push(text(LEFT - 15, midY + 16, entry.detail, 'class="row-detail" text-anchor="end"'));
    }
    ['unchecked', 'checked'].forEach((mode, modeIndex) => {
      const value = entry.values[mode];
      const barY = topY + modeIndex * (BAR_HEIGHT + BAR_GAP);
      const centerY = barY + BAR_HEIGHT / 2;
      const barRight = x(value.median);
      if (barRight > RIGHT || value.median - value.mad <= minimum) {
        throw new Error(`Plot bounds exclude ${entry.label} ${mode}`);
      }
      const description = `${entry.label}${entry.detail ? `, ${entry.detail}` : ''}, ${mode}: ` +
        `${number(value.median)} million filter-turns per second; MAD ${variation(value.mad)}; ten samples.`;
      const attrs = `data-language="${esc(entry.label)}" data-mode="${mode}" data-source="${entry.source}" data-median="${value.median}" data-mad="${value.mad}" data-n="10"`;
      elements.push(`<g ${attrs} role="group" aria-label="${esc(description)}"><title>${esc(description)}</title>`);
      elements.push(`<rect x="${LEFT}" y="${barY}" width="${barRight - LEFT}" height="${BAR_HEIGHT}" fill="${COLORS[entry.color][modeIndex]}"/>`);
      if (entry.gpu) elements.push(`<rect x="${LEFT}" y="${barY}" width="${barRight - LEFT}" height="${BAR_HEIGHT}" fill="url(#${hatchId})"/>`);
      const lo = x(value.median - value.mad);
      const hi = x(value.median + value.mad);
      elements.push(`<path d="M${lo},${centerY} H${hi} M${lo},${centerY - 4} V${centerY + 4} M${hi},${centerY - 4} V${centerY + 4}" fill="none" stroke="#f5f7f8" stroke-width="1.15"/>`);
      elements.push(text(MEDIAN_X, centerY + 5, number(value.median), 'class="measurement" text-anchor="end"'));
      elements.push(text(PLUS_X, centerY + 5, '±', 'class="measurement" text-anchor="middle"'));
      elements.push(text(MAD_X, centerY + 5, variation(value.mad), 'class="measurement" text-anchor="end"'));
      elements.push('</g>');
    });
  });
  elements.push(`<line x1="${LEFT}" y1="${bottom}" x2="${RIGHT}" y2="${bottom}" stroke="#64767e"/>`);
  elements.push(text((LEFT + RIGHT) / 2, bottom + 49,
    `Throughput (million filter-turns/s, ${log ? 'logarithmic' : 'linear'} scale)`,
    'class="axis-label" text-anchor="middle"'));
  return { svg: elements.join('\n'), end: bottom + 69 };
}

function chart({ title, subtitle, panels, archives, notes }) {
  const id = `iros-chart-${createHash('sha256').update(title).digest('hex').slice(0, 10)}`;
  const hatchId = `${id}-gpu-hatch`;
  const content = [text(25, 35, title, 'class="title"'), text(25, 63, subtitle, 'class="subtitle"'), legend(96, hatchId)];
  let next = 124;
  for (const configuration of panels) {
    const result = panel(configuration.rows, next, configuration, hatchId);
    content.push(result.svg);
    next = result.end + 12;
  }
  notes.forEach((note, index) => content.push(text(25, next + index * 21, note, 'class="note"')));
  next += notes.length * 21 + 8;
  const label = archives.length > 1 ? 'Raw CPU and Metal records' : 'Raw samples and provenance';
  archives.forEach((archive, index) => {
    content.push(`<a href="${SOURCE_ROOT}${archive.filename}" target="_blank">` +
      text(25, next + index * 21, archives.length > 1 ? `${label}: ${index === 0 ? 'CPU' : 'Metal'}` : label, 'class="source-link"') + '</a>');
  });
  const height = next + archives.length * 21 + 13;
  const rows = panels.flatMap(value => value.rows);
  const description = `${subtitle} Unchecked bars are above checked bars. Diagonal hatching indicates GPU execution. ` +
    'Values and whiskers show the median and unscaled median absolute deviation from ten retained process trials per mode. No samples were removed. ' +
    panels.map(value => `${value.title || 'Chart'} uses a ${value.log ? 'logarithmic' : 'linear'} throughput axis.`).join(' ') + ' ' + notes.join(' ');
  const metadata = { units: 'million filter-turns/s', summary: 'median and unscaled MAD',
    archives: archives.map(({ filename, sha256 }) => ({ filename, sha256 })),
    rows: rows.map(({ key, label, source, values }) => ({ key, label, source, values })),
  };
  return `<svg xmlns="http://www.w3.org/2000/svg" id="${id}" width="${WIDTH}" height="${height}" viewBox="0 0 ${WIDTH} ${height}" role="img" aria-labelledby="${id}-title ${id}-description">
<title id="${id}-title">${esc(title)}</title>
<desc id="${id}-description">${esc(description)}</desc>
<metadata>${esc(JSON.stringify(metadata))}</metadata>
<defs><pattern id="${hatchId}" width="8" height="8" patternUnits="userSpaceOnUse"><path d="M-2 2 L2 -2 M0 8 L8 0 M6 10 L10 6" stroke="#101719" stroke-opacity="0.33" stroke-width="1.4"/></pattern></defs>
<style>
#${id} text { font-family: Inter, Arial, sans-serif; fill: ${FOREGROUND}; }
#${id} .title { font-family: 'Fira Code', monospace; font-size: 20px; font-weight: 600; fill: #f4c653; }
#${id} .subtitle, #${id} .note, #${id} .tick, #${id} .row-detail, #${id} .axis-label { fill: ${MUTED}; }
#${id} .subtitle { font-size: 15px; }
#${id} .row-label { font-size: 17px; }
#${id} .row-detail { font-size: 12px; }
#${id} .measurement { font-size: 15px; font-variant-numeric: tabular-nums; }
#${id} .panel-title { font-size: 18px; font-weight: 600; }
#${id} .tick, #${id} .axis-label, #${id} .note { font-size: 13px; }
#${id} .source-link { font-size: 13px; fill: #8ccde7; text-decoration: underline; }
</style>
<rect width="${WIDTH}" height="${height}" rx="8" fill="${BACKGROUND}"/>
${content.join('\n')}
</svg>\n`;
}

// Mobile figures use their own geometry rather than shrinking the desktop's
// labels and values. Data, scale limits, bar order and summaries are identical.
function wrapWords(value, maximum) {
  const lines = [];
  let line = '';
  for (const word of value.split(/\s+/)) {
    if (line && `${line} ${word}`.length > maximum) {
      lines.push(line);
      line = word;
    } else {
      line = line ? `${line} ${word}` : word;
    }
  }
  if (line) lines.push(line);
  return lines;
}

function mobileChart({ title, subtitle, panels, archives, notes }) {
  const width = 560;
  const margin = 20;
  const left = margin;
  const right = 352;
  const medianX = 438;
  const plusX = 464;
  const madX = width - margin;
  const rowHeight = 118;
  const barHeight = 24;
  const barGap = 6;
  const id = `iros-chart-${createHash('sha256').update(title).digest('hex').slice(0, 10)}-mobile`;
  const hatchId = `${id}-gpu-hatch`;
  const content = [];
  let next = 34;
  function lines(value, maximum, lineHeight, className) {
    for (const line of wrapWords(value, maximum)) {
      content.push(text(margin, next, line, `class="${className}"`));
      next += lineHeight;
    }
  }
  lines(title, 35, 30, 'title');
  next += 4;
  lines(subtitle, 49, 25, 'subtitle');
  next += 15;
  content.push('<g class="legend" aria-label="Dark upper bars are unchecked; light lower bars are checked; diagonal hatching indicates GPU execution.">');
  for (const [x, fill, label] of [[20, '#69767b', 'unchecked (upper)'], [290, '#dce5e8', 'checked (lower)']]) {
    content.push(`<rect x="${x}" y="${next - 16}" width="22" height="16" fill="${fill}"/>`);
    content.push(text(x + 31, next - 1, label, 'class="legend-label"'));
  }
  next += 29;
  content.push(`<rect x="20" y="${next - 16}" width="22" height="16" fill="#dce5e8"/>`);
  content.push(`<rect x="20" y="${next - 16}" width="22" height="16" fill="url(#${hatchId})"/>`);
  content.push(text(51, next - 1, 'GPU', 'class="legend-label"'), '</g>');
  next += 28;

  for (const configuration of panels) {
    const { rows, title: panelTitle = '', log = false, maximum = 200 } = configuration;
    const minimum = log ? 0.1 : 0;
    const x = value => left + (right - left) * (log ?
      Math.log10(value / minimum) / Math.log10(maximum / minimum) : value / maximum);
    if (panelTitle) {
      content.push(text(margin, next + 20, panelTitle, 'class="panel-title"'));
      next += 45;
    }
    const ticks = log ? [0.1, 1, 10, 100, 1000] :
      maximum === 500 ? [0, 100, 200, 300, 400, 500] : [0, 50, 100, 150, 200];
    const plotTop = next;
    const bottom = plotTop + rows.length * rowHeight - 11;
    rows.forEach((entry, index) => {
      const top = plotTop + index * rowHeight;
      const label = `${entry.label}${entry.detail ? ` · ${entry.detail}` : ''}`;
      content.push(text(margin, top + 20, label, 'class="row-label"'));
      for (const tick of ticks) {
        content.push(`<line x1="${x(tick)}" y1="${top + 33}" x2="${x(tick)}" y2="${top + 91}" stroke="#304047" stroke-width="1"/>`);
      }
      ['unchecked', 'checked'].forEach((mode, modeIndex) => {
        const value = entry.values[mode];
        const barY = top + 35 + modeIndex * (barHeight + barGap);
        const centerY = barY + barHeight / 2;
        const barRight = x(value.median);
        if (barRight > right || value.median - value.mad <= minimum) {
          throw new Error(`Mobile plot bounds exclude ${entry.label} ${mode}`);
        }
        const description = `${label}, ${mode}: ${number(value.median)} million filter-turns per second; ` +
          `MAD ${variation(value.mad)}; ten samples.`;
        const attrs = `data-language="${esc(entry.label)}" data-mode="${mode}" data-source="${entry.source}" data-median="${value.median}" data-mad="${value.mad}" data-n="10"`;
        content.push(`<g ${attrs} role="group" aria-label="${esc(description)}"><title>${esc(description)}</title>`);
        content.push(`<rect x="${left}" y="${barY}" width="${barRight - left}" height="${barHeight}" fill="${COLORS[entry.color][modeIndex]}"/>`);
        if (entry.gpu) content.push(`<rect x="${left}" y="${barY}" width="${barRight - left}" height="${barHeight}" fill="url(#${hatchId})"/>`);
        const lo = x(value.median - value.mad), hi = x(value.median + value.mad);
        content.push(`<path d="M${lo},${centerY} H${hi} M${lo},${centerY - 5} V${centerY + 5} M${hi},${centerY - 5} V${centerY + 5}" fill="none" stroke="#f5f7f8" stroke-width="1.3"/>`);
        content.push(text(medianX, centerY + 7, number(value.median), 'class="measurement" text-anchor="end"'));
        content.push(text(plusX, centerY + 7, '±', 'class="measurement" text-anchor="middle"'));
        content.push(text(madX, centerY + 7, variation(value.mad), 'class="measurement" text-anchor="end"'));
        content.push('</g>');
      });
    });
    content.push(`<line x1="${left}" y1="${bottom}" x2="${right}" y2="${bottom}" stroke="#64767e"/>`);
    for (const tick of ticks) content.push(text(x(tick), bottom + 26, tick, 'class="tick" text-anchor="middle"'));
    content.push(text(width / 2, bottom + 58, 'Throughput (million filter-turns/s)', 'class="axis-label" text-anchor="middle"'));
    content.push(text(width / 2, bottom + 83, log ? 'Logarithmic scale' : 'Linear scale', 'class="axis-label" text-anchor="middle"'));
    next = bottom + 124;
  }
  for (const note of notes) {
    lines(note, 55, 24, 'note');
    next += 9;
  }
  const label = archives.length > 1 ? 'Raw CPU and Metal records' : 'Raw samples and provenance';
  archives.forEach((archive, index) => {
    content.push(`<a href="${SOURCE_ROOT}${archive.filename}" target="_blank">` +
      text(margin, next, archives.length > 1 ? `${label}: ${index === 0 ? 'CPU' : 'Metal'}` : label, 'class="source-link"') + '</a>');
    next += 27;
  });
  const height = next + 10;
  const description = `${subtitle} Unchecked bars are above checked bars. Diagonal hatching indicates GPU execution. ` +
    'Values and whiskers show the median and unscaled median absolute deviation from ten retained process trials per mode. No samples were removed. ' +
    panels.map(value => `${value.title || 'Chart'} uses a ${value.log ? 'logarithmic' : 'linear'} throughput axis.`).join(' ') + ' ' + notes.join(' ');
  const metadata = { units: 'million filter-turns/s', summary: 'median and unscaled MAD',
    archives: archives.map(({ filename, sha256 }) => ({ filename, sha256 })),
    rows: panels.flatMap(value => value.rows).map(({ key, label, source, values }) => ({ key, label, source, values })),
  };
  return `<svg xmlns="http://www.w3.org/2000/svg" id="${id}" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="${id}-title ${id}-description">
<title id="${id}-title">${esc(title)}</title>
<desc id="${id}-description">${esc(description)}</desc>
<metadata>${esc(JSON.stringify(metadata))}</metadata>
<defs><pattern id="${hatchId}" width="8" height="8" patternUnits="userSpaceOnUse"><path d="M-2 2 L2 -2 M0 8 L8 0 M6 10 L10 6" stroke="#101719" stroke-opacity="0.33" stroke-width="1.4"/></pattern></defs>
<style>
#${id} text { font-family: Inter, Arial, sans-serif; fill: ${FOREGROUND}; }
#${id} .title { font-family: 'Fira Code', monospace; font-size: 24px; font-weight: 600; fill: #f4c653; }
#${id} .subtitle, #${id} .note, #${id} .tick, #${id} .axis-label { fill: ${MUTED}; }
#${id} .subtitle, #${id} .legend-label, #${id} .axis-label { font-size: 20px; }
#${id} .row-label { font-size: 22px; font-weight: 600; }
#${id} .measurement { font-size: 23px; font-variant-numeric: tabular-nums; }
#${id} .panel-title { font-size: 24px; font-weight: 600; }
#${id} .tick { font-size: 19px; }
#${id} .note { font-size: 18px; }
#${id} .source-link { font-size: 18px; fill: #8ccde7; text-decoration: underline; }
</style>
<rect width="${width}" height="${height}" rx="8" fill="${BACKGROUND}"/>
${content.join('\n')}
</svg>\n`;
}

/** Write desktop and mobile SVGs for four charts; return all eight paths. */
export function buildCharts(outputDir) {
  const destination = outputDir instanceof URL ? fileURLToPath(outputDir) : resolve(outputDir);
  const cpu = load('apple-m1-cpu-equal-n10-2026-09-24.json');
  const metal = load('apple-m1-metal-equal-n10-2026-09-24.json');
  const backends = load('apple-m1-mech-backend-pairs-n10-2026-09-25.json');
  const cpuRow = (key, label, color, detail) => row(cpu, 'cpu', key, label, color, detail);
  const gpuRow = (key, label, color, detail = '') => row(metal, 'metal', key, label, color, detail, true);
  const backendRow = (key, label, detail, gpu = false) => row(backends, null, key, label, 'Mech', detail, gpu);
  const charts = {
    'cpu.svg': {
      title: 'Mech and Rust reach similar checked CPU throughput',
      subtitle: 'Apple M1 · 500,000 f32 filters × 40 turns · eight workers · median ± MAD',
      panels: [{ rows: [
        cpuRow('Mech fused SIMD/JIT', 'Mech', 'Mech', 'SIMD/JIT'),
        cpuRow('Rust packed SIMD', 'Rust', 'Rust', 'packed SIMD'),
        cpuRow('Mojo SIMD-4', 'Mojo', 'Mojo', 'SIMD-4'),
        cpuRow('Julia SIMD.jl', 'Julia', 'Julia', 'SIMD.jl'),
        cpuRow('Futhark ISPC AOT', 'Futhark', 'Futhark', 'AOT'),
        cpuRow('NumPy/Numba', 'Numba', 'Numba', 'compiled Python kernel'),
      ] }],
      archives: [cpu],
      notes: ['2026-09-24 CPU campaign. Fused 40-turn execution; failure interfaces differ.',
        'Ten process trials per mode; Mech modes share a process. MAD is descriptive spread.'],
    },
    'backends.svg': {
      title: 'Backend choice changes EKF throughput',
      subtitle: 'Apple M1 · same f32 source · 500,000 filters × 40 turns · median ± MAD',
      panels: [{ log: true, maximum: 1000, rows: [
        backendRow('metal', 'Metal GPU', '64-thread groups', true),
        backendRow('simd-jit-8w', 'SIMD JIT', '8 CPU workers'),
        backendRow('simd-aot', 'SIMD AOT', '1 CPU worker'),
        backendRow('scalar-jit', 'Scalar JIT', '1 CPU worker'),
        backendRow('evaluator', 'Interpreter', 'numeric instructions'),
      ] }],
      archives: [backends],
      notes: ['2026-09-25 matched campaign. Both modes publish state after every turn.',
        'Ten fresh processes per mode. JIT and AOT share the numerical lowering pipeline.'],
    },
    'metal.svg': {
      title: 'Metal implementations use a common turn boundary',
      subtitle: 'Apple M1 GPU · 500,000 f32 filters × 40 turns · median ± MAD',
      panels: [{ maximum: 500, rows: [
        gpuRow('Mech generated MSL', 'Mech', 'Mech', 'generated Metal'),
        gpuRow('Rust + hand-written MSL', 'Rust + MSL', 'Rust', 'handwritten Metal'),
        gpuRow('Mojo native Metal', 'Mojo', 'Mojo', 'native Metal'),
        gpuRow('Julia Metal.jl', 'Julia', 'Julia', 'Metal.jl'),
        gpuRow('Taichi native Metal', 'Taichi', 'Taichi', 'native Metal'),
        gpuRow('Halide Metal schedule', 'Halide', 'Halide', 'Metal schedule'),
      ] }],
      archives: [metal],
      notes: ['2026-09-24 Metal campaign. Resident state; completed publication after every turn.',
        'Ten fresh processes per mode. Schedules and fault-status observation differ.'],
    },
    'portable.svg': {
      title: 'One application source targets CPU and Metal',
      subtitle: 'Apple M1 · 500,000 f32 filters × 40 turns · per-turn publication · median ± MAD',
      panels: [
        { title: 'CPU · eight workers', rows: [
          cpuRow('Mech per-turn SIMD/JIT', 'Mech', 'Mech', 'SIMD/JIT'),
          cpuRow('Taichi LLVM CPU', 'Taichi', 'Taichi', 'LLVM CPU'),
          cpuRow('Halide native CPU', 'Halide', 'Halide', 'native CPU'),
        ] },
        { title: 'Metal GPU', maximum: 500, rows: [
          gpuRow('Mech generated MSL', 'Mech', 'Mech'),
          gpuRow('Taichi native Metal', 'Taichi', 'Taichi'),
          gpuRow('Halide Metal schedule', 'Halide', 'Halide'),
        ] },
      ],
      archives: [cpu, metal],
      notes: ['2026-09-24 campaigns. Each system selects CPU and Metal from one application source.',
        'Ten process trials per mode. Panel scales differ; schedules and fault transport differ.'],
    },
  };
  // Validate every dataset before writing any chart.
  const rendered = Object.entries(charts).flatMap(([filename, configuration]) => [
    [filename, chart(configuration)],
    [filename.replace('.svg', '-mobile.svg'), mobileChart(configuration)],
  ]);
  mkdirSync(destination, { recursive: true });
  return rendered.map(([filename, svg]) => {
    const output = join(destination, filename);
    writeFileSync(output, svg);
    return output;
  });
}
