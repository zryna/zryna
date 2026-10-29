#if defined(__linux__)
#include <sys/prctl.h>
#endif
#include <assert.h>
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>
extern int32_t zryna_v1_e_entry(int32_t);
int main(int argc,char **argv) {
  assert(argc==2);char *end=NULL;errno=0;long depth=strtol(argv[1],&end,10);
  assert(!errno && end!=argv[1] && !*end && depth>=0 && depth<=65536);
  const struct rlimit core={0,0};assert(setrlimit(RLIMIT_CORE,&core)==0);
#if defined(__linux__)
  /* Piped crash collectors ignore RLIMIT_CORE; suppress dumps in this child. */
  assert(prctl(PR_SET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  assert(prctl(PR_GET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
#endif
  for(unsigned i=0;i<2;i++) {
    int32_t result=zryna_v1_e_entry((int32_t)depth);
    printf("result:%d\n",result);assert(result==7);
  }
  return 0;
}
