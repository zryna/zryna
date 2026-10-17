// Separate reviewed-source obligations; no gate execution, v1 capability or support activation.
import { registry as standalone } from './registry.mjs';

const rename = role => ({ 'playground-toolkit': 'source-composition',
  'bubblewrap-linux': 'daemon-supervisor-linux', 'python-linux': 'docker-engine-linux' }[role] ?? role);
const freeze = value => {
  if (value && typeof value === 'object') {
    Object.values(value).forEach(freeze);
    Object.freeze(value);
  }
  return value;
};

export const sourceRegistry = freeze({ format: 'zryna.m6-source-registry.v2', version: 2,
  route: 'reviewed-source-v1',
  products: { ...standalone.products, playground: '0.1.0' },
  requiredArtifacts: [...standalone.requiredArtifacts.map(rename), 'compiler-materials',
    'evaluator-materials', 'compiler-runtime-image', 'evaluator-runtime-image'],
  supersession: [
    { gate: 'publication-linux', previousCase: 'outer-signature',
      case: 'published-compiler-signature',
      reason: 'Actual independently authenticated published compiler plus separately reviewed source composition; no standalone toolkit capability.' },
  ],
  gates: standalone.gates.map(gate => ({ ...gate,
    cases: gate.cases.map(id => gate.id === 'publication' && id === 'outer-signature'
      ? 'published-compiler-signature' : id),
    artifactRoles: [...gate.artifactRoles.map(rename), ...(gate.id === 'publication' ? ['playground-compiler'] : []),
      ...(['containment', 'browser'].includes(gate.id)
      ? ['compiler-materials', 'evaluator-materials', 'compiler-runtime-image', 'evaluator-runtime-image'] : [])],
  })),
});

export const requiredSourceGates = Object.freeze(sourceRegistry.gates.flatMap(gate =>
  gate.platforms.map(platform => `${gate.id}-${platform}`)));
