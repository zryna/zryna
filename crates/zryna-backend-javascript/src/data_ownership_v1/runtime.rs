pub(super) const PRELUDE: &str = r#"const $zryna$U = void 0;
const $zryna$sentinel = {};
let $zryna$status = 0;
function $zryna$trap(id) {
  const code = {BOUNDS: 1, ALLOCATION: 2, CAPACITY: 3, REFCOUNT: 4, UTF8: 5}[id];
  if (code === void 0) throw new Error("ZRYNA-R3" + id);
  $zryna$status = code;
  throw $zryna$sentinel;
}
function $zryna$observation() { return $zryna$status; }
export { $zryna$observation };
function $zryna$index(value, length) {
  if ((value | 0) !== value || value < 0 || value >= length) $zryna$trap("BOUNDS");
  return value;
}
function $zryna$clone(value) {
  if (value === $zryna$U || value === null || typeof value !== "object") return value;
  switch (value.$k) {
    case 1: return {$k: 1, $v: value.$v};
    case 2: {
      const out = {$k: 2, $v: []};
      try { for (const item of value.$v) out.$v.push($zryna$clone(item)); }
      catch (failure) {
        for (let i = out.$v.length - 1; i >= 0; i--) $zryna$drop(out.$v[i]);
        throw failure;
      }
      return out;
    }
    case 3: return {$k: 3, $t: value.$t, $v: $zryna$clone(value.$v)};
    case 4:
      if (value.$c.$s === 4294967295) $zryna$trap("REFCOUNT");
      value.$c.$s++; return {$k: 4, $c: value.$c};
    case 5:
      if (value.$c.$w === 4294967295) $zryna$trap("REFCOUNT");
      value.$c.$w++; return {$k: 5, $c: value.$c};
    default: $zryna$trap("ABI");
  }
}
function $zryna$drop(value) {
  if (value === $zryna$U || value === null || typeof value !== "object" || value.$d) return;
  value.$d = true;
  if (value.$k === 1) return;
  if (value.$k === 2) { for (let i = value.$v.length - 1; i >= 0; i--) $zryna$drop(value.$v[i]); return; }
  if (value.$k === 3) { $zryna$drop(value.$v); return; }
  if (value.$k === 4) {
    if (value.$c.$s === 0) $zryna$trap("ABI");
    if (--value.$c.$s === 0) {
      $zryna$drop(value.$c.$p); value.$c.$p = $zryna$U;
      if (value.$c.$w === 0) $zryna$trap("ABI");
      value.$c.$w--;
    }
    return;
  }
  if (value.$k === 5) {
    if (value.$c.$w === 0) $zryna$trap("ABI");
    value.$c.$w--; return;
  }
  $zryna$trap("ABI");
}
function $zryna$read(places, roots, values, id) {
  const d = places[id];
  if (d[0] < 2) return roots[id];
  if (d[0] === 2) return values[d[1]];
  const base = $zryna$read(places, roots, values, d[1]);
  if (d[0] === 3 || d[0] === 5) return base.$v[d[2]];
  if (d[0] === 4) return base.$v;
  $zryna$trap("ABI");
}
function $zryna$write(places, roots, values, id, value) {
  const d = places[id];
  if (d[0] < 2) { roots[id] = value; return; }
  if (d[0] === 2) { values[d[1]] = value; return; }
  const base = $zryna$read(places, roots, values, d[1]);
  if (d[0] === 3 || d[0] === 5) { base.$v[d[2]] = value; return; }
  if (d[0] === 4) { base.$v = value; return; }
  $zryna$trap("ABI");
}
function $zryna$take(places, roots, values, id) {
  const value = $zryna$read(places, roots, values, id);
  $zryna$write(places, roots, values, id, $zryna$U);
  return value;
}
function $zryna$borrowRead(borrows, places, roots, values, id) {
  const b = borrows[id];
  if (b.$p !== $zryna$U) return $zryna$read(places, roots, values, b.$p);
  if (b.$r !== $zryna$U) {
    const root = $zryna$read(places, roots, values, b.$r);
    return root.$v[$zryna$index(b.$i, root.$v.length)];
  }
  const value = $zryna$borrowRead(borrows, places, roots, values, b.$b);
  return value.$v[$zryna$index(b.$i, value.$v.length)];
}
function $zryna$borrowWrite(borrows, places, roots, values, id, value) {
  const b = borrows[id];
  if (b.$p !== $zryna$U) { $zryna$write(places, roots, values, b.$p, value); return; }
  if (b.$r !== $zryna$U) {
    const root = $zryna$read(places, roots, values, b.$r);
    root.$v[$zryna$index(b.$i, root.$v.length)] = value; return;
  }
  const base = $zryna$borrowRead(borrows, places, roots, values, b.$b);
  base.$v[$zryna$index(b.$i, base.$v.length)] = value;
}

"#;
