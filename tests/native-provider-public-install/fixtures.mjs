// Original complete source programs. Observations are fixed before invocation.
export const cases = Object.freeze([
  { id: 'm1', profile: 'i32-v1', cliProfile: null, manifestProfile: 'zryna-m1-cli-v1', version: 1,
    files: { 'src/main.zry': 'export function main(): i32 { return 42; }\n' } },
  { id: 'm2', profile: 'control-flow-v1', cliProfile: 'control-flow-v1',
    manifestProfile: 'zryna-control-flow-v1', version: 2,
    files: {
      'src/main.zry': 'import { double } from "./math.zry";\nexport function main(): i32 { return double(21); }\n',
      'src/math.zry': 'export function double(value: i32): i32 { return value + value; }\n',
    } },
  { id: 'm3', profile: 'data-ownership-v1', cliProfile: 'data-ownership-v1',
    manifestProfile: 'zryna-data-ownership-v1', version: 3,
    files: { 'src/main.zry': 'interface Pair extends ZrynaStruct { left: i32; right: i32; }\nexport function main(): i32 { const value: Pair = Pair({ left: 19, right: 23 }); return value.left + value.right; }\n' } },
]);
