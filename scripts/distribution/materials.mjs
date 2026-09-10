import { requireValue } from './canonical.mjs';
import { targetPaths } from './inventory.mjs';

const PACKAGE_FILES = Object.freeze({
  'lib/zryna/bootstrap/node_modules/@typescript/typescript6/package.json':
    [841, 'd9b8fb53c67c947fece83bcb73fa8512227ea6a12b110e096fdab8ecdc02b655'],
  'lib/zryna/bootstrap/node_modules/@typescript/typescript6/lib/typescript.js':
    [45, 'd3f3cd2b04b7f466f4484df921b744223f7bd1f3e353ec9110bdf52695b983d5'],
  'lib/zryna/bootstrap/node_modules/@typescript/old/package.json':
    [3527, '9332e97c30d3e53ed54910b89207ed657fb444066484df6e5b6965bf130865e9'],
  'lib/zryna/bootstrap/node_modules/@typescript/old/lib/typescript.js':
    [9144216, '569177652966bd528c319171c7dd22860dbf72bde116cbc4f644f1d02bb12e39'],
  'licenses/typescript6-LICENSE.txt':
    [9197, 'a7d00bfd54525bc694b6e32f64c7ebcf5e6b7ae3657be5cc12767bce74654a47'],
  'licenses/typescript-LICENSE.txt':
    [9197, 'a7d00bfd54525bc694b6e32f64c7ebcf5e6b7ae3657be5cc12767bce74654a47'],
  'licenses/typescript-ThirdPartyNoticeText.txt':
    [37824, '1af3c68039c57e539422da82a4faada506ce6d0ea6f90e0b699d02dbcdb7a90c'],
});

export const NODE_UPSTREAM = Object.freeze({
  version: '22.22.1',
  keyRepository: 'https://github.com/nodejs/release-keys',
  keyCommit: '481637f813e912c4aa3622d7964ab426c97b8e8d',
  fingerprint: 'CC68F5A3106FF448322E48ED27F5E38D5B0A215F',
  keySha256: 'e31e1aa40a8331f01d753cef475f7b9eab934fc25f5f0b36995bfd80bd66ad27',
  checksumsSha256: 'b4e0a68b3fd205e27a000f5722d7932cf31c1e0b6b3d9101bcb153abfd5d8023',
  signatureSha256: '3bbdd7f36e2aa9823de9c999b81956628f07a36b4d2a0567c9f3e016d5e391f4',
});

export const NODE_TARGETS = Object.freeze({
  'x86_64-unknown-linux-gnu': {
    archive: 'node-v22.22.1-linux-x64.tar.xz',
    archiveSize: 31060640,
    archiveSha256: '9a6bc82f9b491279147219f6a18add1e18424dce90d41d2a5fcd69d4924ba3aa',
    executable: [124674920, '243fd8938011479f41b3de101842150fa990f33fbbb3f7aabd330857f2d79e1d'],
    license: [145485, 'c738ae413cf561f174e34f6961f8ca458aae2369a73640dda6234c629b98bcc4'],
  },
  'x86_64-pc-windows-msvc': {
    archive: 'node-v22.22.1-win-x64.zip',
    archiveSize: 35945338,
    archiveSha256: '877cb93829e14fffbbc7903e7d8037336c9a79f3ea43c5d0b8c2379b79da56de',
    executable: [87059456, '923a41f268ab49ede2e3363fbdd9e790609e385c6f3ca880b4ee9a56a8133e5a'],
    license: [148217, '8cc9bb466b19fc7e7cc99d03e9df1132021fda8b01eea2624c58bb372dbef576'],
  },
});

export function validateBootstrapPins(entries, target) {
  const paths = targetPaths(target);
  const node = NODE_TARGETS[target];
  const pins = { ...PACKAGE_FILES, [paths.node]: node.executable, 'licenses/node-LICENSE': node.license };
  const actual = new Map(entries.map(entry => [entry.path, entry]));
  for (const [path, [size, digest]] of Object.entries(pins)) {
    const entry = actual.get(path);
    requireValue(entry?.size === size && entry?.sha256 === digest, 'bootstrap material pin mismatch');
  }
  // Worker modules and Rust notices are additionally bound to the authenticated source recipe;
  // these upstream pins alone do not establish either source or release provenance.
}
