# Native allocation registry controls

These are raw canonical-runtime controls, not authenticated source/IR or recursive-profile qualification. The fixture explicitly includes the private `runtime/native/generic_execution_v1.c` aggregation in one translation unit so it can deliberately find real collision chains without exposing registry internals as a runtime API. It does not change the runtime's element-layout marker or exported declarations.

Run both configurations with assertions enabled, for example from the repository root:

```sh
cc -std=c11 -pedantic -Wall -Wextra -Werror -O2 -fsanitize=address,undefined -fno-omit-frame-pointer tests/m7-generic-native-registry/runtime-registry.c -o /tmp/native-registry-controls
/tmp/native-registry-controls
cc -std=c11 -pedantic -Wall -Wextra -Werror -O2 -fsanitize=address,undefined -fno-omit-frame-pointer -DZRYNA_M3_OBSERVATION=1 tests/m7-generic-native-registry/runtime-registry.c -o /tmp/native-registry-observation-controls
/tmp/native-registry-observation-controls
```

The ordinary fixture creates8193 distinct live allocations, forcing a chain of at least three in4096 buckets. It grows an interior collision owner while its neighbors remain live, then checks middle/tail/head removal, remaining payload identities and same-size growth, duplicate/foreign/interior rejection, atomic allocation failure, shrink/zero growth and retry. Private chain checks supplement public logical-ABI observations; they do not establish a source proof.

Both modes check unchanged header offsets/size and malloc request geometry, canonical control-count/pending behavior, and no malloc call during release. Observation mode keeps the original global list and checks exact newest-first scratch finalization, retained owned-leak rejection and later recovery. Registry performance is unchanged in that mode.

The fixed ordinary index adds no owner limit: collisions remain linked lists and never allocate separate registry storage. It is not a worst-case constant-time or concurrency guarantee. Deep native Clone/drop glue, full17-operation conformance, supported-platform and performance qualification require separate checks.

The aggregation joins the three private GenericExecutionV1 runtime parts. Verified-program tests
assemble the same bytes only through a helper requiring their actual `VerifiedExecution`; the
legacy/default driver continues to read the byte-preserved `ownership_runtime_v1.c`. The raw
observation-mode compile above tests the generic implementation branch and grants no M3 profile,
public selector, or new execution permission.
