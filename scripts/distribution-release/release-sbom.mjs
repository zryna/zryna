import { createHash } from 'node:crypto';
import { TextDecoder } from 'node:util';
import { canonical, canonicalBounded, parseCanonical, sha256 } from './canonical.mjs';

const VERSION = '0.2.0';
const DOCUMENT = 'SPDXRef-DOCUMENT';
const ROOT_PACKAGE = 'SPDXRef-Package-zryna';
const MAX_FILES = 512;
const PATH = /^[A-Za-z0-9][A-Za-z0-9@+._-]*(?:\/[A-Za-z0-9@][A-Za-z0-9@+._-]*){0,11}$/;

function reject(message) {
  throw new Error(`R406-SPDX: ${message}`);
}

function exactKeys(value, expected, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)
    || canonical(Object.keys(value).sort()) !== canonical([...expected].sort())) {
    reject(`${label} fields differ from the closed SPDX 2.3 profile`);
  }
}

function id(kind, value) {
  return `SPDXRef-${kind}-${sha256(Buffer.from(value, 'utf8')).slice(0, 24)}`;
}

function sha1(value) {
  return createHash('sha1').update(value).digest('hex');
}

function decodedInventory(files) {
  const matches = files.filter(({ path }) => path === 'metadata/inventory.json');
  if (matches.length !== 1 || !Buffer.isBuffer(matches[0].data)) {
    reject('authenticated final inventory is missing or ambiguous');
  }
  let value;
  try {
    value = parseCanonical(new TextDecoder('utf-8', { fatal: true }).decode(matches[0].data));
  } catch (error) {
    if (error instanceof TypeError) reject('authenticated final inventory is not UTF-8');
    throw error;
  }
  if (value?.format !== 'zryna.distribution-inventory.v1' || !Array.isArray(value.files)) {
    reject('authenticated final inventory format differs');
  }
  return value.files;
}

function finalGraph(files) {
  if (!Array.isArray(files) || files.length < 1 || files.length > MAX_FILES) {
    reject('authenticated archive file count differs');
  }
  const indexed = decodedInventory(files);
  if (indexed.length !== files.length - 2
    || indexed.some((entry, index) => index > 0 && indexed[index - 1]?.path >= entry?.path)
    || indexed.some(({ path }) => [
      'metadata/inventory.json', 'metadata/checksums.sha256',
    ].includes(path))) {
    reject('authenticated final inventory coverage or order differs');
  }
  const tuples = new Map(indexed.map((entry) => [entry.path, entry]));
  if (tuples.size !== indexed.length) reject('authenticated final inventory paths repeat');
  for (const path of ['metadata/inventory.json', 'metadata/checksums.sha256']) {
    tuples.set(path, { path, material: 'source', licenses: ['LICENSE'] });
  }
  const result = files.map((file) => {
    const tuple = tuples.get(file?.path);
    if (!PATH.test(file?.path ?? '') || !Buffer.isBuffer(file.data)
      || ![0o644, 0o755].includes(file.mode) || !tuple
      || (tuple.size !== undefined && (tuple.size !== file.data.length
        || tuple.sha256 !== sha256(file.data) || tuple.mode !== file.mode))
      || typeof tuple.material !== 'string' || !Array.isArray(tuple.licenses)
      || tuple.licenses.length < 1) {
      reject('authenticated archive bytes and material graph differ');
    }
    return {
      path: file.path, size: file.data.length, sha1: sha1(file.data), sha256: sha256(file.data),
      material: tuple.material, licenses: [...tuple.licenses],
    };
  }).sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  if (new Set(result.map(({ path }) => path.toLowerCase())).size !== result.length
    || result.some(({ licenses }) => licenses.some((path) => !tuples.has(path)))) {
    reject('archive paths or license references are incomplete');
  }
  return result;
}

function packageRecord(SPDXID, name, extra = {}) {
  return {
    SPDXID, name, downloadLocation: 'NOASSERTION', filesAnalyzed: false,
    licenseConcluded: 'NOASSERTION', licenseDeclared: 'NOASSERTION',
    copyrightText: 'NOASSERTION', ...extra,
  };
}

