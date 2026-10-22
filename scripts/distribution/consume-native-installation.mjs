// One-shot hosted consumer. No compiler build, guest exception or release authority.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { chmodSync, closeSync, constants, fstatSync, lstatSync, mkdirSync, openSync,
  readdirSync, readSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { basename, dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';
const BASE = '236d2a7126dd0a9722cd07974843b84fbc48f76c';
const TREE = 'ff70cd8cadcefe68b0e5cf43d49c2dd8350b9af0';
const BRANCH = 'refs/heads/codex/414-native-clean-consumer-main3-20261022';
const RUN = 39722689792, ATTEMPT = 1;
const SELF = 'scripts/distribution/consume-native-installation.mjs';
const WORKFLOW = '.github/workflows/native-provider-clean-consumer.yml';
const PINS = {
  "scripts/distribution-release/extract-verified-files.mjs": "467ab082ad41f028736bd03f6dd80e63f4d98af4af03f8df953bc35629805763",
  "scripts/distribution/archive-entries.mjs": "bea7c3ca8dc4bad4770d28d3450429e24e9fc361b5ee40c8cfc673491a865384",
  "scripts/distribution/archive-tar.mjs": "9a2d906a3b085055ad02d4bf0752ba7544dd32776203fb22db193e4b40c5aaee",
  "scripts/distribution/archive-zip.mjs": "721904a07539ddd2896c6a53cdbf931be5431953df7c9a963f9f556c61546ff1",
  "scripts/distribution/binary-identity.mjs": "a4ebfbc73225ce54fe021e196284963dedfea0cecbde706ec9a759bd4c7cda1c",
  "scripts/distribution/canonical.mjs": "7bee4ec69538d074fb134dd9cc2134c8964d336f3a08cef877029a9c3b02f63d",
  "scripts/distribution/inventory.mjs": "14331d89d86f610edd3122c941837a4ffb2d968e487f357317f06e130e3f7475",
  "scripts/distribution/material-record.mjs": "e9de7ed83bbcb3bbb45f2d6c786e95a60f2e6debe75fe528fb0d016f232320b4",
  "scripts/distribution/materials-rust-command-candidate-v1.json": "499be6581463e71e54ede6d2c5807c346173f34e7505e6faeaae95944c254483",
  "scripts/distribution/materials-rust-v1.json": "32b6343a6dfae19d7c3cdcc50658762c2c0ad08aca15124bdce85b28db38a84b",
  "scripts/distribution/native-distribution-v2.mjs": "db00adde0a4bcb7746542da68cd0dd8af23d44ca87d6b0107347e43a54f5ba9e",
  "scripts/distribution/native-inventory-v2.mjs": "1c6c9fef588d94539f6afa889bebecd97fbc89262fbb879bd82df0f5aeb5f867",
  "scripts/distribution/native-recipe-v2.json": "1dd148410ee06fbb00ef3c07fa28a55c14977c70b6f34797d935cc12ffc82558",
  "scripts/distribution/runtime.mjs": "7bec637b6e6c7c5cccb7fe6ce3a80c45274d9266e9dfe3d5f878857ac484f798",
  "scripts/distribution/source-receipt.mjs": "a9ca623f24414fb342e0253f7933a547f137eb555834c0976a7f3474d0be0316",
  "scripts/m4/hosted-api.mjs": "1f9cb7ae182243e9d4798afea8e2f9eec017e0406243adc66527894a74cba428",
  "scripts/m4/hosted-archive.mjs": "7d07616422c3297bf2fd5360465d6ca8a9eeaf86fae5eb5d3df718676f355ab1",
  "scripts/m4/registry-files.mjs": "0da0990b5992d206405afb751200615abd5ab886c5ed46c5bff7dd25714bd83d",
  "scripts/m4/retain-publication.mjs": "b7590edfdf96cbcc9eb8ed52fc9a3ba3bbb5956543f27fabb7b4a12a70f7400b",
  "tests/native-provider-public-install/command-h1.mjs": "3a05a5fa0926f0e424e20acbd3a5166f41f70530f5ef253a4cc2eb787472c087",
  "tests/native-provider-public-install/commands.mjs": "55b7287a5c8cd51354d5fe03d521c941d73e1aea04a073b8489a9cd8090431ca",
  "tests/native-provider-public-install/fixtures.mjs": "ffbf564145933be55ecf5b61f12e3647b1c47d4750de815b7dafa8ad49ae6ebe",
  "tests/native-provider-public-install/run.mjs": "f9ecd80c66f764446561b9ef46398d6267d0e83deac55bd668871d300b64c690"
};
const TARGETS = {
  "linux": {
    "os": "ubuntu-24.04",
    "triple": "x86_64-unknown-linux-gnu",
    "job": 119639548087,
    "artifact": {
      "id": 12402106671,
      "bytes": 38518868,
      "sha256": "fd0af350b4c8af17beba97511450959d5dff706a41ca352deea34d5c1e8af559"
    },
    "memberCount": 309,
    "inventorySha256": "856b317eef43ef171fb7e9755fb1b26f720612cf6910d941e249749934bcc7de",
    "filename": "zryna-native-0.2.3-x86_64-unknown-linux-gnu.tar.gz",
    "size": 19228820,
    "sha256": "6bbccb6aa7d6f94324eb021cc54eda1e19cbca0f61436ad0e2f0348ba6fe7136",
    "subjectSize": 744,
    "subjectSha256": "b4264a6cba5fe96e37adaba49243cb1f44d20756b2c711a7764302252bbc5537"
  },
  "win32": {
    "os": "windows-2022",
    "triple": "x86_64-pc-windows-msvc",
    "job": 119639547873,
    "artifact": {
      "id": 12402256142,
      "bytes": 35160025,
      "sha256": "b96920796551040b462f1bc4e23d2de46c8f602ee988445859cf76c07227039d"
    },
    "memberCount": 319,
    "inventorySha256": "e14eb660514b2cce52c9beeaa4501ebedf98fc7916e03f4755adab22ba767053",
    "filename": "zryna-native-0.2.3-x86_64-pc-windows-msvc.zip",
    "size": 62856671,
    "sha256": "fb08e1b70bbdbd0ca55d76549b1937087d3468cedeb7d670fb254498d024f073",
    "subjectSize": 770,
    "subjectSha256": "e57d2d41962fd9a1765a89d6706ddc373ab654d12d128251d3cbf7e65e4c6369"
  }
};
const selected = TARGETS[process.platform]; assert(selected && process.arch === 'x64');
const hash = body => createHash('sha256').update(body).digest('hex');
const encoded = value => Buffer.from(JSON.stringify(value) + '\n');
const metadata = st => Object.fromEntries(['dev', 'ino', 'mode', 'nlink', 'size', 'mtimeNs', 'ctimeNs']
  .map(key => [key, String(st[key])]));
const stat = path => lstatSync(path, { bigint: true, throwIfNoEntry: false });
function ordinary(path, maximum = 256 * 1024 * 1024, allowEmpty = false) {
  assert(isAbsolute(path) && realpathSync.native(path) === path, 'canonical ordinary path');
  const before = stat(path); assert(before?.isFile() && !before.isSymbolicLink()
    && before.nlink === 1n && before.size >= (allowEmpty ? 0 : 1) && before.size <= maximum, 'bounded single-link file');
  const fd = openSync(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  try {
    const opened = fstatSync(fd, { bigint: true }), body = Buffer.alloc(Number(before.size) + 1);
    let count = 0; while (count < body.length) {
      const size = readSync(fd, body, count, body.length - count, null); if (!size) break; count += size;
    }
    for (const observed of [opened, fstatSync(fd, { bigint: true }), stat(path)]) {
      assert.deepEqual(metadata(observed), metadata(before), 'stable ordinary identity');
    }
    assert.equal(count, Number(before.size)); return body.subarray(0, count);
  } finally { closeSync(fd); }
}
function save(path, value) {
  const body = Buffer.isBuffer(value) ? value : encoded(value);
  writeFileSync(path, body, { flag: 'wx', mode: 0o600 }); return { path, bytes: body.length, sha256: hash(body) };
}
function inventory(root) {
  const files = []; let total = 0;
  function visit(path, logical = '', depth = 0) {
    assert(depth <= 16); const dir = stat(path); assert(dir?.isDirectory() && !dir.isSymbolicLink());
    for (const name of readdirSync(path).sort()) {
      assert(name !== '.' && name !== '..' && !name.includes('\\') && !name.includes('/'));
      const child = join(path, name), key = logical ? logical + '/' + name : name, st = stat(child);
      assert(st && !st.isSymbolicLink());
      if (st.isDirectory()) visit(child, key, depth + 1);
      else { const body = ordinary(child, 256 * 1024 * 1024, true); total += body.length;
        assert(files.length < 1024 && total <= 256 * 1024 * 1024);
        files.push({ path: key, bytes: body.length, sha256: hash(body) }); }
    }
  }
  visit(root); return files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
}
const { values } = parseArgs({ options: { mode: { type: 'string' } }, strict: true });
assert(['stage', 'consume'].includes(values.mode));
assert.equal(process.env.GITHUB_ACTIONS, 'true'); assert.equal(process.env.GITHUB_REPOSITORY, 'zryna/zryna');
assert.equal(process.env.GITHUB_REF, BRANCH); assert.equal(process.env.GITHUB_EVENT_NAME, 'push');
const temp = realpathSync.native(process.env.RUNNER_TEMP), workspace = realpathSync.native(process.env.GITHUB_WORKSPACE);
const root = join(temp, 'm414-native-clean-consumer'), helpers = join(root, 'helpers');
const source = join(workspace, 'compiler-checkout'), extracted = join(root, 'action-extraction');
assert(relative(temp, root) === 'm414-native-clean-consumer' && !root.startsWith(workspace + '/'));
const git = args => execFileSync('git', args, { cwd: source, encoding: null, maxBuffer: 4 * 1024 * 1024 });
const text = args => git(args).toString('utf8').trim();
const load = file => import(pathToFileURL(join(helpers, file)).href);
let phase = values.mode;
try {
  if (values.mode === 'stage') {
    assert(!stat(root), 'fresh task-owned staging root');
    assert.equal(realpathSync.native(source), source); assert.equal(text(['rev-parse', '--show-toplevel']), source.replaceAll('\\', '/'));
    assert.equal(text(['rev-parse', 'HEAD']), process.env.GITHUB_SHA); assert.equal(text(['show', '-s', '--format=%P', 'HEAD']), BASE);
    assert.equal(text(['rev-parse', BASE + '^{tree}']), TREE); assert.equal(text(['status', '--porcelain=v1', '--untracked-files=all']), '');
    const additions = text(['diff', '--name-status', BASE, 'HEAD']).split('\n').sort();
    assert.deepEqual(additions, ['A\t' + WORKFLOW, 'A\t' + SELF].sort(), 'only two operator additions');
    mkdirSync(root, { mode: 0o700 }); mkdirSync(helpers, { mode: 0o700 });
    const files = [];
    for (const [file, digest] of Object.entries(PINS)) {
      const body = ordinary(join(source, file), 2 * 1024 * 1024), blob = git(['show', BASE + ':' + file]);
      assert.equal(hash(body), digest); assert.equal(hash(blob), digest, 'exact final-main helper blob pin');
      const destination = join(helpers, file); mkdirSync(dirname(destination), { recursive: true, mode: 0o700 });
      files.push(save(destination, body));
    }
    files.push(save(join(helpers, SELF), ordinary(join(source, SELF), 2 * 1024 * 1024)));
    const operator = { commit: process.env.GITHUB_SHA, tree: text(['rev-parse', 'HEAD^{tree}']),
      workflow: WORKFLOW, branch: BRANCH, parent: BASE, sourceRoot: source, sourceIdentity: metadata(stat(source)),
      runId: Number(process.env.GITHUB_RUN_ID), attempt: Number(process.env.GITHUB_RUN_ATTEMPT) };
    assert(operator.runId !== RUN && operator.runId > 0 && operator.attempt > 0);
    save(join(root, 'staging.json'), { operator, files, helperPins: PINS,
      actualFinalMainGitBlobsVerified: true, closedBuiltinHelperCount: 20, pinnedJsonCount: 3 });
    const { githubApi } = await load('scripts/m4/hosted-api.mjs');
    const { downloadPublicationArchive } = await load('scripts/m4/retain-publication.mjs');
    phase = 'producer-authentication-and-full-outer-download';
    const authenticated = await authenticate(githubApi(), operator);
    save(join(root, 'producer-authentication.json'), authenticated);
    save(join(root, 'producer-artifact.zip'), await downloadPublicationArchive('zryna/zryna', selected.artifact));
    console.log(JSON.stringify({ status: 'staged-authenticated-no-cli-executed', root, operator }));
  } else {
    assert.equal(fileURLToPath(import.meta.url), join(helpers, SELF), 'consume only staged coordinator');
    const state = JSON.parse(ordinary(join(root, 'staging.json'), 1024 * 1024));
    assert.equal(state.operator.commit, process.env.GITHUB_SHA); assert.equal(state.operator.sourceRoot, source);
    assert.equal(state.operator.runId, Number(process.env.GITHUB_RUN_ID));
    assert.deepEqual(state.helperPins, PINS);
    for (const [file, digest] of Object.entries(PINS)) {
      assert.equal(hash(ordinary(join(helpers, file), 2 * 1024 * 1024)), digest, 'hardcoded final-main staged helper pin');
    }
    assert.deepEqual(state.files.map(file => file.path).sort(), [...Object.keys(PINS), SELF].map(file => join(helpers, file)).sort());
    for (const file of state.files) { const body = ordinary(file.path, 2 * 1024 * 1024);
      assert.equal(hash(body), file.sha256); assert.equal(body.length, file.bytes); }
    assert.deepEqual(inventory(helpers).map(file => file.path), [...Object.keys(PINS), SELF].sort());
    const outer = ordinary(join(root, 'producer-artifact.zip'));
    assert.equal(outer.length, selected.artifact.bytes); assert.equal(hash(outer), selected.artifact.sha256);
    phase = 'complete-action-extraction-byte-binding';
    const members = inventory(extracted); assert.equal(members.length, selected.memberCount);
    assert.equal(hash(encoded(members)), selected.inventorySha256, 'entire extraction equals authenticated reviewed outer member inventory');
    save(join(root, 'action-extraction-inventory.json'), { outer: selected.artifact, members,
      inventorySha256: selected.inventorySha256, independentlyReviewedOuterMemberBinding: true });
    const archive = ordinary(join(extracted, 'qualification', 'replica-a-' + selected.filename));
    const subjectBytes = ordinary(join(extracted, 'qualification', 'replica-a-subject.json'), 262144);
    assert.equal(archive.length, selected.size); assert.equal(hash(archive), selected.sha256);
    assert.equal(subjectBytes.length, selected.subjectSize); assert.equal(hash(subjectBytes), selected.subjectSha256);
    save(join(root, selected.filename), archive); save(join(root, 'selected-subject.json'), subjectBytes);
    const subject = JSON.parse(subjectBytes);
    assert.equal(subject.source.commit, BASE); assert.equal(subject.source.tree, TREE); assert.equal(subject.target.triple, selected.triple);
    const { githubApi } = await load('scripts/m4/hosted-api.mjs');
    save(join(root, 'consumer-authentication.json'), await authenticate(githubApi(), state.operator));
    phase = 'remove-only-verified-task-owned-input-trees';
    assert.equal(realpathSync.native(source), source); assert.equal(text(['rev-parse', 'HEAD']), state.operator.commit);
    assert.equal(text(['status', '--porcelain=v1', '--untracked-files=all']), '');
    assert.deepEqual(metadata(stat(source)), state.operator.sourceIdentity);
    assert.equal(basename(source), 'compiler-checkout'); assert.equal(dirname(source), workspace);
    rmSync(extracted, { recursive: true }); rmSync(source, { recursive: true });
    assert(!stat(source) && !stat(extracted));
    save(join(root, 'checkout-removal.json'), { source, extracted, actualOperator: state.operator,
      checkoutAbsentBeforeCli: true, producerExtractionAbsentBeforeCli: true, retainedClosure: inventory(helpers),
      compilerSourcesBootstrapAndBuildRootsStaged: false, globalOsToolAbsenceClaimed: false });
    delete process.env.GITHUB_TOKEN; delete process.env.GH_TOKEN;
    const { verifyNativeArchive } = await load('scripts/distribution/native-distribution-v2.mjs');
    const { extractVerifiedProductionFiles } = await load('scripts/distribution-release/extract-verified-files.mjs');
    const { runNativeInstalledSmoke } = await load('tests/native-provider-public-install/run.mjs');
    phase = 'original-native-archive-verification-extraction-and-smoke';
    const verified = await verifyNativeArchive(archive, subject), installation = join(root, 'fresh-install');
    extractVerifiedProductionFiles(installation, verified.files);
    if (process.platform !== 'win32') for (const file of verified.files) {
      const installed = join(installation, file.path); chmodSync(installed, file.mode);
      assert.equal(Number(stat(installed).mode & 0o7777n), file.mode);
    }
    const smoke = await runNativeInstalledSmoke({ source: subject.source, output: root, installation,
      target: selected.triple, verifiedFiles: verified.files });
    assert.equal(smoke.independentCleanMachineClaimed, false); assert(!stat(source) && !stat(extracted));
    save(join(root, 'RESULT.json'), { format: 'zryna.native-independent-consumer.v1',
      status: 'actual-separate-hosted-consumer-cli-smoke-passed', productSource: subject.source,
      operator: state.operator, producerRun: RUN, producerAttempt: ATTEMPT, outer: selected.artifact,
      nativeArchive: { bytes: archive.length, sha256: hash(archive) }, originalSmoke: smoke,
      separateHostedConsumerJob: true, checkoutAbsentBeforeAndAfterCli: true,
      coordinatorNodeRemainsOnVm: true, nodeBootstrapAbsentFromInstallation: true,
      globalNodeOrRustAbsenceClaimed: false, cargoOrCompilerBuildPerformed: false,
      existingSmokeClaimRewritten: false, productionAdmission: 'forbidden' });
    console.log(JSON.stringify({ status: 'actual-separate-hosted-consumer-cli-smoke-passed', root }));
  }
} catch (error) {
  if (stat(root)) save(join(root, values.mode + '-FAILURE.json'), { status: 'failed-no-independent-acceptance-claim',
    phase, message: String(error.message).slice(0, 3000), cleanupConfirmed: false });
  throw error;
}
async function authenticate(api, operator) {
  const prefix = '/repos/zryna/zryna';
  const [run, jobs, commit, artifact, consumer, consumerJobs] = await Promise.all([
    api(`${prefix}/actions/runs/${RUN}/attempts/${ATTEMPT}`), api(`${prefix}/actions/runs/${RUN}/attempts/${ATTEMPT}/jobs?per_page=100`),
    api(`${prefix}/git/commits/${BASE}`), api(`${prefix}/actions/artifacts/${selected.artifact.id}`),
    api(`${prefix}/actions/runs/${operator.runId}/attempts/${operator.attempt}`),
    api(`${prefix}/actions/runs/${operator.runId}/attempts/${operator.attempt}/jobs?per_page=100`),
  ]);
  assert.equal(run.id, RUN); assert.equal(run.run_attempt, ATTEMPT); assert.equal(run.head_sha, BASE);
  assert.equal(run.path, '.github/workflows/native-provider-install.yml'); assert.equal(run.status, 'completed'); assert.equal(run.conclusion, 'success');
  assert.equal(run.repository.full_name, 'zryna/zryna'); assert.equal(commit.sha, BASE); assert.equal(commit.tree.sha, TREE);
  assert.equal(jobs.total_count, 2); assert.equal(jobs.jobs.length, 2);
  for (const target of Object.values(TARGETS)) {
    const job = jobs.jobs.find(item => item.id === target.job); assert(job);
    assert.equal(job.name, `native installation (${target.os})`); assert.equal(job.run_id, RUN); assert.equal(job.run_attempt, ATTEMPT);
    assert.equal(job.head_sha, BASE); assert.equal(job.status, 'completed'); assert.equal(job.conclusion, 'success');
    assert.equal(job.runner_group_name, 'GitHub Actions'); assert(job.labels.includes(target.os));
  }
  assert.equal(artifact.id, selected.artifact.id); assert.equal(artifact.size_in_bytes, selected.artifact.bytes);
  assert.equal(artifact.digest, 'sha256:' + selected.artifact.sha256); assert.equal(artifact.expired, false);
  assert(Date.parse(artifact.expires_at) > Date.now()); assert.equal(artifact.workflow_run.id, RUN); assert.equal(artifact.workflow_run.head_sha, BASE);
  assert.equal(artifact.name, `native-provider-install-${selected.os}-${BASE}`);
  assert.equal(consumer.status, 'in_progress');
  assert.equal(consumer.id, operator.runId); assert.equal(consumer.run_attempt, operator.attempt);
  assert.equal(consumer.head_sha, operator.commit); assert.equal(consumer.path, WORKFLOW); assert.equal(consumer.event, 'push');
  assert.equal(consumer.repository.full_name, 'zryna/zryna'); assert.equal(consumer.head_branch, BRANCH.slice('refs/heads/'.length));
  const own = consumerJobs.jobs.filter(job => job.name === `clean consumer (${selected.os})`); assert.equal(own.length, 1);
  assert.equal(own[0].status, 'in_progress'); assert(!own[0].labels.includes('self-hosted'));
  assert.equal(own[0].head_sha, operator.commit); assert.equal(own[0].runner_group_name, 'GitHub Actions');
  assert.equal(own[0].runner_name, process.env.RUNNER_NAME); assert(own[0].labels.includes(selected.os));
  assert(jobs.jobs.every(job => job.id !== own[0].id && job.runner_name !== own[0].runner_name));
  return { producer: run, producerJobs: jobs, source: commit, artifact, consumer, consumerJob: own[0] };
}
