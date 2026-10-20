// Actual source-candidate producer. No network acquisition, release or publication authority.
import { chmodSync, existsSync, lstatSync, mkdirSync, realpathSync } from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { bytes, parseCanonical, requireValue, sha256 } from './canonical.mjs';
import { targetPaths } from './inventory.mjs';
import { nativeRecipe, NATIVE_RECIPE_SHA256, nativeRustMaterials } from './native-inventory-v2.mjs';
import {
  prepareNativePayload, assembleNativeInstallation, verifyNativeArchive,
} from './native-distribution-v2.mjs';
import { requireArchiveRuntime } from './runtime.mjs';
import { checkQualificationRustClosure } from './rust-materials.mjs';
import { extractVerifiedProductionFiles } from '../distribution-release/run-installed-acceptance.mjs';
import { validateSourceReceipt } from './source-receipt.mjs';
import {
  executeRecorded, ordinaryBytes, pinBytes, saveEvidence, successful,
} from '../../tests/native-provider-public-install/commands.mjs';
import { captureProducerExecutable, verifyProducerExecutableCapture } from './native-producer-capture.mjs';
import { selectNativeSourceRef } from './native-source-ref-v2.mjs';
import { runNativeInstalledSmoke } from '../../tests/native-provider-public-install/run.mjs';

const INPUTS = ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json'];
const ARCHITECTURE = ['cargo', 'run', '--locked', '-p', 'zryna', '--', 'architecture', 'check', '--json'];
const RECIPE_SHA256 = '0f522abd343ba4e3c07a38d93c778fa7a0e30aac2295abe1f075091f6b6d7db1';
const CARGO = 'cargo 1.97.1 (c980f4866 2026-06-30)';
const RUSTC = 'rustc 1.97.1 (8bab26f4f 2026-07-14)';

function rejectCargoConfiguration(path) {
  for (const name of ['config', 'config.toml']) {
    requireValue(!lstatSync(join(path, name), { throwIfNoEntry: false }),
      'native producer forbids Cargo config overrides');
  }
}

function outsideSource(source, output) {
  const delta = relative(source, output);
  return delta && (isAbsolute(delta) || delta.split(/[\\/]/)[0] === '..');
}

