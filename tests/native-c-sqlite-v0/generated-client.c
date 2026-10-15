/* SCENARIO and ENTRY are supplied only by the fixed private Rust test harness. */
int main(void) {
  if (SCENARIO == 5) {
    volatile size_t length = 1;
    uint8_t *pointer = malloc(length);
    assert(pointer != NULL);
    pointer[length] = 7; /* Independent live ASan heap diagnostic control. */
    free(pointer);
    return 0;
  }
  if (SCENARIO == 6) {
    volatile int32_t maximum = INT32_MAX;
    volatile int32_t overflow = maximum + 1;
    (void)overflow;
    return 0;
  }
  const size_t lengths[] = {0,3,4096,4097,1,1};
  size_t cases = SCENARIO == 0 ? 6 : 1;
  for (size_t test=0;test<cases;++test) {
    reset_observation();
    struct zryna_c_v0_context context; zryna_c_v0_context_initialize(&context);
    struct zryna_c_v0_inputs inputs = {.count=1};
    struct zryna_c_v0_outcome outcome = {0};
    size_t length = SCENARIO == 0 ? lengths[test] : 3;
    input_bytes(&inputs,length);
    uintptr_t input = inputs.owned[0].pointer;
    if (SCENARIO == 0 && test >= 4)
      ((int32_t *)input)[0] = test == 4 ? -1 : 256;
    observed_outcome = &outcome;
    arm_private_failure = SCENARIO == 1;
    uint32_t tag = ENTRY(&context,&inputs,&outcome);
    if (SCENARIO == 1 || SCENARIO == 2) {
      assert(tag == 2 && "private-copy failure must trap");
      assert(outcome.tag == 2 && outcome.trap == 4 && outcome.status == 1);
      assert(private_failures == 1 && private_after_sqlite == 1);
      assert(outcome.owned.pointer == 0 && outcome.value == 0 && outcome.unresolved == 0);
    } else if (SCENARIO == 3) {
      assert(tag == 3 && "malformed SQLite result must be rejected");
      assert(outcome.tag == 3 && outcome.trap == 0 && outcome.value == 0);
      assert(outcome.owned.pointer == 0 && outcome.unresolved == 0);
      assert(context.poisoned == 1 && private_after_sqlite == 0);
    } else if (SCENARIO == 0 && test >= 3) {
      assert(tag == 2 && outcome.trap == (test == 3 ? 2U : 3U));
      assert(outcome.tag == tag && outcome.owned.pointer == 0 && outcome.value == 0);
      assert(sqlite_attempts == 0 && sqlite_allocations == 0 && sqlite_releases == 0);
      assert(outcome.unresolved == 0);
    } else {
      assert(tag == 0 && outcome.tag == 0 && outcome.unresolved == 0);
      assert(outcome.owned.length == length && outcome.owned.capacity == length);
      assert(sqlite_owner == NULL && "selected SQLite owner must be released exactly once");
      if (length) {
        assert(outcome.owned.pointer != 0 && outcome.owned.pointer != input);
        assert(outcome.owned.pointer != sqlite_address);
        assert(private_after_sqlite == 1 && private_live() == 1);
        int32_t *result = (int32_t *)outcome.owned.pointer;
        for (size_t i=0;i<length;++i) assert(result[i] == (int32_t)(i%256));
        release_result(&outcome);
      } else {
        assert(outcome.owned.pointer == 0 && sqlite_attempts == 0);
        assert(sqlite_allocations == 0 && private_after_sqlite == 0);
      }
    }
    clean_context(&context);
    if (SCENARIO != 3) assert(context.poisoned == 0);
    assert(sqlite_owner == NULL && sqlite_releases == sqlite_allocations);
    if (length <= 4096 && !(SCENARIO == 0 && test >= 4)) {
      assert(sqlite_allocations == (length != 0));
      assert(sqlite_attempts == sqlite_allocations);
    }
    assert(private_live() == 0 && private_releases == private_count);
  }
  return 0;
}
