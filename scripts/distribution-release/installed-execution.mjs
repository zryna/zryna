import { basename, join } from 'node:path';
import {
  cleanInstalledEnvironment, exactInstalledOutput, removeVerifiedProductionFiles,
} from './installed-lifecycle.mjs';

const MAX_OUTPUT = 256 * 1024;
const TIMEOUT = 5 * 60_000;

function reject(message) {
  throw new Error(`R406-INSTALLED-ACCEPTANCE: ${message}`);
}

function requireDirectory(system, path) {
  const entry = system.lstat(path, { throwIfNoEntry: false });
  if (!entry?.isDirectory() || entry.isSymbolicLink()) reject('installation ancestor is unsafe');
}

function command(spawn, executable, args, cwd, env) {
  const result = spawn(executable, args, {
    cwd, encoding: null, maxBuffer: MAX_OUTPUT + 1, timeout: TIMEOUT,
    windowsHide: true, shell: false, env,
  });
  const stdout = Buffer.isBuffer(result.stdout) ? result.stdout : Buffer.alloc(0);
  const stderr = Buffer.isBuffer(result.stderr) ? result.stderr : Buffer.alloc(0);
  if (result.error || result.signal !== null || stdout.length + stderr.length > MAX_OUTPUT) {
    reject(`installed command ${args[0]} did not complete within its bounds`);
  }
  return { status: result.status, stdout, stderr };
}

function success(spawn, executable, args, cwd, env) {
  const result = command(spawn, executable, args, cwd, env);
  if (result.status !== 0) reject(`installed command ${args[0]} failed`);
  return result;
}

function mutate(system, path) {
  const entry = system.lstat(path);
  if (!entry.isFile() || entry.isSymbolicLink() || entry.size < 1) reject('tamper subject is unsafe');
  system.write(path, Buffer.alloc(entry.size, 0x78), { flag: 'r+' });
}

