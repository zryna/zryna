// Independently selected official HEAD observations retained by the operator, 2026-10-15.
// Product commit is checked from actual authenticated archive bytes, never inferred from CDN path.
export const EDITOR_VERSION = '1.138.0';
export const EDITOR_ARCHIVES = Object.freeze({
  linux: Object.freeze({ platform: 'linux-x64', bytes: 344482658,
    sha256: '59fb0f88eab2fe2e3a053ac020e18fcb2b5c113d255e41c82d26302b675d580c',
    url: 'https://vscode.download.prss.microsoft.com/dbazure/download/stable/7debcd0e2acdea1c52de81bf9ee1620444407dda/code-stable-x64-1789458585.tar.gz' }),
  win32: Object.freeze({ platform: 'win32-x64-archive', bytes: 341288168,
    sha256: 'c0a9f12a0d8962fa4cda1959eecd5fdd3f82de9715ad4dadd5601856bb60fd21',
    url: 'https://vscode.download.prss.microsoft.com/dbazure/download/stable/7debcd0e2acdea1c52de81bf9ee1620444407dda/VSCode-win32-x64-1.138.0.zip' }),
});
