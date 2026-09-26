// Use the same complete source for formatting, encoded AST, and resident REPL
// reflection. Standard-library imports are resolved by the v0.4 catalog.
export function documentSourceBundle(source) {
  if (typeof source !== 'string' || !source.trim()) {
    throw new Error('The resident document requires its complete Mechdown source.');
  }
  return Buffer.from(JSON.stringify({
    version: 2,
    rootSpecifier: 'article.mec',
    sources: [{ specifier: 'article.mec', source }],
    resolutions: [],
  }), 'utf8').toString('base64');
}
