//! A physical call counter distinguishes no foreign entry from an allocation-free C call.

use super::*;

const CLIENT: &str = r"
int main(void) {
  int32_t raw=99;
  const uint8_t small[]={1,2,3};
  sum_calls=0;
  assert(sum_bytes(small,3,&raw)==0 && raw==6 && sum_calls==1);
  const uint8_t valid[]={0,0x7f,0xc2,0x80,0xdf,0xbf,0xe0,0xa0,0x80,
    0xed,0x9f,0xbf,0xee,0x80,0x80,0xef,0xbf,0xbf,0xf0,0x90,0x80,0x80,
    0xf4,0x8f,0xbf,0xbf};
  const uint8_t invalid[][4]={{0x80,0,0,0},{0xc0,0x80,0,0},{0xc2,0,0,0},
    {0xe0,0x9f,0xbf,0},{0xed,0xa0,0x80,0},{0xf0,0x8f,0xbf,0xbf},
    {0xf4,0x90,0x80,0x80},{0xf5,0x80,0x80,0x80}};
  const size_t sizes[]={1,2,1,3,3,4,4,4};
  for(size_t test=0;test<11;++test) {
    reset_observation();
    struct zryna_c_v0_context context; zryna_c_v0_context_initialize(&context);
    struct zryna_c_v0_inputs inputs={.count=1};
    struct zryna_c_v0_outcome outcome;
    zryna_rt_o1_handle input={0};
    if(test==0) {
      assert(zryna_rt_o1_string_from_utf8_copy(NULL,0,&input)==0);
    } else if(test==1 || test==10) {
      assert(zryna_rt_o1_string_from_utf8_copy(valid,sizeof(valid),&input)==0);
    } else {
      const uint8_t ascii[]={'a','a','a','a'};
      assert(zryna_rt_o1_string_from_utf8_copy(ascii,sizes[test-2],&input)==0);
      memcpy((void *)input.pointer,invalid[test-2],sizes[test-2]);
    }
    memcpy(&inputs.owned[0],&input,sizeof(input));
    sum_calls=0;
    if(test==0 || test==1 || test==10) {
      int32_t expected=0;
      if(test!=0) for(size_t i=0;i<sizeof(valid);++i) expected+=valid[i];
      assert(ENTRY(&context,&inputs,&outcome)==0 && outcome.tag==0);
      assert(outcome.value==expected && sum_calls==1);
      assert(outcome.unresolved==0 && context.private_unresolved==0);
      assert(context.poisoned==0 && context.live==0 && context.reserved==0);
      assert(live_count()==0 && releases==owner_count);
    } else {
      assert(ENTRY(&context,&inputs,&outcome)==3 && outcome.tag==3);
      assert(sum_calls==0 && outcome.trap==0 && outcome.value==0);
      assert(outcome.owned.pointer==0 && outcome.unresolved==1);
      assert(context.private_unresolved==1 && context.poisoned==1);
      assert(owner_count==1 && live_count()==1 && releases==0);
      assert(ENTRY(&context,&inputs,&outcome)==3 && sum_calls==0);
      assert(outcome.unresolved==1 && live_count()==1 && releases==0);
      /* Explicit oracle disposal is not generated cleanup of this poisoned input. */
      assert(zryna_rt_o1_string_release(&input)==0);
      assert(live_count()==0 && releases==owner_count);
    }
  }
  return 0;
}
";

fn run_client(counter: harness::Counter, sanitized: bool) -> crate::native::BoundedProcessOutput {
    let fixture = Fixture::new();
    let requirements = requirements(&capture::reference(), "utf8Sum");
    let library = library_source();
    let snapshot = accept(&requirements, &fixture.compile(&library), &["free", "malloc"]);
    assert_eq!(snapshot.definitions().collect::<Vec<_>>(), DEFINITIONS);
    assert_eq!(snapshot.dependencies().collect::<Vec<_>>(), ["free", "malloc"]);
    let client = CLIENT.replace("ENTRY", &entry(&requirements, "utf8Sum"));
    harness::observe_with_sum_counter(
        &fixture,
        &requirements,
        &snapshot,
        &library,
        &client,
        sanitized,
        counter,
    )
}

#[test]
fn separate_utf8_corruption_rejects_before_foreign_call_and_retains_private_owner() {
    for sanitized in [false, true] {
        assert!(run_client(harness::Counter::Live, sanitized).status.success());
    }
}

#[test]
fn separate_sum_entry_counter_detects_missing_increment_then_pristine_recovers() {
    for sanitized in [false, true] {
        let failure = run_client(harness::Counter::MissingIncrement, sanitized);
        assert_eq!(failure.status.signal(), Some(libc::SIGABRT));
        assert!(String::from_utf8_lossy(&failure.stderr).contains("sum_calls==1"));
        assert!(run_client(harness::Counter::Live, sanitized).status.success());
    }
}
