#[derive(Debug, Eq, PartialEq)]
struct Row {
    requirement: String,
    source: Vec<Evidence>,
    ir: Vec<Evidence>,
}

#[derive(Debug, Eq, PartialEq)]
struct Evidence {
    file: String,
    test: String,
}

fn item(file: &str, test: &str) -> Evidence {
    Evidence { file: file.into(), test: test.into() }
}

fn evidence(cell: &str) -> Vec<Evidence> {
    cell.split("; ")
        .map(|entry| {
            let entry = entry
                .strip_prefix('`')
                .and_then(|entry| entry.strip_suffix('`'))
                .unwrap_or_else(|| panic!("non-canonical evidence: {entry}"));
            let (file, test) = entry.split_once("::").expect("file-qualified evidence");
            item(file, test)
        })
        .collect()
}

fn rows(matrix: &str) -> Vec<Row> {
    let lines = matrix.lines().collect::<Vec<_>>();
    let header = lines
        .iter()
        .position(|line| *line == "| Requirement | Source evidence | Independent IR evidence |")
        .expect("matrix header");
    assert_eq!(lines[header + 1], "| --- | --- | --- |");
    lines[header + 2..]
        .iter()
        .take_while(|line| line.starts_with("| "))
        .map(|line| {
            let cells = line
                .strip_prefix("| ")
                .and_then(|line| line.strip_suffix(" |"))
                .expect("table delimiters")
                .split(" | ")
                .collect::<Vec<_>>();
            assert_eq!(cells.len(), 3);
            Row { requirement: cells[0].into(), source: evidence(cells[1]), ir: evidence(cells[2]) }
        })
        .collect()
}

fn assert_enabled(source: &str, evidence: &Evidence) {
    let declaration = format!("fn {}()", evidence.test);
    let lines = source.lines().collect::<Vec<_>>();
    let matches = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with(&declaration))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "evidence must resolve exactly once: {evidence:?}");
    let attributes = lines[..matches[0].0]
        .iter()
        .rev()
        .take_while(|line| line.trim().is_empty() || line.trim().starts_with("#["))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert!(attributes.iter().any(|line| line.trim() == "#[test]"), "enabled test: {evidence:?}");
    assert!(
        attributes.iter().all(|line| !line.trim().starts_with("#[ignore")),
        "ignored evidence is forbidden: {evidence:?}"
    );
}

fn source_file(path: &str) -> &'static str {
    match path {
        "nonindexed_static_owned_borrow.rs" => include_str!("nonindexed_static_owned_borrow.rs"),
        "active_enum_payload_borrow_source.rs" => {
            include_str!("active_enum_payload_borrow_source.rs")
        }
        "nonindexed_borrow_calls.rs" => include_str!("nonindexed_borrow_calls.rs"),
        "nonindexed_borrow_resources.rs" => include_str!("nonindexed_borrow_resources.rs"),
        "lexical_borrow_calls.rs" => include_str!("lexical_borrow_calls.rs"),
        "structured_graph_resources.rs" => include_str!("structured_graph_resources.rs"),
        _ => panic!("unbound source evidence: {path}"),
    }
}

macro_rules! ir {
    ($path:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../zryna-ir/src/data_ownership_v1/",
            $path
        ))
    };
}

fn ir_file(path: &str) -> &'static str {
    match path {
        "tests.rs" => ir!("tests.rs"),
        "nonindexed_owned_borrow_proof.rs" => ir!("tests/nonindexed_owned_borrow_proof.rs"),
        "active_enum_payload_borrow.rs" => ir!("tests/active_enum_payload_borrow.rs"),
        "borrow_nonindexed_call_scope.rs" => ir!("tests/borrow_nonindexed_call_scope.rs"),
        "borrow_resource_boundaries.rs" => ir!("tests/borrow_resource_boundaries.rs"),
        _ => panic!("unbound IR evidence: {path}"),
    }
}

fn section_items<'a>(matrix: &'a str, heading: &str) -> Vec<&'a str> {
    let lines = matrix.lines().collect::<Vec<_>>();
    let start = lines.iter().position(|line| *line == heading).expect("section heading");
    lines[start + 2..].iter().take_while(|line| line.starts_with("- ")).copied().collect()
}

#[test]
fn nonindexed_borrowing_matrix_binds_enabled_evidence_and_exact_boundaries() {
    let matrix = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/M3_NONINDEXED_OWNED_BORROWING_MATRIX.md"
    ));
    let parsed = rows(matrix);
    assert_eq!(parsed.len(), 4);
    assert_eq!(
        parsed.iter().map(|row| row.requirement.as_str()).collect::<Vec<_>>(),
        [
            "Owned roots and static Struct/FixedArray places retain exact ownership, masks, overlap, clone, replacement, and recovery",
            "Refined active enum payloads preserve exact variant, referent, parent cleanup, replacement, and lexical restoration",
            "Nested lexical and direct-call use neither clones authority nor permits escape and restores the exact owner",
            "Exact and first-extra resource dimensions, overflow, atomic rejection, and deterministic replay are checked",
        ]
    );
    for row in &parsed {
        assert!(!row.source.is_empty() && !row.ir.is_empty());
        for evidence in &row.source {
            assert_enabled(source_file(&evidence.file), evidence);
        }
        for evidence in &row.ir {
            assert_enabled(ir_file(&evidence.file), evidence);
        }
    }
    assert_eq!(
        section_items(matrix, "## Retained boundaries"),
        [
            "- Dynamic-index FixedArray and Vec borrowing remains owned by #254–#256 and #274, not this matrix.",
            "- Stored, returned, or captured borrows and borrow-carrying branch or loop edges remain rejected.",
            "- Implicit lifetime shortening, arbitrary reborrowing, moves through live borrows, interior mutability, and raw pointers remain rejected.",
            "- Borrowed imports, nominal type-import grammar, indirect calls, callbacks, recursion, wildcard arms, and terminating match arms remain outside this boundary.",
            "- Runtime no-alias checks, backends, driver and CLI routes, artifacts, public ABI/profile activation, and target execution remain unavailable.",
        ]
    );
    assert_eq!(
        section_items(matrix, "## Closure status"),
        [
            "- #337 supplies owned-root and static-projection authority.",
            "- #338 supplies refined active-enum payload authority.",
            "- #339 supplies lexical and direct-call composition.",
            "- #340 supplies hostile IR, resource, documentation, and checked-matrix evidence.",
            "- #275 is a compiler-only closure candidate; parent #269 remains open.",
        ]
    );
}