export function executeInstalledAcceptance({
  archiveSha256, extract, provider, spawn, system, target, targetTriple, verifiedFiles,
  version, workRoot,
}) {
  requireDirectory(system, workRoot);
  const root = system.makeTemp(join(workRoot, 'zryna-installed-acceptance-'));
  const install = (label) => extract(join(root, label), verifiedFiles, system);
  try {
    const collision = join(root, 'collision');
    system.make(collision, { recursive: false, mode: 0o700 });
    system.write(join(collision, 'unrelated.txt'), Buffer.from('retain\n'), { flag: 'wx' });
    let collisionRejected = false;
    try {
      extract(collision, verifiedFiles, system);
    } catch {
      collisionRejected = true;
    }
    if (!collisionRejected || !system.read(join(collision, 'unrelated.txt')).equals(Buffer.from('retain\n'))) {
      reject('installation path collision was not preserved');
    }

    const staged = install('staged');
    const relocated = join(root, 'relocated');
    system.move(staged, relocated);
    requireDirectory(system, relocated);
    const executable = join(relocated, ...target.cli.split('/'));
    const projectParent = join(root, 'projects');
    system.make(projectParent, { recursive: false, mode: 0o700 });
    const environment = cleanInstalledEnvironment(executable, root);
    const observedVersion = success(spawn, basename(executable), ['--version'], projectParent,
      environment);
    if (!exactInstalledOutput(observedVersion, `zryna ${version}`)) {
      reject('installed version output differs');
    }
    const localeVersion = success(spawn, basename(executable), ['--version'], projectParent,
      cleanInstalledEnvironment(executable, root, 'tr_TR.UTF-8'));
    if (!exactInstalledOutput(localeVersion, `zryna ${version}`)) reject('locale changed version output');
    success(spawn, executable, ['new', 'hello'], projectParent, environment);
    for (const [operation, targetName, name] of [
      ['build', 'javascript', 'js-build'], ['run', 'javascript', 'js-run'],
      ['build', 'webassembly', 'wasm-build'], ['run', 'webassembly', 'wasm-run'],
    ]) {
      const args = [operation, 'src/main.zry', '--project-root', 'hello', '--target', targetName,
        '--name', name];
      if (operation === 'run') args.push('--export', 'main');
      const result = success(spawn, executable, args, projectParent, environment);
      if (operation === 'run' && !exactInstalledOutput(result, `${targetName}: i32 42`)) {
        reject(`${targetName} result differs`);
      }
    }
    let native = 'unsupported';
    if (targetTriple === 'x86_64-unknown-linux-gnu') {
      for (const operation of ['build', 'run']) {
        const args = [operation, 'src/main.zry', '--project-root', 'hello', '--target', 'native',
          '--name', `native-${operation}`];
        if (operation === 'run') args.push('--export', 'main');
        const result = success(spawn, executable, args, projectParent, environment);
        if (operation === 'run' && !exactInstalledOutput(result, 'native: i32 42')) {
          reject('native result differs');
        }
      }
      native = 'built-and-ran';
    } else {
      const name = 'native-rejection';
      const result = command(spawn, executable, [
        'run', 'src/main.zry', '--project-root', 'hello', '--target', 'native', '--name', name,
        '--export', 'main',
      ], projectParent, environment);
      if (result.status === 0 || !result.stderr.includes(Buffer.from('ZRYNA-N4002'))
        || system.exists(join(projectParent, 'hello', '.zryna', 'out', `${name}.run`))) {
        reject('unsupported Windows native run did not fail closed');
      }
    }

    const tamperSubjects = [provider, target.runtime, 'metadata/distribution.json'];
    for (let index = 0; index < tamperSubjects.length; index += 1) {
      const changed = install(`tampered-${index}`);
      mutate(system, join(changed, ...tamperSubjects[index].split('/')));
      const changedCli = join(changed, ...target.cli.split('/'));
      const name = `tamper-${index}`;
      const result = command(spawn, changedCli, [
        'build', 'src/main.zry', '--project-root', 'hello', '--target', 'javascript', '--name', name,
      ], projectParent, cleanInstalledEnvironment(changedCli, root));
      if (result.status === 0 || !result.stderr.includes(Buffer.from('ZRYNA-C4220'))
        || system.exists(join(projectParent, 'hello', '.zryna', 'out', `${name}.build`))) {
        reject('tampered installation reached artifact execution');
      }
    }

    const projectRoot = join(projectParent, 'hello');
    const retained = [
      'zryna.package.json', 'zryna.lock.json', 'src/main.zry',
      '.zryna/out/js-build.build/zryna-manifest-v1.json',
      '.zryna/out/wasm-build.build/zryna-manifest-v1.json',
    ].map((path) => [path, system.read(join(projectRoot, ...path.split('/')))]);
    system.write(join(projectRoot, 'user-notes.txt'), Buffer.from('user source retained\n'), { flag: 'wx' });
    system.write(join(relocated, 'unrelated.txt'), Buffer.from('unrelated installation file\n'),
      { flag: 'wx' });
    removeVerifiedProductionFiles(relocated, verifiedFiles, system);
    if (system.exists(executable) || !system.exists(join(relocated, 'unrelated.txt'))
      || !system.read(join(relocated, 'unrelated.txt'))
        .equals(Buffer.from('unrelated installation file\n'))
      || !system.read(join(projectRoot, 'user-notes.txt')).equals(Buffer.from('user source retained\n'))
      || retained.some(([path, data]) => !system.read(join(projectRoot, ...path.split('/'))).equals(data))) {
      reject('owned removal changed user or project files');
    }
    return Object.freeze({
      format: 'zryna.installed-acceptance.v1', version, target: targetTriple, archiveSha256,
      checks: Object.freeze({ collision: true, relocated: true, path: true, version: true,
        locale: 'tr_TR.UTF-8', project: true, javascript: true, webassembly: true, native,
        tamperSubjects: 3, uninstall: true, userFilesRetained: true }),
    });
  } finally {
    system.remove(root, { recursive: true, force: true });
  }
}
