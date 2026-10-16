import { requireValue } from '../../distribution/canonical.mjs';

export const COMPILER_TARGET = 'x86_64-unknown-linux-gnu';
const VALUES = Object.freeze({ target_abi: [''], target_arch: ['x86_64'], target_endian: ['little'],
  target_env: ['gnu'], target_family: ['unix'], target_feature: ['fxsr', 'sse', 'sse2'],
  target_has_atomic: ['8', '16', '32', '64', 'ptr'], target_os: ['linux'],
  target_pointer_width: ['64'], target_vendor: ['unknown'] });

// A closed target-only cfg grammar supplements the builder's retained --filter-platform call.
// Unknown cfg keys cannot silently select another compilation recipe.
export function compilerTargetMatches(value) {
  if (value === null || value === COMPILER_TARGET) return true;
  requireValue(typeof value === 'string' && value.length > 0 && value.length <= 2048,
    'compiler target condition budget');
  if (!value.startsWith('cfg(')) {
    requireValue(/^[A-Za-z0-9_.-]+$/.test(value), 'compiler target triple');
    return false;
  }
  const tokens = [];
  const pattern = /\s*("(?:[^"\\]|\\.)*"|[A-Za-z_][A-Za-z0-9_]*|[(),=])/y;
  let offset = 0;
  while (offset < value.length) {
    if (/^\s*$/.test(value.slice(offset))) break;
    pattern.lastIndex = offset;
    const match = pattern.exec(value);
    requireValue(match && tokens.length < 256, 'compiler target cfg token');
    tokens.push(match[1]);
    offset = pattern.lastIndex;
  }
  let cursor = 0;
  const take = token => { requireValue(tokens[cursor++] === token, 'compiler target cfg grammar'); };
  function expression(depth) {
    requireValue(depth <= 12, 'compiler target cfg depth');
    const name = tokens[cursor++];
    if (tokens[cursor] === '(') {
      requireValue(['all', 'any', 'not'].includes(name), 'compiler target cfg operation');
      take('(');
      const args = [];
      while (tokens[cursor] !== ')') {
        args.push(expression(depth + 1));
        if (tokens[cursor] !== ',') break;
        take(',');
      }
      take(')');
      requireValue(name !== 'not' || args.length === 1, 'compiler target cfg negation');
      return name === 'not' ? !args[0] : name === 'all' ? args.every(Boolean) : args.some(Boolean);
    }
    if (tokens[cursor] === '=') {
      take('=');
      const token = tokens[cursor++];
      requireValue(Object.hasOwn(VALUES, name) && token?.startsWith('"'), 'compiler target cfg key');
      const selected = JSON.parse(token);
      requireValue(typeof selected === 'string', 'compiler target cfg string');
      return VALUES[name].includes(selected);
    }
    requireValue(['unix', 'windows'].includes(name), 'compiler target cfg flag');
    return name === 'unix';
  }
  take('cfg'); take('(');
  const result = expression(0);
  take(')');
  requireValue(cursor === tokens.length, 'compiler target cfg trailing tokens');
  return result;
}
