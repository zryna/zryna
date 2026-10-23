/* All genuine successful calls use the canonical runtime; only clone route faults are injected. */
uint32_t __wrap_zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes,uint64_t length,zryna_rt_o1_handle *out) {
  zero_out(out,"StringFromUtf8");string_creations++;
  if(strcmp(case_name,"generic-replace")==0 && tracked_root) root_check(2,"root-committed");
  inside_runtime++;uint32_t status=__real_zryna_rt_o1_string_from_utf8_copy(bytes,length,out);inside_runtime--;
  if(status==0) {
    assert(out->length==length && memcmp((const void *)out->pointer,bytes,(size_t)length)==0);
    acquire(out,0);
  } else {
    assert(status==1 && out->pointer==0 && out->length==0 && out->capacity==0);
    if(tracked_root) root_check(2,"unchanged-root");
  }
  return status;
}
uint32_t __wrap_zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source,zryna_rt_o1_handle *out) {
  acquired *owner=find(source->pointer);assert(source!=out && source->length==owner->length && source->capacity==owner->bytes);
  zero_out(out,"StringClone");string_clones++;
  zryna_rt_o1_handle before=*source;
  record("clone-entry","String",identity(owner),source->pointer,(uintptr_t)source,0,(const uint8_t *)source->pointer,source->length);
  if(strcmp(fault,"stringClone")==0 && string_clones==fail_ordinal) {
    *out=(zryna_rt_o1_handle){UINTPTR_MAX,UINT64_MAX,UINT64_MAX};
    record("failure","stringClone",string_clones,0,0,source->pointer,NULL,0);
    record("unchanged-source","String",identity(owner),source->pointer,(uintptr_t)source,0,(const uint8_t *)source->pointer,source->length);
    return 1;
  }
  inside_runtime++;uint32_t status=__real_zryna_rt_o1_string_clone(source,out);inside_runtime--;
  assert(memcmp(source,&before,sizeof(before))==0 && owner->live);
  if(status==0) {
    assert(out->pointer!=source->pointer && out->length==source->length && memcmp((const void *)out->pointer,(const void *)source->pointer,(size_t)source->length)==0);
    acquire(out,source->pointer);
  } else {
    assert(status==1 && out->pointer==0 && out->length==0 && out->capacity==0);
    record("unchanged-source","String",identity(owner),source->pointer,(uintptr_t)source,0,(const uint8_t *)source->pointer,source->length);
  }
  return status;
}
uint32_t __wrap_zryna_rt_o1_string_release(const zryna_rt_o1_handle *value) {
  acquired *owner=find(value->pointer);size_t id=identity(owner);
  assert(value->length==owner->length && value->capacity==owner->bytes);
  record("release-entry","String",id,value->pointer,(uintptr_t)value,0,(const uint8_t *)value->pointer,value->length);
  unsigned replacement=strcmp(case_name,"generic-replace")==0 && ((id==1 && successes>=2) || (id==2 && successes>=3));
  if(replacement) {
    if(id==1) tracked_root=value;
    assert(tracked_root==value && owners[id].live);
    record("old-drop-entry","String",id,value->pointer,(uintptr_t)value,owners[id].pointer,NULL,0);
  }
  if(strcmp(case_name,"generic-replace")==0 && id==3) root_check(3,"root-committed");
  uintptr_t previous=value->pointer;
  inside_runtime++;uint32_t status=__real_zryna_rt_o1_string_release(value);inside_runtime--;
  assert(status==0 && !owner->live);
  if(replacement) {
    assert(value->pointer==previous && owners[id].live);
    record("old-drop-complete","String",id,value->pointer,(uintptr_t)value,owners[id].pointer,NULL,0);
  }
  return status;
}
