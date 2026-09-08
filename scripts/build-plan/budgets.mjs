function collectionBound(value, limit, label) {
  if (Array.isArray(value) && value.length > limit) {
    throw new Error(`P361-BUDGET: ${label} exceeds ${limit}`);
  }
}

export function validateCollectionBudgets(document) {
  const plan = document?.sourcePlan;
  collectionBound(plan?.packages, 16, 'packages');
  collectionBound(plan?.sources, 256, 'sources');
  collectionBound(plan?.targets, 3, 'targets');
  collectionBound(plan?.hostTools, 8, 'host tools');
  collectionBound(plan?.outputs, 16, 'outputs');
  collectionBound(plan?.host?.environment, 8, 'host environment');
  for (const pkg of Array.isArray(plan?.packages) ? plan.packages : []) {
    collectionBound(pkg?.dependencies, 8, 'package dependencies');
  }
  for (const target of Array.isArray(plan?.targets) ? plan.targets : []) {
    collectionBound(target?.features, 0, 'target features');
  }
  for (const tool of Array.isArray(plan?.hostTools) ? plan.hostTools : []) {
    collectionBound(tool?.targets, 3, 'host tool targets');
  }

  const appendix = document?.nativeAppendix;
  collectionBound(appendix?.acquisition?.targetLibraries, 32, 'target libraries');
  collectionBound(appendix?.acquisition?.sysroots, 8, 'sysroots');
  collectionBound(appendix?.acquisition?.staticArtifacts, 32, 'static artifacts');
  collectionBound(appendix?.acquisition?.sharedArtifacts, 32, 'shared artifacts');
  collectionBound(appendix?.acquisition?.runtimeDependencies, 32, 'runtime dependencies');
  collectionBound(appendix?.compilation?.steps, 64, 'compilation steps');
  for (const step of Array.isArray(appendix?.compilation?.steps) ? appendix.compilation.steps : []) {
    collectionBound(step?.inputs, 64, 'compilation inputs');
  }
  collectionBound(appendix?.linking?.inputs, 96, 'linker inputs');
}
