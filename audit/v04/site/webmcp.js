// Optional browser tools use the same handlers and visible state as the controls.
const context = document.modelContext;
const lifecycle = new AbortController();
window.addEventListener('pagehide', () => lifecycle.abort(), {once: true});
function emptyInput(input) {
  if (!input || typeof input !== 'object' || Array.isArray(input) || Object.keys(input).length) throw new Error('Expected an empty object.');
}
if (context?.registerTool) {
  const tool = {
    name: 'run_inventory_recorded_sequence',
    title: 'Run inventory sequence',
    description: 'Restart the visible inventory instance, execute its nine recorded turns, and return the resulting stock and rejection count.',
    inputSchema: {type: 'object', properties: {}, additionalProperties: false},
    annotations: {readOnlyHint: false, untrustedContentHint: false},
    execute(input) {
      emptyInput(input);
      if (!window.mechInventoryReady) throw new Error('Inventory executable is still loading.');
      document.getElementById('sequence').onclick();
      const history = window.mechInventoryHistory();
      return {attempts: history.length, stock: document.getElementById('stock').textContent, rejections: Number(document.getElementById('rejections').textContent), passed: history.every(row => row.check.passed)};
    },
  };
  try { Promise.resolve(context.registerTool(tool, {signal: lifecycle.signal})).catch(error => console.warn('Mech browser tool registration:', error)); }
  catch (error) { console.warn('Mech browser tool registration:', error); }
}