export async function produceNativeInstallation({ source, output, jobs = 2 }) {
  requireArchiveRuntime();
  source = realpathSync.native(resolve(source)); output = resolve(output);
  requireValue(isAbsolute(source) && isAbsolute(output) && outsideSource(source, output)
    && realpathSync.native(dirname(output)) === dirname(output) && !existsSync(output)
    && Number.isSafeInteger(jobs) && jobs >= 1 && jobs <= 4, 'fresh external native producer roots/jobs');
  mkdirSync(output, { mode: 0o700 });
  const logs = join(output, 'producer-commands'); mkdirSync(logs, { mode: 0o700 });
  let serial = 0;
  const execute = (name, executable, args, env = process.env, timeoutMs = 120_000) =>
    executeRecorded(logs, String(++serial).padStart(3, '0') + '-' + name,
      executable, args, source, env, { timeoutMs, maximum: 64 * 1024 * 1024 });
  const text = (name, executable, args, env) => successful(execute(name, executable, args, env)).toString('utf8').trim();
  let stage = 'exact-source-and-builder-admission';
  try {
    const head = text('source-head', 'git', ['rev-parse', 'HEAD']);
    const tree = text('source-tree', 'git', ['show', '-s', '--format=%T', 'HEAD']);
    const refObservation = execute('source-ref', 'git', ['symbolic-ref', '--quiet', 'HEAD']);
    requireValue([0, 1].includes(refObservation.exitCode), 'actual symbolic source ref observation');
    const attachedRef = refObservation.exitCode === 0 ? refObservation.stdoutBytes.toString('utf8').trim() : null;
    const selectedRef = selectNativeSourceRef({ head, attachedRef, env: process.env });
    const branch = selectedRef.ref;
    saveEvidence(output, 'source-ref.json', selectedRef);
    const epoch = Number(text('source-epoch', 'git', ['show', '-s', '--format=%ct', 'HEAD']));
    requireValue(/^[a-f0-9]{40}$/.test(head) && /^[a-f0-9]{40}$/.test(tree), 'exact source Git identity');
    const assertCurrent = () => {
      requireValue(text('unchanged-head', 'git', ['rev-parse', 'HEAD']) === head
        && text('clean-source', 'git', ['status', '--porcelain=v1', '--untracked-files=all']) === '',
      'candidate must be committed and unchanged');
    };
    assertCurrent();
    const target = process.platform === 'win32' ? 'x86_64-pc-windows-msvc' : 'x86_64-unknown-linux-gnu';
    const paths = targetPaths(target);
    const cargoHome = realpathSync.native(process.env.CARGO_HOME ?? '');
    requireValue(isAbsolute(cargoHome), 'explicit controlled CARGO_HOME required');
    rejectCargoConfiguration(cargoHome);
    for (let ancestor = source; ; ancestor = dirname(ancestor)) {
      rejectCargoConfiguration(join(ancestor, '.cargo'));
      if (ancestor === dirname(ancestor)) break;
    }
    const captureRoot = join(output, 'producer-executables'); mkdirSync(captureRoot, { mode: 0o700 });
    const rustupSource = realpathSync.native(join(cargoHome, 'bin', process.platform === 'win32' ? 'rustup.exe' : 'rustup'));
    const rustupProof = captureProducerExecutable({ source: rustupSource,
      destination: join(captureRoot, process.platform === 'win32' ? 'rustup.exe' : 'rustup'),
      aliasDirectory: dirname(rustupSource), kind: 'rustup-shim' });
    saveEvidence(output, 'rustup-capture.json', rustupProof);
    const rustup = rustupProof.capture.path;
    const selectTool = name => realpathSync.native(text(name + '-selection', rustup,
      ['which', '--toolchain', '1.97.1', name]));
    const cargo = selectTool('cargo'), rustc = selectTool('rustc');
    requireValue(text('cargo-version', cargo, ['--version']) === CARGO
      && text('rustc-version', rustc, ['--version']) === RUSTC, 'exact Rust builder versions');
    const linker = realpathSync.native(process.platform === 'win32'
      ? text('linker-selection', join(process.env.SystemRoot, 'System32/where.exe'), ['link.exe']).split(/\r?\n/)[0]
      : '/usr/bin/cc');
    const tools = Object.fromEntries([['rustup', rustup], ['cargo', cargo], ['rustc', rustc], ['linker', linker],
      ['archive-node', realpathSync.native(process.execPath)]]
      .map(([name, path]) => [name, pinBytes(path, ordinaryBytes(path))]));
    const originalRecipeBytes = ordinaryBytes(join(source, 'scripts/distribution/release-recipe-v1.json'), 262144);
    requireValue(sha256(originalRecipeBytes) === RECIPE_SHA256, 'original deterministic compile recipe');
    const compile = parseCanonical(originalRecipeBytes).compile.targets[target];
    const builderEnvironment = replica => {
      const work = join(output, replica), targetRoot = join(work, 'target'), temp = join(work, 'temp');
      if (!existsSync(work)) {
        mkdirSync(work, { mode: 0o700 }); mkdirSync(targetRoot, { mode: 0o700 }); mkdirSync(temp, { mode: 0o700 });
      }
      const replacements = { '@source-root@': source, '@work-root@': work, '@cargo-home@': cargoHome,
        '@target-root@': targetRoot, '@temp-root@': temp, '@rustc@': rustc, '@linker@': linker,
        '@source-date-epoch@': String(epoch), '@qualification-binding-sha256@': '' };
      const replace = value => {
        for (const [token, replacement] of Object.entries(replacements)) value = value.replaceAll(token, replacement);
        value = value.replace(/@host-([A-Za-z_]+)@/g, (_, name) => process.env[name] ?? '');
        requireValue(!/@[A-Za-z0-9_-]+@/.test(value), 'unresolved native compile token'); return value;
      };
      const env = Object.fromEntries(compile.environment.map(({ name, value }) => [name, replace(value)]));
      delete env.ZRYNA_DISTRIBUTION_SHA256;
      for (const key of Object.keys(env)) if (key.startsWith('CARGO_PROFILE_RELEASE_')) delete env[key];
      Object.assign(env, { CARGO: cargo, CARGO_BUILD_TARGET: target, CARGO_BUILD_JOBS: String(jobs),
        CARGO_PROFILE_DEV_DEBUG: '0', CARGO_PROFILE_DEV_INCREMENTAL: 'false',
        CARGO_PROFILE_DEV_STRIP: 'symbols', CARGO_PROFILE_DEV_CODEGEN_UNITS: '1',
        RUSTUP_HOME: realpathSync.native(process.env.RUSTUP_HOME) });
      env.CARGO_ENCODED_RUSTFLAGS = [...compile.encodedRustFlags.map(replace),
        `--remap-path-prefix=${cargoHome}=/zryna/cargo-home`].join('\x1f');
      return env;
    };
    const firstEnvironment = builderEnvironment('replica-a');
    saveEvidence(output, 'builder.json', { source: { head, tree, branch, epoch }, target, tools,
      jobs, buildProfile: 'dev-source-candidate', originalCompileRecipeSha256: RECIPE_SHA256,
      sourceArchitectureArgv: ARCHITECTURE,
      independentBuildRoots: ['replica-a', 'replica-b'],
      environment: Object.fromEntries(Object.entries(firstEnvironment)
        .filter(([key]) => !['INCLUDE', 'LIB', 'LIBPATH', 'PATH'].includes(key))) });
    stage = 'actual-source-architecture-command';
    const report = JSON.parse(successful(execute('source-architecture', cargo, ARCHITECTURE.slice(1),
      firstEnvironment, 45 * 60_000)));
    requireValue(bytes(report).equals(bytes({ diagnostics: [] })), 'actual successful architecture report required');
    const receipt = bytes({ format: 'zryna.source-build-receipt.v1',
      source: { repository: 'https://github.com/zryna/zryna', commit: head, tree }, command: ARCHITECTURE,
      toolchain: { channel: '1.97.1', cargoVersion: CARGO, cargoSha256: tools.cargo.sha256,
        rustcVersion: RUSTC, rustcSha256: tools.rustc.sha256 }, report,
      inputs: INPUTS.map(logicalPath => {
        const body = ordinaryBytes(join(source, logicalPath), 262144);
        return { logicalPath, size: body.length, sha256: sha256(body) };
      }) });
    saveEvidence(output, 'architecture-receipt.json', receipt);
    const identity = { version: nativeRecipe.compilerVersion,
      source: { repository: nativeRecipe.sourceRepository, ref: branch,
        commit: head, tree, sourceDateEpoch: epoch },
      target: { triple: target, archiveFormat: paths.format, platformBaseline: process.platform === 'win32'
        ? { os: 'windows', product: 'windows-server', version: '2022', architecture: 'x86_64', runtime: 'operating-system-ucrt' }
        : { os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64' } },
      recipe: { format: nativeRecipe.format, sha256: NATIVE_RECIPE_SHA256 } };
    validateSourceReceipt(receipt, identity); assertCurrent();
    stage = 'actual-frozen-Rust-supplier-capture';
    const runtimeClosure = checkQualificationRustClosure({ cwd: source, materialRecords: nativeRustMaterials,
      spawn: (executable, args) => {
        const observed = execute('supplier-closure', executable === 'rustup' ? rustup : executable,
          args, firstEnvironment);
        return { status: observed.exitCode, signal: observed.signal, error: null,
          stdout: observed.stdoutBytes.toString('utf8'), stderr: observed.stderrBytes.toString('utf8') };
      } });
    saveEvidence(output, 'rust-closure.json', runtimeClosure);
    const metadata = JSON.parse(successful(execute('cargo-metadata', cargo,
      ['metadata', '--format-version=1', '--locked', '--offline', '--filter-platform', target], firstEnvironment)));
    const captured = [{ path: 'LICENSE', mode: 0o644, data: ordinaryBytes(join(source, 'LICENSE'), 2097152) }];
    for (const record of nativeRustMaterials(target)) {
      const pkg = metadata.packages.find(item => item.name === record.name && item.version === record.version);
      requireValue(pkg?.source === 'registry+https://github.com/rust-lang/crates.io-index', 'native Rust registry material identity');
      for (const file of record.files) {
        const owner = file.origin.startsWith('https://raw.githubusercontent.com/bytecodealliance/wasmtime/')
          ? metadata.packages.find(item => item.name === 'wasmtime' && item.version === '48.0.1') : pkg;
        requireValue(owner, 'native upstream notice package missing');
        const data = ordinaryBytes(join(dirname(owner.manifest_path), file.sourcePath), 2097152);
        requireValue(data.length === file.size && sha256(data) === file.sha256, 'whole native Rust notice pin');
        captured.push({ path: file.path, mode: 0o644, data });
      }
    }
    captured.sort((left, right) => left.path < right.path ? -1 : 1);
    const prepared = prepareNativePayload(identity, captured, receipt);
    const manifestPin = saveEvidence(output, 'prepared-distribution.json', prepared.distribution);
    const replicas = [];
    for (const replica of ['replica-a', 'replica-b']) {
      stage = replica + '-actual-ordinary-compiler-build'; assertCurrent();
      const env = builderEnvironment(replica); env.ZRYNA_DISTRIBUTION_SHA256 = manifestPin.sha256;
      const args = ['rustc', '--locked', '--offline', '--target', target, '-p', 'zryna', '--bin', 'zryna'];
      successful(execute(replica + '-compile', cargo, args, env, 45 * 60_000)); assertCurrent();
      const cliPath = join(env.CARGO_TARGET_DIR, target, 'debug', process.platform === 'win32' ? 'zryna.exe' : 'zryna');
      const cliCapture = captureProducerExecutable({ source: cliPath,
        destination: join(captureRoot, replica + (process.platform === 'win32' ? '.exe' : '')),
        aliasDirectory: join(dirname(cliPath), 'deps'), kind: 'cargo-cli' });
      saveEvidence(output, replica + '-compiler-capture.json', cliCapture);
      const cli = ordinaryBytes(cliCapture.capture.path), assembled = await assembleNativeInstallation(prepared, cli);
      const verified = await verifyNativeArchive(assembled.archive, assembled.subject);
      const archivePin = saveEvidence(output, replica + '-' + assembled.subject.filename, assembled.archive);
      saveEvidence(output, replica + '-subject.json', assembled.subject);
      replicas.push({ replica, cliPath, cliCapture, cli, ...assembled, verified, archivePin });
    }
    stage = 'actual-two-build-reproduction';
    requireValue(replicas[0].cli.equals(replicas[1].cli)
      && replicas[0].archive.equals(replicas[1].archive), 'two independent native builds differ');
    const reproduction = { status: 'two-actual-independent-native-builds-byte-identical',
      source: identity.source, target, preparedManifestSha256: manifestPin.sha256,
      cliSha256: sha256(replicas[0].cli), archiveSha256: sha256(replicas[0].archive),
      replicas: replicas.map(value => ({ replica: value.replica, cliPath: value.cliPath, capture: value.cliCapture, archive: value.archivePin })),
      dependencyBuildCachesDistinct: true, secondCachedCompilerCountedAsRebuild: false };
    saveEvidence(output, 'reproduction.json', reproduction); assertCurrent();
    stage = 'actual-verified-relocated-native-installation';
    const installation = join(output, 'fresh-native-install');
    extractVerifiedProductionFiles(installation, replicas[0].verified.files);
    if (process.platform !== 'win32') for (const file of replicas[0].verified.files) {
      const path = join(installation, file.path); chmodSync(path, file.mode);
      requireValue((lstatSync(path).mode & 0o7777) === file.mode, 'normalized native installation modes');
    }
    const smoke = await runNativeInstalledSmoke({ source: identity.source, output, installation,
      target, verifiedFiles: replicas[0].verified.files });
    assertCurrent();
    for (const tool of Object.values(tools)) requireValue(sha256(ordinaryBytes(tool.path)) === tool.sha256,
      'builder tool changed during qualification');
    verifyProducerExecutableCapture(rustupProof);
    for (const value of replicas) verifyProducerExecutableCapture(value.cliCapture);
    const result = { format: 'zryna.native-public-installation-qualification.v1',
      status: 'actual-native-build-reproduction-and-ordinary-installed-cli-passed',
      source: identity.source, observedSourceRef: branch, target, preparedManifest: manifestPin,
      reproduction, archive: replicas[0].subject, ordinaryInstalledSmoke: smoke,
      productionAdmission: 'forbidden', outerSignatureAuthentication: 'not-performed',
      privateActivationHarnessUsed: false, sourceCleanBeforeAndAfter: true };
    saveEvidence(output, 'RESULT.json', result); console.log(JSON.stringify({ status: result.status, output, target }));
    return result;
  } catch (error) {
    saveEvidence(output, 'FAILURE.json', { status: 'failed-no-native-installation-acceptance-claim',
      stage, type: error.name, message: String(error.message).slice(0, 2000),
      cleanupConfirmed: false, retainedOwnedInputs: true, retry: 'Diagnose retained actual failure; never relabel or rerun unchanged.' });
    throw error;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { values } = parseArgs({ options: { source: { type: 'string' }, output: { type: 'string' },
    jobs: { type: 'string', default: '2' } }, strict: true });
  requireValue(values.source && values.output && /^[1-4]$/.test(values.jobs),
    'usage: --source <clean-checkout> --output <fresh-external-directory> [--jobs 2]');
  await produceNativeInstallation({ source: values.source, output: values.output, jobs: Number(values.jobs) });
}
