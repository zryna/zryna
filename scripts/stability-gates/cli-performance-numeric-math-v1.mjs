// Exact arithmetic only. Decimal native integers are never routed through Number.
import assert from 'node:assert/strict';
import { exact } from './input.mjs';
const U64 = (1n << 64n) - 1n;
export const MAX_WALL_NS = 15000000000n; // Existing reviewed raw hosted guard, not a performance SLO.
export function integer(value, zero = false) {
  assert(typeof value === 'string' && /^(0|[1-9][0-9]{0,19})$/u.test(value), 'canonical u64 decimal');
  const result = BigInt(value);
  assert(result <= U64 && (zero || result > 0n), 'native u64 range');
  return result;
}
function gcd(a, b) {
  while (b !== 0n) [a, b] = [b, a % b];
  return a;
}
export function fraction(numerator, denominator = 1n) {
  assert(numerator >= 0n && denominator > 0n);
  const divisor = gcd(numerator, denominator);
  return { numerator: String(numerator / divisor), denominator: String(denominator / divisor) };
}
export function ratio(value, atLeastOne = false) {
  exact(value, ['numerator', 'denominator'], 'positive policy ratio');
  const n = integer(value.numerator), d = integer(value.denominator);
  if (atLeastOne) assert(n >= d, 'cross-round and regression ratios cannot be below one');
  return value;
}
export function compare(a, b) {
  const difference = BigInt(a.numerator) * BigInt(b.denominator) - BigInt(b.numerator) * BigInt(a.denominator);
  return difference < 0n ? -1 : difference > 0n ? 1 : 0;
}
export function divide(a, b) {
  assert(BigInt(b.numerator) > 0n);
  return fraction(BigInt(a.numerator) * BigInt(b.denominator), BigInt(a.denominator) * BigInt(b.numerator));
}
export function subtract(a, b) {
  return fraction(BigInt(a.numerator) * BigInt(b.denominator) - BigInt(b.numerator) * BigInt(a.denominator),
    BigInt(a.denominator) * BigInt(b.denominator));
}
export const maximum = (a, b) => compare(a, b) >= 0 ? a : b;
export function summarize(values) {
  assert.equal(values.length, 5, 'all five values required');
  const ordered = [...values].sort(compare);
  return { median: ordered[2], maximum: ordered[4],
    spread: divide(subtract(ordered[4], ordered[0]), ordered[2]) };
}
