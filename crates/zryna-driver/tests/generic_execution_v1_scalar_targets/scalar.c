/* Scalar ABI harness; canonical runtime remains real and every logical ABI event is forbidden. */
#if defined(__linux__)
#include <sys/prctl.h>
#endif
#include "entry_abi.h"
#include <assert.h>
#include <errno.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>
#if ENTRY_ARGUMENTS > 0
static int32_t argument(const char *text) {
  char *end=NULL;errno=0;intmax_t value=strtoimax(text,&end,10);
  assert(!errno && end!=text && !*end && value>=INT32_MIN && value<=INT32_MAX);return (int32_t)value;
}
#endif
int main(int argc,char **argv) {
  assert(argc==ENTRY_ARGUMENTS+1);const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
#if defined(__linux__)
  /* Piped crash collectors ignore RLIMIT_CORE; suppress dumps in this child. */
  assert(prctl(PR_SET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  assert(prctl(PR_GET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
#endif
  assert(setvbuf(stdout,NULL,_IONBF,0)==0);
  int32_t args[2]={0,0};
#if ENTRY_ARGUMENTS > 0
  for(int i=0;i<ENTRY_ARGUMENTS;i++)args[i]=argument(argv[i+1]);
#else
  (void)argv;
#endif
  (void)args;
  for(int retry=0;retry<3;retry++) {
#if ENTRY_ARGUMENTS == 0
    int32_t value=zryna_v1_e_entry();
#elif ENTRY_ARGUMENTS == 1
    int32_t value=zryna_v1_e_entry(args[0]);
#else
    int32_t value=zryna_v1_e_entry(args[0],args[1]);
#endif
    printf("result:%" PRId32 "\n",value);
  }
  return 0;
}
