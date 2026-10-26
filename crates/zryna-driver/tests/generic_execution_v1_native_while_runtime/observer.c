#include "observer.h"
#include "observer_host.h"
#include "observer_strings.h"
static size_t number(const char *text) {
  char *end=NULL;errno=0;unsigned long n=strtoul(text,&end,10);
  assert(!errno && end!=text && !*end && n<=6);return (size_t)n;
}
#if M416_ENTRY_ARGUMENTS == 1
static int32_t argument(const char *text) {
  char *end=NULL;errno=0;long long n=strtoll(text,&end,10);
  assert(!errno && end!=text && !*end && n>=INT32_MIN && n<=INT32_MAX);
#if M416_ENTRY_BOOL
  assert(n==0 || n==1);
#endif
  return (int32_t)n;
}
#endif
int main(int argc,char **argv) {
  assert(argc==6);case_name=argv[1];fault=argv[2];fail_ordinal=number(argv[3]);
  assert(strcmp(case_name,"copy-assignment")==0 || strcmp(case_name,"zero-multiple")==0 || strcmp(case_name,"nested-lexical")==0 || strcmp(case_name,"early-return")==0 || strcmp(case_name,"body-loan-cleanup")==0 || strcmp(case_name,"opaque-retained")==0 || strcmp(case_name,"owned-root-replacement")==0 || strcmp(case_name,"branch-copy-update")==0 || strcmp(case_name,"match-condition")==0);
  assert(strcmp(fault,"none")==0 || strcmp(fault,"allocation")==0 || strcmp(fault,"stringClone")==0);
  assert((strcmp(fault,"none")==0)==(fail_ordinal==0));
  const struct rlimit no_core={0,0};assert(setrlimit(RLIMIT_CORE,&no_core)==0);
  assert(setvbuf(stdout,NULL,_IONBF,0)==0);trace=fopen(argv[4],"wx");assert(trace && setvbuf(trace,NULL,_IONBF,0)==0);
#if M416_ENTRY_ARGUMENTS == 0
  assert(strcmp(argv[5],"none")==0);
  assert(fputs("{\"event\":\"entry\",\"arguments\":[],\"boolean\":0}\n",trace)>=0);
  int32_t result=zryna_v1_e_entry();
#else
  int32_t input=argument(argv[5]);
  assert(fprintf(trace,"{\"event\":\"entry\",\"arguments\":[%d],\"boolean\":%u}\n",input,(unsigned)M416_ENTRY_BOOL)>0);
  int32_t result=zryna_v1_e_entry(input);
#endif
  assert(strcmp(fault,"none")==0 && !inside_runtime);
  for(size_t i=0;i<successes;i++) assert(!owners[i].live && !blocks[i].live && blocks[i].owner==i+1);
  assert(successes==host_successes);
  assert(fprintf(trace,"{\"event\":\"done\",\"allocations\":%zu,\"attempts\":%zu,\"clones\":%zu,\"concats\":%zu,\"zero_checks\":%zu}\n",successes,attempts,string_clones,string_concats,zero_checks)>0);
  assert(fclose(trace)==0);printf("result:%d\n",result);return 0;
}
