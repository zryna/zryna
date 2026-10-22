// This bootstrap and its Node runtime must already be authenticated by the trusted operator.
import { pathToFileURL } from 'node:url';
import { join } from 'node:path';
import { captureFile } from './launcher-capture.mjs';
import { snapshot } from './launcher-snapshot.mjs';
import { compilerPublicationReceipt, compilerPublicationBinary } from './compiler-publication-capability.mjs';
import { closedJson, exact, fail, prefix, sha256, uiNames, validatePolicy } from './launcher-policy.mjs';

function candidateReceipt(bytes, reviewed) {
  const value = closedJson(bytes, 262144);
  exact(value, ['type', 'status', 'product', 'productVersion', 'source', 'compiler']);
  exact(value.source, ['kind', 'commit', 'tree']); exact(value.compiler, ['bytes', 'sha256']);
  if (value.type !== reviewed.compilerPublication.type || value.status !== 'qualification-candidate' ||
      value.product !== 'zryna-playground-compiler' || value.productVersion !== '0.1.0' ||
      value.source.kind !== reviewed.source.kind || value.source.commit !== reviewed.source.commit || value.source.tree !== reviewed.source.tree ||
      value.compiler.bytes !== reviewed.compiler.bytes || value.compiler.sha256 !== reviewed.compiler.sha256) fail('SOURCE-PUBLICATION');
}
function bindApproval(value, reviewed) {
  if (!value || typeof value !== 'object' || Array.isArray(value) || value.version !== 2 || value.composition_kind !== reviewed.compositionKind ||
      value.composition_sha256 !== reviewed.compositionSha256 || value.compiler_executable_sha256 !== reviewed.compiler.sha256 ||
      value.binding_template_sha256 !== reviewed.template.sha256 || value.wit_closure_sha256 !== reviewed.witSha256 ||
      typeof value.policy_sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(value.policy_sha256)) fail('SOURCE-APPROVAL');
  // Full closed daemon record validation remains the separately pinned Rust host's authority.
  return Object.freeze({ compilerSha256: reviewed.compiler.sha256,
    // Explicit v2 compatibility field; this is the reviewed source composition, not a signed toolkit.
    toolkitSha256: reviewed.compositionSha256, policySha256: value.policy_sha256,
    bindingTemplateSha256: reviewed.template.sha256 });
}

export function createSourceLauncher(independentlyReviewed) {
  return create(independentlyReviewed, null);
}

export function createPublishedSourceLauncher(independentlyReviewed, publicationCapability) {
  // Only the actual pretrusted verifier module's original WeakMap instance can supply these.
  const receipt = compilerPublicationReceipt(publicationCapability);
  const binary = compilerPublicationBinary(publicationCapability);
  return create(independentlyReviewed, { receipt, binary });
}

async function create(independentlyReviewed, publication) {
  const reviewed = validatePolicy(independentlyReviewed, publication ? 'authenticated' : 'qualification-candidate');
  if (process.platform !== 'linux' || process.arch !== 'x64' || process.version !== `v${reviewed.runtime.nodeVersion}` ||
      process.execPath !== reviewed.runtime.node.path) fail('SOURCE-RUNTIME');
  const captured = [];
  let staged;
  let core;
  try {
    captured.push(captureFile(reviewed.runtime.node, 268435456, { copy: false, executable: true }));
    if (!publication) captured.push(captureFile(reviewed.compiler, 268435456, { copy: false, executable: true }));
    const approval = captureFile(reviewed.approval, 262144); captured.push(approval);
    const template = captureFile(reviewed.template, 65536); captured.push(template);
    const provenance = captureFile(reviewed.compilerPublication.receipt, 262144); captured.push(provenance);
    if (publication) {
      const receipt = publication.receipt;
      if (reviewed.source.kind !== 'git-revision' || receipt.type !== reviewed.compilerPublication.type || receipt.status !== 'authenticated' ||
          receipt.product !== 'zryna-playground-compiler' || receipt.productVersion !== '0.1.0' ||
          receipt.source.commit !== reviewed.source.commit || receipt.source.tree !== reviewed.source.tree ||
          receipt.compiler.bytes !== reviewed.compiler.bytes || receipt.compiler.sha256 !== reviewed.compiler.sha256 ||
          publication.binary.length !== reviewed.compiler.bytes || sha256(publication.binary) !== reviewed.compiler.sha256 ||
          !Buffer.from(provenance.bytes).equals(Buffer.from(JSON.stringify(receipt)))) fail('SOURCE-PUBLICATION');
    } else candidateReceipt(provenance.bytes, reviewed);
    const authority = bindApproval(closedJson(approval.bytes, 262144), reviewed);
    const files = new Map();
    for (const file of reviewed.source.files) {
      const selected = captureFile({ path: join(reviewed.source.root.path, file.path), bytes: file.bytes,
        sha256: file.sha256, ownerUid: reviewed.source.root.ownerUid }, 131072, { mode: file.mode });
      captured.push(selected); files.set(file.path, Buffer.from(selected.bytes));
    }
    const resources = new Map(uiNames.map(name => [`/${prefix}${name}`, {
      bytes: Buffer.from(files.get(prefix + name)),
      type: name.endsWith('.html') ? 'text/html; charset=utf-8' : name.endsWith('.css') ?
        'text/css; charset=utf-8' : 'text/javascript; charset=utf-8',
    }]));
    resources.set(`/${prefix}`, resources.get(`/${prefix}index.html`));
    files.set('resources/loader.js', Buffer.from(template.bytes));
    staged = snapshot(files, publication?.binary ?? null);
    for (const value of captured) value.revalidate();
    staged.verify();
    const module = await import(pathToFileURL(staged.modulePath()).href);
    staged.verify();
    core = await module.createCore({ bridgePolicy: { compiler: publication ? staged.compiler() : reviewed.compiler, approval: reviewed.approval },
      authority, witSha256: reviewed.witSha256, templateBytes: Buffer.from(template.bytes), resources });
    for (const value of captured.reverse()) value.close();
    captured.length = 0;
    let stopping;
    return Object.freeze({
      qualificationOnly: !publication,
      source: Object.freeze({ kind: reviewed.source.kind, repository: reviewed.source.repository,
        commit: reviewed.source.commit, tree: reviewed.source.tree }), compositionKind: 'reviewed-source-v1', compositionSha256: reviewed.compositionSha256,
      async launch() { staged.verify(); return core.launch(); },
      takeLaunchUrl() { return core.takeLaunchUrl(); },
      recovery() { return Object.freeze({ host: core.recovery(), snapshotPath: staged.path }); },
      stop() {
        if (stopping) return stopping;
        stopping = (async () => {
          const errors = [];
          try { await core.close(); } catch (error) { errors.push(error); }
          try { staged.cleanup(); } catch (error) { errors.push(error); }
          if (errors.length === 1) throw errors[0];
          if (errors.length) throw new AggregateError(errors, 'PLAYGROUND-SOURCE-CLEANUP');
        })();
        return stopping;
      },
    });
  } catch (primary) {
    const errors = [primary];
    if (core) { try { await core.close(); } catch (error) { errors.push(error); } }
    if (staged) { try { staged.cleanup(); } catch (error) { errors.push(error); } }
    for (const value of captured.reverse()) { try { value.close(); } catch (error) { errors.push(error); } }
    if (errors.length === 1) throw errors[0];
    throw new AggregateError(errors, 'PLAYGROUND-SOURCE-CLEANUP');
  }
}
