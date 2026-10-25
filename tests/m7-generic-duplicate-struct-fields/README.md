# Duplicate generic Struct fields

These complete source and literal syntax-v5 packets preserve every initializer,
including the second duplicate and its expression. Expected packets were authored
independently before changing either producer. Valid controls use the same forms.

The accepted decisions require `ZRYNA-M7006` at the second field name. The explicit
case includes a UTF-8 prefix; shorthand and named-import alias cases retain exact
half-open byte spans in `manifest.json`. Neither producer assigns semantic authority.
The source verifier authenticates each complete packet; the original body checker
then reports the single owning diagnostic. Omitting the duplicate must fail source
verification. Invalid sources never acquire executable authority.

The frontend integration also covers malformed source and the existing 1,024-field
syntax ceiling with its first-extra rejection. Repeated fields at that ceiling are
syntax input; semantic duplicate rejection remains required. Old v2–v4 producer
behavior and all existing resource limits stay unchanged.
