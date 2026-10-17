import {
  bytes, canonical, sha256,
} from '../../../../scripts/distribution/canonical.mjs';
import {
  compilerGraph, PRODUCT, requireSpdx, sha1, TARGET, VERSION,
} from './compiler-publication-sbom-graph.mjs';

export const MAX_COMPILER_SPDX_BYTES = 2 * 1024 * 1024;
const DOCUMENT = 'SPDXRef-DOCUMENT';
const ROOT = 'SPDXRef-Package-zryna-playground-compiler';

function id(kind, text) {
  return `SPDXRef-${kind}-${sha256(Buffer.from(text))}`;
}

function pkgRecord(SPDXID, name, extra) {
  return {
    SPDXID, name, downloadLocation: 'NOASSERTION', filesAnalyzed: false,
    licenseConcluded: 'NOASSERTION', licenseDeclared: 'NOASSERTION',
    copyrightText: 'NOASSERTION', ...extra,
  };
}

function relation(spdxElementId, relationshipType, relatedSpdxElement) {
  return { spdxElementId, relationshipType, relatedSpdxElement };
}

function document(context) {
  const { envelope, materials } = context;
  const { graph, licenseOwners } = compilerGraph(context);
  const fileIds = new Map(graph.map(file => [file.path, id('File', file.path)]));
  const materialIds = new Map(materials.packages.map(pkg => [
    `${pkg.name}\0${pkg.version}`, id('Material', `${pkg.name}\0${pkg.version}`),
  ]));
  requireSpdx(new Set([DOCUMENT, ROOT, ...fileIds.values(), ...materialIds.values()]).size
    === 2 + fileIds.size + materialIds.size, 'SPDX identifier collision');
  const packages = [pkgRecord(ROOT, PRODUCT, {
    filesAnalyzed: true, versionInfo: VERSION,
    checksums: [{ algorithm: 'SHA256', checksumValue: envelope.binary.sha256 }],
    packageVerificationCode: { packageVerificationCodeValue: sha1(Buffer.from(
      graph.map(file => file.sha1).sort().join(''), 'ascii',
    )) },
    licenseInfoFromFiles: ['NOASSERTION'],
    sourceInfo: canonical({ source: envelope.source, target: TARGET,
      binary: envelope.binary, licenses: ['LICENSE', 'NOTICE'] }),
  }), ...materials.packages.map(pkg => pkgRecord(
    materialIds.get(`${pkg.name}\0${pkg.version}`), pkg.name, {
      versionInfo: pkg.version,
      checksums: [{ algorithm: 'SHA256', checksumValue: pkg.crateSha256 }],
      sourceInfo: canonical({ cargoLockSha256: materials.cargoLockSha256,
        licenses: pkg.licenses }),
    },
  ))];
  const relationships = [relation(DOCUMENT, 'DESCRIBES', ROOT),
    ...materials.packages.map(pkg => relation(ROOT, 'DEPENDS_ON',
      materialIds.get(`${pkg.name}\0${pkg.version}`))),
    ...graph.flatMap(file => {
      const owner = licenseOwners.get(file.path);
      return [relation(ROOT, 'CONTAINS', fileIds.get(file.path)),
        relation(fileIds.get(file.path), 'GENERATED_FROM', owner ? materialIds.get(owner) : ROOT)];
    }),
    ...materials.packages.flatMap(pkg => pkg.licenses.map(license => relation(
      materialIds.get(`${pkg.name}\0${pkg.version}`), 'CONTAINS', fileIds.get(license.path),
    ))),
  ];
  return {
    SPDXID: DOCUMENT, spdxVersion: 'SPDX-2.3', dataLicense: 'CC0-1.0',
    name: `${PRODUCT}-${VERSION}-${TARGET}`,
    documentNamespace: `https://zryna.com/spdx/playground-compiler/${VERSION}/${TARGET}`
      + `/${envelope.source.commit}/${envelope.binary.sha256}`,
    creationInfo: {
      created: new Date(envelope.source.sourceDateEpoch * 1000).toISOString().replace('.000Z', 'Z'),
      creators: [`Tool: zryna-playground-compiler-sbom-${VERSION}`],
    },
    documentDescribes: [ROOT], packages,
    files: graph.map(file => ({
      SPDXID: fileIds.get(file.path), fileName: `./${file.path}`,
      checksums: [{ algorithm: 'SHA1', checksumValue: file.sha1 },
        { algorithm: 'SHA256', checksumValue: file.sha256 }],
      licenseConcluded: 'NOASSERTION', licenseInfoInFiles: ['NOASSERTION'],
      copyrightText: 'NOASSERTION',
      comment: canonical({ bytes: file.size, mode: file.mode,
        material: licenseOwners.get(file.path) ?? 'workspace-source',
        licenses: licenseOwners.has(file.path) ? [file.path] : ['LICENSE', 'NOTICE'] }),
    })),
    relationships,
  };
}

// This deterministic metadata builder confers no signature or publication authority.
export function createCompilerSpdx(context) {
  const data = bytes(document(context));
  requireSpdx(data.length <= MAX_COMPILER_SPDX_BYTES, 'SPDX document byte budget');
  return data;
}

export function validateCompilerSpdx(data, context) {
  requireSpdx(Buffer.isBuffer(data) && data.length > 0
    && data.length <= MAX_COMPILER_SPDX_BYTES, 'SPDX document byte budget');
  const expected = createCompilerSpdx(context);
  // Exact bytes enforce closed fields, ordering, canonical UTF-8, and all graph edges
  // before parsing untrusted JSON. Every accepted byte was generated from actual files.
  requireSpdx(data.equals(expected), 'SPDX source, files, materials, licenses or relationships differ');
  return JSON.parse(expected.toString('utf8'));
}
