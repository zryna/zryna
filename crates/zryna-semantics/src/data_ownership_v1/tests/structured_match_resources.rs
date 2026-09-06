use super::assert_structured_resource_case;

#[test]
fn complete_enum_match_value_place_transition_and_cleanup_resources_are_exact() {
    for shape in 26..28 {
        for resource in 0..5 {
            for extra in [0, 1, usize::MAX] {
                assert_structured_resource_case(shape, resource, extra);
            }
        }
    }
}
