# Zryna JavaScript backend

Direct JavaScript lowering from sealed M1 and M2 verified programs.

The backend consumes only sealed function views and uses each export's sealed scalar ABI
JavaScript name. It emits a deterministic ECMAScript module with LF line endings and a final
newline. One local temporary is emitted for every canonical arena expression, in arena order,
followed by a validated return of the verified body. Emission is iterative and linear in the
verified IR size. Current `i32` addition is rendered as `(left + right) | 0` to preserve signed
wrapping 32-bit behavior.

Every public `I32V1` function checks exact arity and accepts only primitive JavaScript Numbers that
are finite integral signed 32-bit values and are not negative zero. Parameters and results are
validated without coercion. Stable backend diagnostics guard impossible verified-profile states.
Host-boundary failures use stable scalar ABI codes; carrier cases are exercised by the shared ABI
fixture and exact arity by driver integration.

The private Boolean carrier validator accepts only primitive JavaScript Booleans and is tested
against the shared scalar ABI fixture. It does not enable Boolean source: `bool` remains rejected
by the current Universal IR verifier until every active universal backend implements that profile.
Artifact publication and Node.js execution belong to `zryna-driver`, not this backend.

The separate internal `emit_control_flow` boundary accepts only the opaque verified
`control_flow_v1::VerifiedProgram`; raw M2 IR is not accepted. It emits every M2 scalar operation,
direct call, return, jump, and branch in canonical module/function/block order. Dense private names
come only from sealed identities. Entry exports use private typed wrappers plus scalar-ABI export
aliases, so public names cannot shadow implementation bindings. Jump and branch arguments are
copied through scratch slots before target parameters are assigned, preserving simultaneous loop
and merge edges. Conditions compare explicitly with `true` rather than using truthiness.

M2 multiplication uses a private exact low-32-bit decomposition instead of an ambient mutable
intrinsic. The same linear renderer first counts every exact byte and then emits into an exactly
reserved string, returning `ZRYNA-J2003` instead of an artifact beyond 32 MiB. Driver tests import
the public aliases and execute exact `i32` and primitive `bool` wrappers through the pinned,
bounded Node capability; backend tests cover control flow, calls, collisions, repeated bytes, and
exact rendered budget boundaries. This
internal API does not activate M2 in the public driver or CLI; M2 WebAssembly, native, manifest v2,
and three-target conformance remain unavailable.
