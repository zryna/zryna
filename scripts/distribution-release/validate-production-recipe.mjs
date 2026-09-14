const PRODUCTION_RECIPE_IDENTITY = Object.freeze({
  format: 'zryna.distribution-recipe.v1',
  status: 'production-accepted',
  productionAdmission: 'allowed',
  versionCandidate: '0.2.3',
});

export function validateProductionRecipeIdentity(value, reject) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)
    || Object.entries(PRODUCTION_RECIPE_IDENTITY)
      .some(([field, expected]) => value[field] !== expected)) {
    reject('recipe does not carry the accepted production identity');
  }
  return value;
}
