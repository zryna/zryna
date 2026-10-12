//! An actual foreign release exit cannot become a language result or crashed-owner cleanup.

use super::*;

const CLIENT: &str = r#"
#include <stddef.h>
/* Independent mirror of the non-observation LP64 allocation layout. */
struct observed_allocation_header {
  uint64_t magic, size;
  uintptr_t base, data, previous, next;
  uint32_t alignment, pending_last_strong;
};
_Static_assert(sizeof(uintptr_t)==8 && sizeof(int32_t)==4, "private LP64 widths");
_Static_assert(_Alignof(int32_t)==4, "private i32 alignment");
_Static_assert(sizeof(struct observed_allocation_header)==56, "private header width");
_Static_assert(offsetof(struct observed_allocation_header,magic)==0, "magic offset");
_Static_assert(offsetof(struct observed_allocation_header,size)==8, "size offset");
_Static_assert(offsetof(struct observed_allocation_header,base)==16, "base offset");
_Static_assert(offsetof(struct observed_allocation_header,data)==24, "data offset");
_Static_assert(offsetof(struct observed_allocation_header,previous)==32, "previous offset");
_Static_assert(offsetof(struct observed_allocation_header,next)==40, "next offset");
_Static_assert(offsetof(struct observed_allocation_header,alignment)==48, "alignment offset");
_Static_assert(offsetof(struct observed_allocation_header,pending_last_strong)==52, "pending offset");
static uintptr_t initialized_private_copy(void) {
  size_t pending=owner_count-1;
  const size_t payload=3*sizeof(int32_t), header_size=56, alignment=4;
  assert(owner_sizes[pending]==payload+header_size+alignment-1);
  assert(owner_releases[pending]==0 && owners[pending]!=NULL);
  uintptr_t base=owner_addresses[pending];
  assert(base<=UINTPTR_MAX-owner_sizes[pending]);
  uintptr_t data=(base+header_size+alignment-1)&~(uintptr_t)(alignment-1);
  size_t offset=(size_t)(data-base);
  assert(offset>=header_size && offset+payload<=owner_sizes[pending]);
  struct observed_allocation_header header;
  memcpy(&header,(const uint8_t *)owners[pending]+offset-header_size,sizeof(header));
  assert(header.magic==UINT64_C(0x7a72796e616f3175) && header.size==payload);
  assert(header.base==base && header.data==data && header.alignment==alignment);
  assert(header.previous==0 && header.pending_last_strong==0);
  int32_t copied[3]; memcpy(copied,(const uint8_t *)owners[pending]+offset,sizeof(copied));
  assert(copied[0]==0 && copied[1]==1 && copied[2]==2);
  return data;
}
static struct zryna_c_v0_context *tested_context;
static struct zryna_c_v0_outcome *tested_outcome;
static int force_preexposure;
void fixture_test_release_observer(uint8_t *bytes) {
  assert(tested_context!=NULL && tested_outcome!=NULL);
  assert(tested_context->live==1 && tested_context->owners[0].state==2);
  assert(tested_context->owners[0].pointer==(uintptr_t)bytes);
  assert(owner_count==4 && live_count()==4 && releases==0);
  assert(foreign_count(3,1)==1 && foreign_count(3,0)==0);
  size_t exact_foreign=0;
  for(size_t i=0;i<owner_count;++i)
    if(owner_addresses[i]==(uintptr_t)bytes && owner_sizes[i]==3 && owner_releases[i]==0)
      ++exact_foreign;
  assert(exact_foreign==1);
  assert(bytes[0]==0 && bytes[1]==1 && bytes[2]==2);
  uintptr_t pending_copy=initialized_private_copy();
  assert(pending_copy!=(uintptr_t)bytes);
  if(force_preexposure) {
    /* Independent client mutant publishes the actual initialized private Vec.
       It changes the observer client, not generated machine code or runtime. */
    tested_outcome->owned.pointer=pending_copy;
    tested_outcome->owned.length=3;
    tested_outcome->owned.capacity=3;
  }
  assert(tested_outcome->owned.pointer == 0);
  assert(tested_outcome->owned.length==0 && tested_outcome->owned.capacity==0);
  assert(tested_outcome->value==0);
}
int main(void) {
  reset_observation();
  struct zryna_c_v0_context context; zryna_c_v0_context_initialize(&context);
  struct zryna_c_v0_inputs inputs={.count=1}; struct zryna_c_v0_outcome outcome;
  input_bytes(&inputs,3); tested_context=&context; tested_outcome=&outcome;
  force_preexposure=PREEXPOSURE;
  (void)ENTRY(&context,&inputs,&outcome);
  abort(); /* Returning from the injected release must never receive exit86 credit. */
}
"#;

fn observe_fault(preexposure: bool, sanitized: bool) -> crate::native::BoundedProcessOutput {
    let fixture = Fixture::new();
    let requirements = requirements(&capture::reference(), "copied");
    let original = library_source();
    let marker = "void fixture_release_bytes(uint8_t *bytes) {free(bytes);}";
    assert_eq!(original.matches(marker).count(), 1);
    let library = original.replace(
        marker,
        "void fixture_test_release_observer(uint8_t *);\nvoid fixture_release_bytes(uint8_t *bytes) { fixture_test_release_observer(bytes); _Exit(86); }",
    );
    assert_ne!(library, original);
    let bytes = fixture.compile(&library);
    // These two test dependencies must not disappear into the original malloc/free inventory.
    reject(&requirements, &bytes, &["free", "malloc"]);
    let dependencies = ["_Exit", "fixture_test_release_observer", "free", "malloc"];
    let snapshot = accept(&requirements, &bytes, &dependencies);
    assert_eq!(snapshot.definitions().collect::<Vec<_>>(), DEFINITIONS);
    assert_eq!(snapshot.dependencies().collect::<Vec<_>>(), dependencies);
    let client = CLIENT
        .replace("ENTRY", &entry(&requirements, "copied"))
        .replace("PREEXPOSURE", if preexposure { "1" } else { "0" });
    harness::observe(&fixture, &requirements, &snapshot, &library, &client, sanitized)
}

#[test]
fn separate_foreign_release_exit_never_exposes_private_result() {
    for sanitized in [false, true] {
        let fault = observe_fault(false, sanitized);
        assert_eq!(fault.status.code(), Some(86));
        assert!(fault.stderr.is_empty(), "{}", String::from_utf8_lossy(&fault.stderr));
        let mutant = observe_fault(true, sanitized);
        assert_eq!(mutant.status.signal(), Some(libc::SIGABRT));
        assert!(
            String::from_utf8_lossy(&mutant.stderr).contains("tested_outcome->owned.pointer == 0")
        );
        // This new process proves stage recovery, not cleanup of the terminated foreign owner.
        assert!(observe("copied", &library_source(), PRISTINE, sanitized).status.success());
    }
}

const PRISTINE: &str = r"
int main(void) {
  reset_observation();
  struct zryna_c_v0_context context; zryna_c_v0_context_initialize(&context);
  struct zryna_c_v0_inputs inputs={.count=1}; struct zryna_c_v0_outcome outcome;
  input_bytes(&inputs,3);
  assert(ENTRY(&context,&inputs,&outcome)==0 && outcome.tag==0);
  assert(outcome.unresolved==0 && context.live==0 && context.reserved==0);
  assert(context.poisoned==0 && foreign_count(3,0)==1 && foreign_count(3,1)==0);
  release_result(&outcome); assert(live_count()==0 && releases==owner_count);
  return 0;
}
";
