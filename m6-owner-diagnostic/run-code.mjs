import { code } from './diagnostic.mjs';
try { await code(); } catch (error) { console.error(String(error.stack ?? error)); process.exitCode = 1; }
