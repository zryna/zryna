#if defined(__linux__)
#include <sys/prctl.h>
#endif
#include "observer.h"
#include "observer_host.h"
#include "observer_strings.h"
static size_t number(const char *text) {
  char *end=NULL;errno=0;unsigned long n=strtoul(text,&end,10);
  assert(!errno && end!=text && !*end && n<=4);return (size_t)n;
}
int main(int argc,char **argv) {
  assert(argc==5);case_name=argv[1];fault=argv[2];fail_ordinal=number(argv[3]);
  assert(strcmp(case_name,"generic-replace")==0 || strcmp(case_name,"static-disjoint")==0 || strcmp(case_name,"fallible-cleanup")==0);
  assert(strcmp(fault,"none")==0 || strcmp(fault,"allocation")==0 || strcmp(fault,"stringClone")==0);
  assert((strcmp(fault,"none")==0)==(fail_ordinal==0));
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
#if defined(__linux__)
  /* Piped crash collectors ignore RLIMIT_CORE; suppress dumps in this child. */
  assert(prctl(PR_SET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
  assert(prctl(PR_GET_DUMPABLE,0UL,0UL,0UL,0UL)==0);
#endif
  assert(setvbuf(stdout,NULL,_IONBF,0)==0);trace=fopen(argv[4],"wx");assert(trace && setvbuf(trace,NULL,_IONBF,0)==0);
  int32_t result=zryna_v1_e_entry();assert(strcmp(fault,"none")==0 && !inside_runtime);
  for(size_t i=0;i<successes;i++) assert(!owners[i].live && !blocks[i].live && blocks[i].owner==i+1);
  assert(successes==host_successes);
  assert(fprintf(trace,"{\"event\":\"done\",\"allocations\":%zu,\"attempts\":%zu,\"clones\":%zu,\"zero_checks\":%zu}\n",successes,attempts,string_clones,zero_checks)>0);
  assert(fclose(trace)==0);printf("result:%d\n",result);return 0;
}
