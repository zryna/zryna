import { bytes, exactKeys, parseCanonical, requireValue } from './canonical.mjs';
import { validateTuples } from './inventory.mjs';

export function validateMaterials(input, distribution) {
  const record = parseCanonical(input);
  exactKeys(record, ['format', 'files']);
  requireValue(record.format === 'zryna.distribution-materials.v1', 'materials format');
  requireValue(Array.isArray(record.files), 'materials file list');
  validateTuples(record.files, distribution.target.triple);
  const expected = distribution.files.filter(file => file.role !== 'metadata');
  requireValue(bytes(record.files).equals(bytes(expected)), 'materials and distribution disagree');
  return record;
}
