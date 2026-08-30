# Zryna JavaScript backend

Direct JavaScript lowering from `VerifiedProgram`.

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
