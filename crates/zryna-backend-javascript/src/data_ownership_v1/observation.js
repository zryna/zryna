const $zryna$sentinel = {};
let $zryna$tracing = false;
let $zryna$status = 0, $zryna$trace = [], $zryna$fault = 0, $zryna$at = 0, $zryna$attempt = 0;
function $zryna$trap(id) {
  const code = {BOUNDS: 1, ALLOCATION: 2, CAPACITY: 3, REFCOUNT: 4, UTF8: 5}[id];
  if (code === void 0) throw new Error("ZRYNA-R3" + id);
  if ($zryna$status === 0) $zryna$status = code;
  throw $zryna$sentinel;
}
function $zryna$record(word) {
  if (!$zryna$tracing) return;
  if ($zryna$trace.length >= 4096) { $zryna$trace.length = 4097; return; }
  $zryna$trace.push(word);
}
function $zryna$probe(code) {
  if (code === $zryna$fault && ++$zryna$attempt === $zryna$at) {
    $zryna$status = code;
    throw $zryna$sentinel;
  }
}
function $zryna$observation(command) {
  if ((command & 0xf0000000) === 0x20000000) {
    $zryna$tracing = true;
    $zryna$fault = (command >>> 24) & 15;
    $zryna$at = command & 0xffffff;
    $zryna$attempt = 0;
    return 0;
  }
  if ((command & 0xf0000000) === 0x40000000) {
    const index = command & 0xfffffff;
    return index === 0 ? $zryna$trace.length : $zryna$trace[index - 1];
  }
  if (command !== 0) throw new Error("ZRYNA-R3ABI");
  return $zryna$status;
}
