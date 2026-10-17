// These are existing Rust harness groups, not newly implemented acceptance cases.
export const selectors = Object.freeze({
  driver: ['zryna-driver', 'zryna_driver', 'lib', 'crates/zryna-driver/src/lib.rs'],
  cli: ['zryna', 'cli', 'test', 'apps/zryna/tests/cli.rs'],
  main: ['zryna', 'zryna', 'bin', 'apps/zryna/src/main.rs'],
  protocol: ['zryna-language-server', 'protocol', 'test', 'apps/zryna-language-server/tests/protocol.rs'],
  process: ['zryna-language-server', 'process', 'test', 'apps/zryna-language-server/tests/process.rs'],
});
export const project = Object.freeze([
  ['P1', 'driver', 'project::tests::'], ['P2', 'cli', 'new_project_'],
  ['P3', 'cli', 'standalone_project_'], ['P4', 'main', 'installed::tests::'],
]);
export const lsp = Object.freeze([['L1', 'protocol', ''], ['L2', 'process', '']]);
export const formatter = Object.freeze([
  ['F1', 'driver', 'diagnostic_sessions::tests::formatting::'],
  ['F2', 'driver', 'diagnostic_sessions::formatting::control_flow_tests::'],
  ['F3', 'driver', 'diagnostic_sessions::formatting::data_ownership_tests::'],
  ['F4', 'protocol', 'formatting::'],
]);
// Recorded target is Cargo; original test target/filter choices remain4/2/4.
export function cargoArgs(role, filter) {
  const [pkg, target, kind] = selectors[role];
  return ['test', '--locked', '--offline', '-p', pkg,
    ...(kind === 'lib' ? ['--lib'] : [kind === 'bin' ? '--bin' : '--test', target]),
    ...(filter ? [filter] : []), '--', '--nocapture'];
}
export const auxiliary = Object.freeze({
  project: { cli: ['zryna', 'zryna', 'bin', 'apps/zryna/src/main.rs'] },
  lsp: { server: ['zryna-language-server', 'zryna-language-server', 'bin', 'apps/zryna-language-server/src/main.rs'] },
  formatter: {}, // Its fixedgroups use driver/provider and inprocess protocol, no embeddedserver call.
});
