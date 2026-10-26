/* Actual control prefix only; cached pre-free predicates prevent reading released storage. */
static uint32_t acquisition(uintptr_t pointer,unsigned role) {
  (void)known(pointer,3);uint32_t *counts=(uint32_t *)pointer;
  uint32_t strong=counts[0],weak=counts[1];assert(strong && weak);
  unsigned previous=enter(role);uint32_t status=role==10 ? __real_zryna_rt_o1_strong_clone(pointer) :
      role==11 ? __real_zryna_rt_o1_weak_downgrade(pointer) : __real_zryna_rt_o1_weak_clone(pointer);
  assert(status==0 && counts[0]==strong+(role==10) && counts[1]==weak+(role!=10));
  count_record(role==10 ? "strongClone" : role==11 ? "weakDowngrade" : "weakClone",pointer,strong,weak,counts[0],counts[1],status);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_strong_clone(uintptr_t p) { return acquisition(p,10); }
uint32_t __wrap_zryna_rt_o1_weak_downgrade(uintptr_t p) { return acquisition(p,11); }
uint32_t __wrap_zryna_rt_o1_weak_clone(uintptr_t p) { return acquisition(p,12); }
uint32_t __wrap_zryna_rt_o1_strong_release_begin(uintptr_t pointer,uint32_t *last) {
  (void)known(pointer,3);uint32_t *counts=(uint32_t *)pointer;
  uint32_t strong=counts[0],weak=counts[1];assert(strong && weak && last && !*last);
  unsigned previous=enter(14);uint32_t status=__real_zryna_rt_o1_strong_release_begin(pointer,last);
  assert(status==0 && *last==(strong==1) && counts[0]==strong-1 && counts[1]==weak);
  if(*last) {assert(!pending_last);pending_last=pointer;}
  count_record("strongReleaseBegin",pointer,strong,weak,counts[0],counts[1],status);event(6,14,*last,status);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_strong_release_finish(uintptr_t pointer) {
  uint64_t id=known(pointer,3);uint32_t *counts=(uint32_t *)pointer;
  uint32_t strong=counts[0],weak=counts[1];assert(!strong && weak && pending_last==pointer);
  unsigned previous=enter(15);uint32_t status=__real_zryna_rt_o1_strong_release_finish(pointer);assert(status==0);
  if(weak==1)assert(!physical_live(id));else assert(physical_live(id) && !counts[0] && counts[1]==weak-1);
  pending_last=0;count_record("strongReleaseFinish",pointer,strong,weak,0,weak-1,status);
  leave(previous,status);return status;
}
uint32_t __wrap_zryna_rt_o1_weak_release(uintptr_t pointer,uint32_t *deallocated) {
  uint64_t id=known(pointer,3);uint32_t *counts=(uint32_t *)pointer;
  uint32_t strong=counts[0],weak=counts[1];assert(weak && deallocated && !*deallocated);
  unsigned previous=enter(16);uint32_t status=__real_zryna_rt_o1_weak_release(pointer,deallocated);
  assert(status==0 && *deallocated==(weak==1 && !strong));
  if(*deallocated)assert(!physical_live(id));else assert(physical_live(id) && counts[0]==strong && counts[1]==weak-1);
  count_record("weakRelease",pointer,strong,weak,strong,weak-1,status);event(6,16,*deallocated,status);
  leave(previous,status);return status;
}