export function createReleaseSpdx({ files, archive, source, target }) {
  const graph = finalGraph(files);
  if (archive?.filename !== `zryna-${VERSION}-${target}.tar.gz`
    && archive?.filename !== `zryna-${VERSION}-${target}.zip`) {
    reject('archive subject name differs');
  }
  if (!/^[0-9a-f]{64}$/.test(archive?.sha256 ?? '')
    || !/^[0-9a-f]{40}$/.test(source?.commit ?? '')
    || source.repository !== 'https://github.com/zryna/zryna'
    || source.ref !== 'refs/tags/v0.2.0') reject('release source identity differs');
  const materials = [...new Set(graph.map(({ material }) => material))].sort();
  const materialIds = new Map(materials.map((material) => [material, id('Material', material)]));
  const fileIds = new Map(graph.map(({ path }) => [path, id('File', path)]));
  if (new Set([...materialIds.values(), ...fileIds.values(), DOCUMENT, ROOT_PACKAGE]).size
    !== materialIds.size + fileIds.size + 2) reject('SPDX identifiers collide');
  const filesSpdx = graph.map((file) => ({
    SPDXID: fileIds.get(file.path),
    fileName: `./${file.path}`,
    checksums: [
      { algorithm: 'SHA1', checksumValue: file.sha1 },
      { algorithm: 'SHA256', checksumValue: file.sha256 },
    ],
    licenseConcluded: 'NOASSERTION',
    licenseInfoInFiles: ['NOASSERTION'],
    copyrightText: 'NOASSERTION',
    comment: canonical({ material: file.material, licenses: file.licenses }),
  }));
  const packages = [packageRecord(ROOT_PACKAGE, 'zryna', {
    filesAnalyzed: true,
    versionInfo: VERSION,
    packageFileName: archive.filename,
    checksums: [{ algorithm: 'SHA256', checksumValue: archive.sha256 }],
    packageVerificationCode: {
      packageVerificationCodeValue: sha1(Buffer.from(
        graph.map(({ sha1: digest }) => digest).sort().join(''), 'ascii',
      )),
    },
    licenseInfoFromFiles: ['NOASSERTION'],
    sourceInfo: `${source.repository}@${source.commit}`,
  }), ...materials.map((material) => packageRecord(materialIds.get(material), material, {
    sourceInfo: canonical({
      material,
      licenses: [...new Set(graph.filter((file) => file.material === material)
        .flatMap(({ licenses }) => licenses))].sort(),
    }),
  }))];
  const relationships = [
    { spdxElementId: DOCUMENT, relationshipType: 'DESCRIBES', relatedSpdxElement: ROOT_PACKAGE },
    ...materials.map((material) => ({
      spdxElementId: ROOT_PACKAGE, relationshipType: 'DEPENDS_ON',
      relatedSpdxElement: materialIds.get(material),
    })),
    ...graph.flatMap((file) => [
      { spdxElementId: ROOT_PACKAGE, relationshipType: 'CONTAINS',
        relatedSpdxElement: fileIds.get(file.path) },
      { spdxElementId: fileIds.get(file.path), relationshipType: 'GENERATED_FROM',
        relatedSpdxElement: materialIds.get(file.material) },
    ]),
  ];
  return {
    SPDXID: DOCUMENT,
    spdxVersion: 'SPDX-2.3',
    dataLicense: 'CC0-1.0',
    name: `zryna-${VERSION}-${target}`,
    documentNamespace: `https://zryna.com/spdx/${VERSION}/${target}/${archive.sha256}`,
    creationInfo: {
      created: new Date(source.sourceDateEpoch * 1000).toISOString().replace('.000Z', 'Z'),
      creators: [`Tool: zryna-release-sbom-${VERSION}`],
    },
    documentDescribes: [ROOT_PACKAGE],
    packages,
    files: filesSpdx,
    relationships,
  };
}

function validateOfficialShape(sbom) {
  exactKeys(sbom, [
    'SPDXID', 'spdxVersion', 'dataLicense', 'name', 'documentNamespace', 'creationInfo',
    'documentDescribes', 'packages', 'files', 'relationships',
  ], 'document');
  exactKeys(sbom.creationInfo, ['created', 'creators'], 'creationInfo');
  for (const entry of sbom.packages ?? []) {
    const fields = entry.SPDXID === ROOT_PACKAGE
      ? ['SPDXID', 'name', 'downloadLocation', 'filesAnalyzed', 'licenseConcluded',
        'licenseDeclared', 'copyrightText', 'versionInfo', 'packageFileName', 'checksums',
        'packageVerificationCode', 'licenseInfoFromFiles', 'sourceInfo']
      : ['SPDXID', 'name', 'downloadLocation', 'filesAnalyzed', 'licenseConcluded',
        'licenseDeclared', 'copyrightText', 'sourceInfo'];
    exactKeys(entry, fields, 'package');
  }
  for (const entry of sbom.files ?? []) exactKeys(entry, [
    'SPDXID', 'fileName', 'checksums', 'licenseConcluded', 'licenseInfoInFiles',
    'copyrightText', 'comment',
  ], 'file');
  for (const entry of sbom.relationships ?? []) exactKeys(entry, [
    'spdxElementId', 'relationshipType', 'relatedSpdxElement',
  ], 'relationship');
  const root = sbom.packages?.find(({ SPDXID }) => SPDXID === ROOT_PACKAGE);
  if (root?.filesAnalyzed !== true
    || !/^[0-9a-f]{40}$/.test(root.packageVerificationCode?.packageVerificationCodeValue ?? '')
    || !Array.isArray(root.licenseInfoFromFiles) || root.licenseInfoFromFiles.length < 1
    || sbom.packages.some((entry) => entry !== root && entry.filesAnalyzed !== false)) {
    reject('root package must analyze its contained files with a verification code');
  }
}

export function validateReleaseSpdx(sbomBytes, context) {
  let sbom;
  try {
    sbom = parseCanonical(new TextDecoder('utf-8', { fatal: true }).decode(sbomBytes));
  } catch (error) {
    if (error instanceof TypeError) reject('SBOM is not UTF-8');
    throw error;
  }
  validateOfficialShape(sbom);
  const expected = createReleaseSpdx(context);
  if (canonical(sbom) !== canonical(expected)) {
    reject('SPDX subject, archive files, materials, licenses, or relationships differ');
  }
  return sbom;
}

export function releaseSpdxBytes(context) {
  return Buffer.from(`${canonicalBounded(createReleaseSpdx(context))}\n`);
}
