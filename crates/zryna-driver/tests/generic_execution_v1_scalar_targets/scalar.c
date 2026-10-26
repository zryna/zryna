/* Scalar ABI harness; canonical runtime remains real and every logical ABI event is forbidden. */
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
