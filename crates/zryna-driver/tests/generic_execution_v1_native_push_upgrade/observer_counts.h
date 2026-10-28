/* Read the actual runtime control prefix; no synthetic count-owner state replaces it. */
static uint32_t acquisition(uintptr_t control, unsigned which) {
  (void)find(control,3); uint32_t *counts=(uint32_t *)control;
  uint32_t strong=counts[0],weak=counts[1]; assert(strong>0 && weak>0);
  const char *faults[]={"strong-clone-status","weak-downgrade-status","weak-clone-status"};
  const char *names[]={"strongClone","weakDowngrade","weakClone"};
  const char *failed[]={"strongClone:failure","weakDowngrade:failure","weakClone:failure"};
  count_attempts[which]++;
  if(strcmp(fault,faults[which])==0 && count_attempts[which]==fail_ordinal) {
    count_event(failed[which],control,strong,weak,counts[0],counts[1],3);
    record("failure",fault,count_attempts[which],0,control,NULL,0); return 3;
  }
  uint32_t status=which==0 ? __real_zryna_rt_o1_strong_clone(control) : which==1 ? __real_zryna_rt_o1_weak_downgrade(control) : __real_zryna_rt_o1_weak_clone(control);
  assert(status==0); assert(counts[0]==strong+(which==0) && counts[1]==weak+(which!=0));
  count_event(names[which],control,strong,weak,counts[0],counts[1],status); return status;
}
uint32_t __wrap_zryna_rt_o1_strong_clone(uintptr_t c) { return acquisition(c,0); }
uint32_t __wrap_zryna_rt_o1_weak_downgrade(uintptr_t c) { return acquisition(c,1); }
uint32_t __wrap_zryna_rt_o1_weak_clone(uintptr_t c) { return acquisition(c,2); }
uint32_t __wrap_zryna_rt_o1_strong_release_begin(uintptr_t control, uint32_t *last) {
  (void)find(control,3); uint32_t *counts=(uint32_t *)control;
  uint32_t strong=counts[0],weak=counts[1]; assert(strong>0 && weak>0 && last && *last==0); *last=UINT32_MAX;
  uint32_t status=__real_zryna_rt_o1_strong_release_begin(control,last); assert(status==0 && *last==(strong==1));
  assert(counts[0]==strong-1 && counts[1]==weak);
  count_event(*last ? "strongReleaseBegin:last" : "strongReleaseBegin:notLast",control,strong,weak,counts[0],counts[1],status);
  if(*last) { assert(!pending_last); pending_last=control; } return status;
}
uint32_t __wrap_zryna_rt_o1_strong_release_finish(uintptr_t control) {
  acquired *owner=find(control,3); uint32_t *counts=(uint32_t *)control;
  uint32_t strong=counts[0],weak=counts[1]; assert(strong==0 && weak>0 && pending_last==control);
  uint32_t status=__real_zryna_rt_o1_strong_release_finish(control); assert(status==0);
  count_event(weak==1 ? "strongReleaseFinish:deallocated" : "strongReleaseFinish:retained",control,strong,weak,0,weak-1,status);
  if(weak==1) assert(!owner->live && owner->released==1); else assert(counts[0]==0 && counts[1]==weak-1);
  pending_last=0; return status;
}
uint32_t __wrap_zryna_rt_o1_weak_release(uintptr_t control, uint32_t *deallocated) {
  acquired *owner=find(control,3); uint32_t *counts=(uint32_t *)control;
  uint32_t strong=counts[0],weak=counts[1]; assert(weak>0 && deallocated && *deallocated==0); *deallocated=UINT32_MAX;
  uint32_t status=__real_zryna_rt_o1_weak_release(control,deallocated); assert(status==0 && *deallocated==(weak==1));
  count_event("weakRelease",control,strong,weak,strong,weak-1,status);
  if(*deallocated) assert(!owner->live && owner->released==1); else assert(counts[0]==strong && counts[1]==weak-1);
  return status;
}
uint32_t __wrap_zryna_rt_o1_weak_upgrade(uintptr_t control) {
  (void)find(control,3);uint32_t *counts=(uint32_t *)control;uint32_t strong=counts[0],weak=counts[1];assert(weak>0);upgrade_calls++;
  if(strcmp(fault,"weak-upgrade-status")==0 && upgrade_calls==fail_ordinal) {
    count_event("weakUpgrade:failure",control,strong,weak,counts[0],counts[1],3);record("failure",fault,upgrade_calls,0,control,NULL,0);return 3;
  }
  uint32_t status=__real_zryna_rt_o1_weak_upgrade(control);assert(status==(strong ? 0U : 5U));
  assert(counts[0]==strong+(status==0) && counts[1]==weak);
  count_event(status==0 ? "weakUpgrade:live" : "weakUpgrade:expired",control,strong,weak,counts[0],counts[1],status);return status;
}
