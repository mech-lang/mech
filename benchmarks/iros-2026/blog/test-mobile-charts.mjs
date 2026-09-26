import assert from 'node:assert/strict';
import {mkdtempSync, readFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {buildCharts} from './charts.mjs';

// Independent of the page build: exercise only chart generation and geometry.
const output = mkdtempSync(join(tmpdir(), 'mech-mobile-charts-test-'));
const paths = buildCharts(output);
assert.equal(paths.length, 8, 'four desktop and four mobile variants');
const decode = value => value.replaceAll('&quot;', '"').replaceAll('&apos;', "'")
  .replaceAll('&lt;', '<').replaceAll('&gt;', '>').replaceAll('&amp;', '&');
const metadata = svg => JSON.parse(decode(svg.match(/<metadata>(.*?)<\/metadata>/s)[1]));
const attributes = source => Object.fromEntries(
  [...source.matchAll(/([\w-]+)="([^"]*)"/g)].map(match => [match[1], decode(match[2])]),
);
const textNodes = svg => [...svg.matchAll(/<text\b([^>]*)>(.*?)<\/text>/gs)]
  .map(match => ({...attributes(match[1]), value: decode(match[2])}));
const bars = svg => [...svg.matchAll(/<g\b([^>]*data-mode="[^"]+"[^>]*)>(.*?)<\/g>/gs)]
  .map(match => ({...attributes(match[1]), content: match[2]}));

for (const name of ['cpu', 'backends', 'metal', 'portable']) {
  const desktop = readFileSync(join(output, `${name}.svg`), 'utf8');
  const mobile = readFileSync(join(output, `${name}-mobile.svg`), 'utf8');
  assert.deepEqual(metadata(mobile), metadata(desktop), `${name}: exact archived data identity`);
  const [, width, height] = mobile.match(/viewBox="0 0 (\d+) (\d+)"/).map(Number);
  assert.equal(width, 560, `${name}: mobile viewBox`);
  const mobileBars = bars(mobile), desktopBars = bars(desktop);
  assert.equal(mobileBars.length, desktopBars.length, `${name}: bar count`);
  mobileBars.forEach((bar, index) => {
    const desktopBar = desktopBars[index];
    for (const key of ['data-language', 'data-mode', 'data-source', 'data-median', 'data-mad', 'data-n']) {
      assert.equal(bar[key], desktopBar[key], `${name}: ${key}`);
    }
    assert.equal(bar['data-n'], '10');
    assert.equal(bar['data-mode'], index % 2 ? 'checked' : 'unchecked');
    const rectangle = attributes(bar.content.match(/<rect\b([^>]*)/)[1]);
    assert.equal(Number(rectangle.x), 20);
    assert(Number(rectangle.width) > 0);
    assert(Number(rectangle.x) + Number(rectangle.width) <= 352.000001);
    assert.equal(Number(rectangle.height), 24);
    const measurements = textNodes(bar.content);
    assert.deepEqual(measurements.map(node => Number(node.x)), [438, 464, 540]);
    assert.deepEqual(measurements.map(node => node['text-anchor']), ['end', 'middle', 'end']);
    assert(measurements.every(node => node.class === 'measurement'));
    assert(measurements.every(node => Number(node.y) >= 20 && Number(node.y) < height - 20));
    // Numeric glyphs are narrower than this conservative 0.66em budget.
    const medianLeft = 438 - measurements[0].value.length * 23 * 0.66;
    const madLeft = 540 - measurements[2].value.length * 23 * 0.66;
    assert(medianLeft > 352, `${name}: median must clear the plot`);
    assert(madLeft > 478, `${name}: MAD must clear the plus/minus column`);
    if (index % 2) {
      const upper = attributes(mobileBars[index - 1].content.match(/<rect\b([^>]*)/)[1]);
      assert.equal(Number(rectangle.y) - Number(upper.y), 30, `${name}: dark upper, light lower`);
    }
  });
  const texts = textNodes(mobile);
  for (const node of texts) {
    assert(Number(node.x) >= 20 && Number(node.x) <= width - 20, `${name}: text x bounds`);
    assert(Number(node.y) >= 20 && Number(node.y) <= height - 10, `${name}: text y bounds`);
  }
  const labels = texts.filter(node => node.class === 'row-label');
  assert.equal(labels.length, mobileBars.length / 2);
  labels.forEach((label, index) => {
    assert.equal(Number(label.x), 20);
    assert(label.value.length * 22 * 0.64 <= 520, `${name}: row label width budget`);
    const rectangle = attributes(mobileBars[index * 2].content.match(/<rect\b([^>]*)/)[1]);
    assert.equal(Number(rectangle.y) - Number(label.y), 15, `${name}: label above paired bars`);
  });
  const axes = texts.filter(node => node.class === 'axis-label');
  assert(axes.some(node => node.value === 'Throughput (million filter-turns/s)'));
  assert(axes.some(node => node.value === (name === 'backends' ? 'Logarithmic scale' : 'Linear scale')));
  if (name === 'backends') {
    assert.deepEqual(texts.filter(node => node.class === 'tick').map(node => node.value), ['0.1', '1', '10', '100', '1000']);
  }
  if (name === 'portable') {
    const panels = texts.filter(node => node.class === 'panel-title');
    assert.deepEqual(panels.map(node => node.value), ['CPU · eight workers', 'Metal GPU']);
    assert(Number(panels[1].y) > Number(panels[0].y) + 3 * 118, 'portable panels stack vertically');
  }
  const svgId = mobile.match(/<svg[^>]*\bid="([^"]+)"/)[1];
  assert(svgId.endsWith('-mobile'));
  assert.notEqual(svgId, desktop.match(/<svg[^>]*\bid="([^"]+)"/)[1]);
  assert(mobile.includes('<title') && mobile.includes('<desc'), 'accessible figure');
  assert(mobile.includes(`url(#${svgId}-gpu-hatch)`), 'local GPU hatching');
}
console.log('PASS: four mobile charts preserve samples, n=10, scales, paired order, aligned values, and bounded geometry.');
