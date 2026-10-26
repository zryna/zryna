#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <sys/resource.h>
extern int32_t zryna_v1_e_entry(void);
int main(void) {
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
  for(unsigned i=0;i<2;i++) { int32_t value=zryna_v1_e_entry();assert(value==97);printf("result:%d\n",value); }
  return 0;
}
