// Closed scalar selection and observation shared by the supervised transport endpoints.
import { exact, fail, isHash, isI32, isRevision } from './limits.mjs';

export function validateEvaluationMode(mode) {
  if (mode !== 'worker' && mode !== 'supervised-host') fail('EVALUATION-MODE');
}

export function validateSelection(input, metadata) {
  exact(input, ['revision', 'sourceSha256', 'componentSha256', 'logical', 'args']);
  if (!isRevision(input.revision) || !isHash(input.sourceSha256) || !isHash(input.componentSha256) ||
      !metadata || input.revision !== metadata.revision || input.sourceSha256 !== metadata.sourceSha256 ||
      input.componentSha256 !== metadata.identity.componentSha256) fail('STALE');
  const entry = metadata.exports.find(entry => entry.logical === input.logical);
  if (!entry || !Array.isArray(input.args) || input.args.length !== entry.arity ||
      Array.from(input.args).some(value => !isI32(value))) fail('ARGUMENTS');
  return Object.freeze({ revision: input.revision, sourceSha256: input.sourceSha256,
    componentSha256: input.componentSha256, logical: input.logical, args: Object.freeze([...input.args]) });
}

export function validateObservation(value, selection) {
  exact(value, ['revision', 'logical', 'componentSha256', 'value']);
  if (value.revision !== selection.revision || value.logical !== selection.logical ||
      value.componentSha256 !== selection.componentSha256 || !isI32(value.value)) fail('OBSERVATION');
  return Object.freeze({ revision: value.revision, logical: value.logical,
    componentSha256: value.componentSha256, value: value.value });
}
