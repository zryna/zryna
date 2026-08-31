import path from 'node:path';

import { compilerWorkspaceRoot, validateDocsBundle } from './bundle.mjs';

function argumentsByName(argumentsList) {
  const values = new Map();
  for (let index = 0; index < argumentsList.length; index += 2) {
    const name = argumentsList[index];
    const value = argumentsList[index + 1];
    if (!name?.startsWith('--') || value === undefined || values.has(name)) {
      throw new Error('expected unique documentation bundle validation options');
    }
    values.set(name, value);
  }
  const required = [
    '--bundle',
    '--expected-manifest-sha256',
    '--expected-channel',
    '--expected-source-commit',
    '--expected-source-ref',
  ];
  for (const name of required) if (!values.has(name)) throw new Error(`missing ${name}`);
  if (values.size !== required.length) throw new Error('unknown validation option');
  return values;
}

try {
  const values = argumentsByName(process.argv.slice(2));
  await validateDocsBundle(
    path.resolve(values.get('--bundle')),
    {
      expectedManifestSha256: values.get('--expected-manifest-sha256'),
      expectedChannel: values.get('--expected-channel'),
      expectedSourceCommit: values.get('--expected-source-commit'),
      expectedSourceRef: values.get('--expected-source-ref'),
    },
    compilerWorkspaceRoot,
  );
  process.stdout.write('Zryna documentation bundle is valid\n');
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
}
