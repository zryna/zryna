//! Violating error-status output writes confer no guessed foreign release identity.

use super::*;

#[test]
fn separate_error_status_output_write_is_host_failure_without_guessed_release() {
    let original = library_source();
    let marker = "*out=copy; *count=length;\n  return 0;";
    assert_eq!(original.matches(marker).count(), 1);
    for allocates in [false, true] {
        let library = if allocates {
            original.replace(marker, "*out=copy; *count=length;\n  return 1;")
        } else {
            let start = "int32_t fixture_copy_bytes(const uint8_t *bytes,size_t length,uint8_t **out,size_t *count) {";
            assert_eq!(original.matches(start).count(), 1);
            let (prefix, rest) = original.split_once(start).expect("exact byte operation");
            assert_eq!(rest.matches("void fixture_release_bytes(").count(), 1);
            let (_, suffix) =
                rest.split_once("void fixture_release_bytes(").expect("exact named release");
            format!(
                "{prefix}{start}\n  (void)bytes; (void)length; (void)out;\n  *count=3;\n  return 1;\n}}\nvoid fixture_release_bytes({suffix}"
            )
        };
        assert_ne!(library, original);
        let client = format!(
            r"
int main(void) {{
  reset_observation();
  struct zryna_c_v0_context context; zryna_c_v0_context_initialize(&context);
  struct zryna_c_v0_inputs inputs={{.count=1}}; struct zryna_c_v0_outcome outcome;
  input_bytes(&inputs,3);
  assert(ENTRY(&context,&inputs,&outcome)==3 && outcome.tag==3);
  assert(outcome.trap==0 && outcome.value==0 && outcome.owned.pointer==0);
  assert(outcome.unresolved==1 && outcome.reserved==1 && context.poisoned==1);
  assert(context.live==0 && context.reserved==1 && context.private_unresolved==0);
  assert(owner_count=={owners} && releases==2 && live_count()=={live});
  assert(foreign_count(3,0)==0 && foreign_count(3,1)=={live});
  assert(ENTRY(&context,&inputs,&outcome)==3 && outcome.unresolved==1);
  assert(owner_count=={owners} && releases==2 && live_count()=={live});
  /* Only the physical oracle disposes actual malicious output; no generated free. */
  for(size_t i=0;i<owner_count;++i) if(!owner_releases[i]) __wrap_free(owners[i]);
  assert(live_count()==0 && releases==owner_count);
  return 0;
}}
",
            owners = if allocates { 3 } else { 2 },
            live = usize::from(allocates),
        );
        for sanitized in [false, true] {
            let fixture = Fixture::new();
            let requirements = requirements(&capture::reference(), "copied");
            let bytes = fixture.compile(&library);
            let snapshot = accept(&requirements, &bytes, &["free", "malloc"]);
            assert_eq!(snapshot.definitions().collect::<Vec<_>>(), DEFINITIONS);
            assert_eq!(snapshot.dependencies().collect::<Vec<_>>(), ["free", "malloc"]);
            let client = client.replace("ENTRY", &entry(&requirements, "copied"));
            assert!(
                harness::observe(&fixture, &requirements, &snapshot, &library, &client, sanitized,)
                    .status
                    .success()
            );
        }
    }
    for sanitized in [false, true] {
        assert!(observe("copied", &original, PRISTINE, sanitized).status.success());
    }
}

const PRISTINE: &str = r"
int main(void) {
  for(size_t failed=0;failed<2;++failed) {
    reset_observation();
    struct zryna_c_v0_context context; zryna_c_v0_context_initialize(&context);
    struct zryna_c_v0_inputs inputs={.count=1}; struct zryna_c_v0_outcome outcome;
    input_bytes(&inputs,3); attempts=0; fail_at=failed?2:0;
    assert(ENTRY(&context,&inputs,&outcome)==(failed?1U:0U));
    assert(outcome.tag==(failed?1U:0U) && outcome.unresolved==0);
    assert(context.poisoned==0 && context.live==0 && context.reserved==0);
    if(failed) {
      assert(outcome.status==1 && outcome.owned.pointer==0);
      assert(owner_count==2 && attempts==2 && live_count()==0);
    } else {
      assert(outcome.owned.length==3); release_result(&outcome);
    }
    assert(live_count()==0 && releases==owner_count);
  }
  return 0;
}
";
