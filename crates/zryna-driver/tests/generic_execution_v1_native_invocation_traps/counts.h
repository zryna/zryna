/* Actual canonical count prefix and transitions; no natural u32MAX-overflow claim. */
static uint32_t count_acquire(uintptr_t control,unsigned role) {
  block *owner=find(control,3);uint32_t *counts=(uint32_t *)control;
  uint32_t strong=counts[0],weak=counts[1];assert(strong && weak);enter(role);uint32_t status;
  if(fault_code==3 && role==12) {faults++;status=3;record(6,role,1,status,0,0,0,0);}
  else status=role==10?__real_zryna_rt_o1_strong_clone(control):role==11?__real_zryna_rt_o1_weak_downgrade(control):__real_zryna_rt_o1_weak_clone(control);
  assert(status==(fault_code==3 && role==12?3U:0U));
  assert(counts[0]==strong+(!status && role==10) && counts[1]==weak+(!status && role!=10));
  record(7,role,strong,weak,counts[0],counts[1],status,owner->owner);return leave(role,status);
}
uint32_t __wrap_zryna_rt_o1_strong_clone(uintptr_t c) {return count_acquire(c,10);}
uint32_t __wrap_zryna_rt_o1_weak_downgrade(uintptr_t c) {return count_acquire(c,11);}
uint32_t __wrap_zryna_rt_o1_weak_clone(uintptr_t c) {return count_acquire(c,12);}
uint32_t __wrap_zryna_rt_o1_strong_release_begin(uintptr_t c,uint32_t *last) {
  block *owner=find(c,3);uint32_t *counts=(uint32_t *)c;uint32_t s=counts[0],w=counts[1];assert(s && w && last && !*last);enter(14);
  uint32_t status=__real_zryna_rt_o1_strong_release_begin(c,last);assert(!status && *last==(s==1) && counts[0]==s-1 && counts[1]==w);
  record(7,14,s,w,counts[0],counts[1],status,owner->owner);return leave(14,status);
}
uint32_t __wrap_zryna_rt_o1_strong_release_finish(uintptr_t c) {
  block *owner=find(c,3);uint32_t *counts=(uint32_t *)c;assert(!counts[0] && counts[1]==1);enter(15);
  uint32_t status=__real_zryna_rt_o1_strong_release_finish(c);assert(!status && !owner->live);return leave(15,status);
}
uint32_t __wrap_zryna_rt_o1_weak_release(uintptr_t c,uint32_t *gone) {
  block *owner=find(c,3);uint32_t *counts=(uint32_t *)c;uint32_t s=counts[0],w=counts[1];assert(s && w>1 && gone && !*gone);enter(16);
  uint32_t status=__real_zryna_rt_o1_weak_release(c,gone);assert(!status && !*gone && counts[0]==s && counts[1]==w-1 && owner->live);
  record(7,16,s,w,counts[0],counts[1],status,owner->owner);return leave(16,status);
}
uint32_t __wrap_zryna_rt_o1_weak_upgrade(uintptr_t c) {(void)c;abort();}
