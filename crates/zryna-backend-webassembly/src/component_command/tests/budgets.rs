//! Independent binary syntax fixtures exercise rejection before recursive parser allocation.

use wasm_encoder::Encode;

use super::super::{graph_budget, type_budget::TypeBudget};

#[test]
fn binary_type_budget_accepts_exact_type_count_and_rejects_first_extra_across_sections() {
    let mut exact = Vec::new();
    4096_u32.encode(&mut exact);
    exact.extend(std::iter::repeat_n(0x7f, 4096));
    let mut budget = TypeBudget::default();
    budget.section(&exact, 0).expect("exact 4096 primitive definitions");
    assert_eq!(budget.section(&[1, 0x7f], 0).expect_err("first extra type").code(), "ZRYNA-W4013");
}

#[test]
fn binary_type_budget_accepts_exact_entry_count_and_rejects_first_extra() {
    fn records(last: u32) -> Vec<u8> {
        let mut bytes = vec![4];
        for count in [4096, 4096, 4096, last] {
            bytes.push(0x72);
            count.encode(&mut bytes);
            for index in 0..count {
                let name = format!("field-{index}");
                name.encode(&mut bytes);
                bytes.push(0x79);
            }
        }
        bytes
    }
    TypeBudget::default().section(&records(4092), 0).expect("16384 syntax entries");
    assert_eq!(
        TypeBudget::default().section(&records(4093), 0).expect_err("16385 entries").code(),
        "ZRYNA-W4013"
    );
}

#[test]
fn binary_type_budget_rejects_nesting_before_reading_the_nested_body() {
    // One instance declaration starts another instance type, without a body to parse.
    let nested = [1, 0x42, 1, 1, 0x42];
    assert_eq!(
        TypeBudget::default().section(&nested, 0).expect_err("nested type").code(),
        "ZRYNA-W4013"
    );
    assert!(TypeBudget::default().section(&[1, 0x41], 0).is_err());
    assert!(TypeBudget::default().section(&[1, 0x3f], 0).is_err());
}

#[test]
fn binary_type_name_limit_accepts_exact_and_rejects_first_extra() {
    fn record(name_bytes: usize) -> Vec<u8> {
        let mut bytes = vec![1, 0x72, 1];
        "x".repeat(name_bytes).encode(&mut bytes);
        bytes.push(0x79);
        bytes
    }
    TypeBudget::default().section(&record(256), 0).expect("exact name limit");
    assert!(TypeBudget::default().section(&record(257), 0).is_err());
}

#[test]
fn binary_graph_budget_rejects_extra_arguments_and_canonical_options_before_allocation() {
    let mut exact = vec![1, 0, 1, 1];
    "scalar-core".encode(&mut exact);
    exact.extend([0x12, 0]);
    graph_budget::core_instances(&exact, 0).expect("one scalar instance argument");
    assert!(graph_budget::core_instances(&[1, 0, 1, 2], 0).is_err());
    graph_budget::canonical(&[1, 0, 0, 0, 0, 0], 0).expect("one lift with no options");
    assert!(graph_budget::canonical(&[1, 0, 0, 0, 1], 0).is_err());
}

#[test]
fn binary_graph_budget_requires_one_component_export_and_no_trailing_data() {
    let mut exact = vec![1, 1, 1, 0];
    "run".encode(&mut exact);
    exact.extend([1, 0]);
    graph_budget::component_instance(&exact, 0).expect("one function export");
    exact.push(0);
    assert!(graph_budget::component_instance(&exact, 0).is_err());
    assert!(graph_budget::component_instance(&[1, 1, 2], 0).is_err());
}
