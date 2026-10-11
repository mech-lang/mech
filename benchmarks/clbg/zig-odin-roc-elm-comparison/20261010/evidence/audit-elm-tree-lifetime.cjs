'use strict';
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const crypto = require('node:crypto');
const root = path.resolve(__dirname, '..');
const binary = path.join(root, 'binaries/binarytrees-elm-faithful.js');
const source = fs.readFileSync(binary, 'utf8');
const injection = `
scope.TreeLifetimeAudit = function(n) {
  var maximum = Math.max(6, n), temporaryExpected = 0, nodesExpected = Math.pow(2, maximum + 2) - 1 + Math.pow(2, maximum + 1) - 1;
  for (var depth = 4; depth <= maximum; depth += 2) {
    var iterations = Math.pow(2, maximum - depth + 4);
    temporaryExpected += iterations;
    nodesExpected += iterations * (Math.pow(2, depth + 1) - 1);
  }
  var originalBuild = $author$project$BinarytreesFaithful$build;
  var originalCheck = $author$project$BinarytreesFaithful$check;
  var buildDepth = 0, checkDepth = 0, roots = 0, temporaryBuilt = 0, temporaryChecked = 0, checkedNodes = 0, longRoot = null, longChecks = 0;
  var nodes = new Set(), fields = null, leafCount = 0, interiorCount = 0;
  function assert(condition, message) { if (!condition) throw new Error(message); }
  $author$project$BinarytreesFaithful$build = F2(function(depth, identity) {
    var top = buildDepth++ === 0;
    if (top) roots++;
    var node;
    try { node = originalBuild.f(depth, identity); } finally { buildDepth--; }
    assert(!nodes.has(node), 'A tree node was shared or reused within the actual run');
    nodes.add(node);
    var keys = Object.keys(node).sort().join(',');
    if (fields === null) fields = keys;
    assert(fields === keys && keys === '$,a,b,c', 'Leaf/interior node fields differ');
    if (depth === 0) {
      assert(node.b === $author$project$BinarytreesFaithful$Empty && node.c === $author$project$BinarytreesFaithful$Empty, 'Leaf has nonempty children');
      leafCount++;
    } else {
      assert(node.b !== node.c, 'Interior children share one allocation');
      interiorCount++;
    }
    if (top) {
      if (roots === 1) assert(depth === maximum + 1, 'Stretch tree must be constructed first');
      else if (roots === 2) { assert(depth === maximum, 'Long-lived tree must follow stretch'); longRoot = node; }
      else temporaryBuilt++;
    }
    return node;
  });
  $author$project$BinarytreesFaithful$check = function(node) {
    var top = checkDepth++ === 0;
    if (node !== $author$project$BinarytreesFaithful$Empty) checkedNodes++;
    if (top && node === longRoot) {
      assert(temporaryBuilt === temporaryExpected && temporaryChecked === temporaryExpected, 'Long-lived root was checked before all temporary work completed');
      longChecks++;
    } else if (top && roots > 2) temporaryChecked++;
    try { return originalCheck(node); } finally { checkDepth--; }
  };
  var output;
  try { output = originalBuild && $author$project$BinarytreesFaithful$run.f(n, ''); }
  finally {
    $author$project$BinarytreesFaithful$build = originalBuild;
    $author$project$BinarytreesFaithful$check = originalCheck;
  }
  assert(nodes.size === nodesExpected && checkedNodes === nodesExpected, 'Incorrect number of constructed or traversed nodes');
  assert(roots === temporaryExpected + 2 && longChecks === 1, 'Incorrect tree count or final long-lived check');
  var leaf1 = originalBuild.f(0, 1), leaf2 = originalBuild.f(0, 1);
  assert(leaf1 !== leaf2 && leaf1.a === leaf2.a, 'Identical leaves were shared');
  return {n:n, maximum:maximum, distinctNodes:nodes.size, nodesExpected:nodesExpected, traversedNodes:checkedNodes, temporaryTrees:temporaryBuilt, temporaryChecks:temporaryChecked, finalLongLivedChecks:longChecks, leafCount:leafCount, interiorCount:interiorCount, equalLeafInteriorFields:true, freshIdenticalLeaves:true, actualRunCheckOrder:true, output:output};
};
`;
const marker = "_Platform_export({'BinarytreesFaithful':";
if (!source.includes(marker)) throw new Error('Expected compiled Elm module missing');
const context = {};
vm.createContext(context);
vm.runInContext(source.replace(marker, injection + '\n' + marker), context);
const results = [9, 10].map(n => context.TreeLifetimeAudit(n));
for (const result of results) {
  result.output_sha256 = crypto.createHash('sha256').update(result.output).digest('hex');
  delete result.output;
}
const report = {status:'passed', compiled_sha256:crypto.createHash('sha256').update(source).digest('hex'), cases:results,
  reason:'The actual compiled run was instrumented without modifying the measured binary. Every constructor call returns a distinct equal-field node; every node is traversed, both children are distinct, and the long-lived root is checked exactly once after every temporary tree. Generated depths carries the root in every tail-loop iteration until its terminal branch.'};
fs.writeFileSync(path.join(root, 'results/elm-tree-lifetime-final.json'), JSON.stringify(report, null, 2) + '\n');
process.stdout.write(JSON.stringify({status:report.status,cases:results.map(r=>({n:r.n,distinctNodes:r.distinctNodes,temporaryTrees:r.temporaryTrees,actualRunCheckOrder:r.actualRunCheckOrder}))})+'\n');
