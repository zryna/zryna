use super::{BOX, Failure, claim, original_types};

fn ownership_rejects(source: &str) {
    original_types(&[("main.zry", source)]);
    let Failure::Diagnostics(errors) =
        claim(&[("main.zry", source)]).expect_err("nominal transport must retain ownership")
    else {
        panic!("owning diagnostic required");
    };
    assert_eq!(errors[0].code(), "ZRYNA-M7007", "{errors:?}");
}

#[test]
fn nominal_copy_transport_checks_inactive_owned_members_and_opaque_originals() {
    for control in ["if(flag){item=next;}", "while(flag){item=next;}"] {
        for (declaration, ty, first, next) in [
            (BOX, "Box<String>", "Box.none<String>()", "Box.none<String>()"),
            (
                "interface Fixed<T extends ZrynaValue> extends ZrynaEnum {none:ZrynaNone;text:String;}",
                "Fixed<i32>",
                "Fixed.none<i32>()",
                "Fixed.none<i32>()",
            ),
        ] {
            let control = control.replace("next", next);
            ownership_rejects(&format!(
                "{declaration} export function root(flag:bool):i32 {{let item:{ty} ={first}; {control} return 0;}}"
            ));
        }
        for demand in [false, true] {
            let call =
                if demand { "bad<i32>(flag,Box.value<i32>(1),Box.value<i32>(2))" } else { "0" };
            ownership_rejects(&format!(
                "{BOX} function bad<T extends ZrynaValue>(flag:bool,first:Box<T>,next:Box<T>):i32 {{let item:Box<T> =first; {control} return 0;}} export function root(flag:bool):i32 {{return {call};}}"
            ));
        }
    }
}

#[test]
fn nominal_copy_destination_never_replaces_a_shared_or_exclusive_loan_root() {
    for access in ["Borrow", "BorrowMut"] {
        let borrow = if access == "Borrow" { "borrow" } else { "borrowMut" };
        for control in ["if(flag){item=Box.value<i32>(7);}", "while(flag){item=Box.value<i32>(7);}"]
        {
            ownership_rejects(&format!(
                "{BOX} export function root(flag:bool):i32 {{let item:Box<i32> =Box.none<i32>(); const view:{access}<Box<i32>> ={borrow}(item); {control} return 0;}}"
            ));
        }
    }
}
