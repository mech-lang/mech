// General process/port I/O only. Every benchmark algorithm is in Elm.
const fs = require('node:fs');
const path = require('node:path');
const compiled = require(path.resolve(process.argv[2])).Elm;
const application = Object.values(compiled).find(module => typeof module.init === 'function');
if (!application) throw new Error('No Elm worker exported.');
const app = application.init({flags: null});
app.ports.output.subscribe(({data, done}) => {
  const bytes = Buffer.from(data, 'latin1');
  let offset = 0;
  while (offset < bytes.length) {
    const written = fs.writeSync(1, bytes, offset, bytes.length - offset);
    if (written <= 0) throw new Error('stdout made no progress');
    offset += written;
  }
  if (done) process.exit(0);
});
const numeric = Number(process.argv[3] || 0);
const input = numeric ? '' : fs.readFileSync(0, 'latin1');
app.ports.input.send({n: numeric, input});
