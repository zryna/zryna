//! Live C-only probes after the independently checked original eight-case corpus.
use super::*;
const TAIL: &str = "clear_observed();puts(\"ordinary-rust-bytes:4entered4prechecks\");return 0;";
const HEAP: &str = r#"
 fputs("m5-c-sanitizer:heap-overrun\n",stderr);fflush(stderr);
 volatile size_t offset=1;
 volatile unsigned char *probe=__real_malloc(1);assert(probe);
 probe[0]=0;
 volatile unsigned char observed=probe[offset];(void)observed;
 __real_free((void *)probe);
"#;
const OVERFLOW: &str = r#"
 fputs("m5-c-sanitizer:signed-overflow\n",stderr);fflush(stderr);
 volatile int32_t largest=INT32_MAX;
 volatile int32_t increment=1;
 volatile int32_t observed=largest+increment;(void)observed;
"#;
pub(super) fn client(request: &ImportRequest<'_>, case: Case) -> Result<String, Diagnostic> {
    let original = clients::import(request, Mode::Bytes { missing_increment: false })?;
    if original.matches(TAIL).count() != 1 {
        return Err(rejected());
    }
    let probe = match case {
        Case::Positive => "",
        Case::HeapOverrun => HEAP,
        Case::SignedOverflow => OVERFLOW,
    };
    Ok(original.replacen(
        TAIL,
        &format!(
            "clear_observed();{probe}puts(\"ordinary-rust-bytes:4entered4prechecks\");return 0;"
        ),
        1,
    ))
}
