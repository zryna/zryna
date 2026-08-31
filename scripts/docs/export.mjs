import path from 'node:path';

import { compilerWorkspaceRoot, exportDocsBundle } from './bundle.mjs';

function argumentsByName(argumentsList) {
  const values = new Map();
  for (let index = 0; index < argumentsList.length; index += 2) {
    const name = argumentsList[index];
    const value = argumentsList[index + 1];
    if (!name?.startsWith('--') || value === undefined || values.has(name)) {
      throw new Error('expected unique --channel, --source-commit, --source-ref, and --output values');
    }
    values.set(name, value);
  }
  for (const required of ['--channel', '--source-commit', '--source-ref', '--output']) {
    if (!values.has(required)) throw new Error(`missing ${required}`);
  }
  if (values.size !== 4) throw new Error('unknown export option');
  return values;
}

try {
  const values = argumentsByName(process.argv.slice(2));
  const result = await exportDocsBundle({
    workspaceRoot: compilerWorkspaceRoot,
    channel: values.get('--channel'),
    sourceCommit: values.get('--source-commit'),
    sourceRef: values.get('--source-ref'),
    output: path.resolve(compilerWorkspaceRoot, values.get('--output')),
  });
  process.stdout.write(`${result.manifestSha256}\n`);
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
}
