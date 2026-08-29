# Zryna JavaScript backend

Direct JavaScript lowering from `VerifiedProgram`.

The backend consumes only sealed function views and uses the exact verified logical export name.
It emits one local temporary for every canonical arena expression, in arena order, followed by a
return of the verified body. Emission is iterative and linear in the verified IR size. Current
`i32` addition is rendered as `(left + right) | 0` to preserve signed wrapping 32-bit behavior.
