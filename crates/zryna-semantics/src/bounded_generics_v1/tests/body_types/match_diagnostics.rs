//! Exact payload provenance distinguishes invalid borrowed conversion from type inequality.
use super::{declarations, errors, passes, project};

#[test]
fn borrowed_payload_owned_result_rejects_at_binding_for_both_accesses_and_arm_orders() {
    for access in ["Borrow", "BorrowMut"] {
        for family in ["Option<String>", "Choice<String>"] {
            let name = family.split('<').next().expect("family spelling");
            for reversed in [false, true] {
                let none = format!("\"{name}.none\":()=>\"\"");
                let some = format!("\"{name}.some\":(item)=>item");
                let arms =
                    if reversed { format!("{some},{none}") } else { format!("{none},{some}") };
                let source = format!(
                    "interface Choice<T extends ZrynaValue> extends ZrynaEnum {{none:ZrynaNone;some:T;}} function probe(loan:{access}<{family}>):i32 {{const moved:String=match(loan,{{{arms}}}); return 0;}}"
                );
                let errors = errors(&source);
                assert_eq!(errors.len(), 1, "{source}: {errors:?}");
                assert_eq!(errors[0].code(), "ZRYNA-M7007");
                let start = source.find("=>item").expect("original arm result") + 2;
                let at = errors[0].primary_span().expect("borrowed binding result");
                assert_eq!((at.start() as usize, at.end() as usize), (start, start + 4));
            }
        }
    }
}

#[test]
fn unrelated_borrowed_payload_result_inequality_keeps_m7006() {
    for access in ["Borrow", "BorrowMut"] {
        let source = format!(
            "function probe(loan:{access}<Option<String>>):i32 {{match(loan,{{\"Option.none\":()=>1,\"Option.some\":(item)=>item}}); return 0;}}"
        );
        let errors = errors(&source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M7006");
    }
}

#[test]
fn equal_borrowed_arm_results_keep_symbolic_type_proofs_and_explicit_clone_remains_valid() {
    for access in ["Borrow", "BorrowMut"] {
        let source = format!(
            "function select<T extends ZrynaValue>(loan:{access}<Result<T,T>>):{access}<T> {{return match(loan,{{\"Result.ok\":(a)=>a,\"Result.err\":(b)=>b}});}}"
        );
        let input = project(&[("main.zry", &source)]);
        let context = declarations(&input);
        super::check_body_types(&context)
            .expect("symbolic borrowed type proof is not an escape seal");
    }
    passes(
        "function probe(value:Option<String>):String {return match(value,{\"Option.none\":()=>\"\",\"Option.some\":(item)=>clone(item)});}",
    );
}
