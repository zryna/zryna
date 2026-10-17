import { natural } from './diagnostic.mjs';
try { await natural(); } catch (error) { console.error(String(error.stack ?? error)); process.exitCode = 1; }
