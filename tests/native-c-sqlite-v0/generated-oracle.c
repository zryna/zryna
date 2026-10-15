/* Test-only selected SQLite API ledger, separate from private runtime allocations. */
static void *private_owners[128];
static uintptr_t private_addresses[128];
static size_t private_released[128], private_count, private_releases;
static size_t sqlite_depth, sqlite_attempts, sqlite_allocations, sqlite_releases;
static void *sqlite_owner;
static uintptr_t sqlite_address;
static size_t sqlite_length, private_after_sqlite, private_failures;
static int sqlite_returned, arm_private_failure;
static struct zryna_c_v0_outcome *observed_outcome;
void *__real_malloc(size_t);
void __real_free(void *);
void *__real_sqlite3_malloc(int);
void __real_sqlite3_free(void *);
void *__wrap_malloc(size_t size) {
  if (sqlite_depth) return __real_malloc(size);
  if (sqlite_returned) {
    ++private_after_sqlite;
    if (arm_private_failure && private_failures == 0) {
      ++private_failures;
      return NULL;
    }
  }
  void *pointer = __real_malloc(size);
  assert(pointer != NULL && private_count < 128);
  private_owners[private_count] = pointer;
  private_addresses[private_count] = (uintptr_t)pointer;
  private_released[private_count++] = 0;
  return pointer;
}
void __wrap_free(void *pointer) {
  if (sqlite_depth) { __real_free(pointer); return; }
  assert(pointer != NULL);
  size_t i = private_count;
  while (i && (private_released[i-1] || private_owners[i-1] != pointer)) --i;
  assert(i != 0 && private_released[i-1] == 0);
  private_released[i-1] = ++private_releases;
  __real_free(pointer);
}
void *__wrap_sqlite3_malloc(int length) {
  assert(length > 0 && length <= 4096 && sqlite_owner == NULL);
  ++sqlite_attempts;
  ++sqlite_depth;
  void *pointer = __real_sqlite3_malloc(length);
  --sqlite_depth;
  assert(pointer != NULL);
  for (size_t i=0;i<private_count;++i)
    if (!private_released[i]) assert((uintptr_t)pointer != private_addresses[i]);
  sqlite_owner = pointer;
  sqlite_address = (uintptr_t)pointer;
  sqlite_length = (size_t)length;
  ++sqlite_allocations;
  sqlite_returned = 1;
  return pointer;
}
void __wrap_sqlite3_free(void *pointer) {
  if (pointer == NULL) {
    ++sqlite_depth; __real_sqlite3_free(NULL); --sqlite_depth;
    return;
  }
  assert(pointer == sqlite_owner && sqlite_allocations == sqlite_releases+1);
  assert(observed_outcome != NULL && observed_outcome->owned.pointer == 0);
  const uint8_t *bytes = pointer;
  for (size_t i=0;i<sqlite_length;++i) assert(bytes[i] == (uint8_t)(i%256));
  ++sqlite_depth; __real_sqlite3_free(pointer); --sqlite_depth;
  sqlite_owner = NULL;
  ++sqlite_releases;
}
static size_t private_live(void) {
  size_t live = 0;
  for (size_t i=0;i<private_count;++i) live += private_released[i] == 0;
  return live;
}
static void reset_observation(void) {
  assert(private_live() == 0 && sqlite_owner == NULL);
  private_count = private_releases = 0;
  sqlite_attempts = sqlite_allocations = sqlite_releases = 0;
  sqlite_address = sqlite_length = private_after_sqlite = private_failures = 0;
  sqlite_returned = arm_private_failure = 0;
  observed_outcome = NULL;
}
static void input_bytes(struct zryna_c_v0_inputs *inputs, size_t length) {
  zryna_rt_o1_handle input = {0};
  assert(zryna_rt_o1_vec_allocate(1,length ? length : 1,&input) == 0);
  int32_t *data = (int32_t *)input.pointer;
  for (size_t i=0;i<length;++i) data[i] = (int32_t)(i%256);
  input.length = length;
  memcpy(&inputs->owned[0],&input,sizeof(input));
}
static void release_result(struct zryna_c_v0_outcome *outcome) {
  zryna_rt_o1_handle result;
  memcpy(&result,&outcome->owned,sizeof(result));
  assert(zryna_rt_o1_vec_release_storage(1,&result) == 0);
}
static void clean_context(const struct zryna_c_v0_context *context) {
  assert(context->busy == 0 && context->live == 0 && context->reserved == 0);
  assert(context->private_unresolved == 0);
}
