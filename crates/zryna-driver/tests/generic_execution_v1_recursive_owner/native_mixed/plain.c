#if defined(__linux__)
#include <sys/prctl.h>
#endif
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <sys/resource.h>
extern int32_t zryna_v1_e_entry(void);
int main(void) {
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
#if defined(__linux__)
  /* Piped crash collectors ignore RLIMIT_CORE; suppress dumps in this child. */
  assert(prctl(PR_SET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  assert(prctl(PR_GET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
#endif
  for(unsigned i=0;i<2;i++) { int32_t value=zryna_v1_e_entry();assert(value==97);printf("result:%d\n",value); }
  return 0;
}
