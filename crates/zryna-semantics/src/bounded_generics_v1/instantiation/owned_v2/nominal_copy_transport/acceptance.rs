use super::{BOX, MARKER, original_types, raw, seal, with_claim};

#[test]
fn nominal_concrete_copy_branch_and_loop_sources_seal_exact_nominal_parameters() {
    for (declaration, ty, first, next) in [
        (BOX, "Box<i32>", "Box.none<i32>()", "Box.value<i32>(7)"),
        (BOX, "Box<bool>", "Box.value<bool>(true)", "Box.value<bool>(false)"),
        (
            BOX,
            "Box<Option<Result<bool,i32>>>",
            "Box.value<Option<Result<bool,i32>>>(Option.some<Result<bool,i32>>(Result.ok<bool,i32>(true)))",
            "Box.value<Option<Result<bool,i32>>>(Option.some<Result<bool,i32>>(Result.err<bool,i32>(7)))",
        ),
        (MARKER, "Marker<String>", "Marker.none<String>()", "Marker.code<String>(true)"),
        (
            "interface Plain extends ZrynaEnum {none:ZrynaNone;code:i32;}",
            "Plain",
            "Plain.none()",
            "Plain.code(7)",
        ),
    ] {
        for control in [
            format!("if(flag){{item={next};}}"),
            format!("let go:bool=flag; while(go){{item={next}; go=false;}}"),
        ] {
            let source = format!(
                "{declaration} export function root(flag:bool):i32 {{let item:{ty} ={first}; {control} return 7;}}"
            );
            original_types(&[("main.zry", &source)]);
            with_claim(&[("main.zry", &source)], |claim, syntax, sources, linear, linux| {
                let parameters = claim
                    .graph
                    .functions
                    .iter()
                    .flat_map(|function| &function.blocks)
                    .filter(|block| block.id != 0)
                    .flat_map(|block| &block.parameters)
                    .filter_map(|parameter| match parameter.ty {
                        raw::Type::Stored(id) => linear.types().find(|ty| ty.id().index() == id),
                        _ => None,
                    })
                    .filter(|ty| matches!(ty.key().first(), Some(0x11 | 0x13)))
                    .collect::<Vec<_>>();
                assert!(!parameters.is_empty(), "a nominal value crosses the actual join/header");
                assert!(parameters.iter().all(|ty| ty.drop_kind() == 0));
                seal(&claim, syntax, sources, linear, linux);
            });
        }
    }
}
