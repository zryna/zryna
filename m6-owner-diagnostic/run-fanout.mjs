import { fanout } from './diagnostic.mjs';
try { await fanout(); } catch (error) { console.error(String(error.stack ?? error)); process.exitCode = 1; }
