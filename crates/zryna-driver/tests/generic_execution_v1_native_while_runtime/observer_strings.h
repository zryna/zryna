/* Canonical runtime owns actual UTF8 payload allocation; route faults precede canonical calls. */
static void unchanged(const zryna_rt_o1_handle *source,const zryna_rt_o1_handle *before) {
  acquired *owner=find(source->pointer);
  assert(memcmp(source,before,sizeof(*before))==0 && source->length==owner->length && source->capacity==owner->bytes);
  record("unchanged-source","String",identity(owner),source->pointer,(uintptr_t)source,0,(const uint8_t *)source->pointer,source->length);
}
static void poison(zryna_rt_o1_handle *out) {
  *out=(zryna_rt_o1_handle){UINTPTR_MAX,UINT64_MAX,UINT64_MAX};
}
uint32_t __wrap_zryna_rt_o1_string_from_utf8_copy(const uint8_t *bytes,uint64_t length,zryna_rt_o1_handle *out) {
  zero_out(out,"StringFromUtf8");
  inside_runtime++;uint32_t status=__real_zryna_rt_o1_string_from_utf8_copy(bytes,length,out);inside_runtime--;
  if(status==0) {
    assert(out->length==length && memcmp((const void *)out->pointer,bytes,(size_t)length)==0);
    acquire(out,0);
  } else { assert(status==1 && out->pointer==0 && out->length==0 && out->capacity==0); }
  return status;
}
uint32_t __wrap_zryna_rt_o1_string_clone(const zryna_rt_o1_handle *source,zryna_rt_o1_handle *out) {
  acquired *owner=find(source->pointer);assert(source!=out && source->length==owner->length && source->capacity==owner->bytes);
  zero_out(out,"StringClone");string_clones++;
  zryna_rt_o1_handle before=*source;
  record("clone-entry","String",identity(owner),source->pointer,(uintptr_t)source,0,(const uint8_t *)source->pointer,source->length);
  if(strcmp(fault,"stringClone")==0 && string_clones==fail_ordinal) {
    poison(out);record("failure","stringClone",string_clones,0,0,source->pointer,NULL,0);
    unchanged(source,&before);return 1;
  }
  inside_runtime++;uint32_t status=__real_zryna_rt_o1_string_clone(source,out);inside_runtime--;
  assert(memcmp(source,&before,sizeof(before))==0 && owner->live);
  if(status==0) {
    assert(out->pointer!=source->pointer && out->length==source->length && memcmp((const void *)out->pointer,(const void *)source->pointer,(size_t)source->length)==0);
    acquire(out,source->pointer);
  } else {
    assert(status==1 && out->pointer==0 && out->length==0 && out->capacity==0);
    unchanged(source,&before);
  }
  return status;
}
uint32_t __wrap_zryna_rt_o1_string_concat(const zryna_rt_o1_handle *left,const zryna_rt_o1_handle *right,zryna_rt_o1_handle *out) {
  acquired *a=find(left->pointer),*b=find(right->pointer);
  assert(out!=left && out!=right && left->length==a->length && right->length==b->length && left->capacity==a->bytes && right->capacity==b->bytes);
  zero_out(out,"StringConcat");string_concats++;
  zryna_rt_o1_handle before_left=*left,before_right=*right;
  record("concat-left","String",identity(a),left->pointer,(uintptr_t)left,0,(const uint8_t *)left->pointer,left->length);
  record("concat-right","String",identity(b),right->pointer,(uintptr_t)right,0,(const uint8_t *)right->pointer,right->length);
  if(strcmp(fault,"stringConcat")==0 && string_concats==fail_ordinal) {
    poison(out);record("failure","stringConcat",string_concats,0,0,0,NULL,0);
    unchanged(left,&before_left);unchanged(right,&before_right);return 1;
  }
  inside_runtime++;uint32_t status=__real_zryna_rt_o1_string_concat(left,right,out);inside_runtime--;
  assert(memcmp(left,&before_left,sizeof(before_left))==0 && memcmp(right,&before_right,sizeof(before_right))==0 && a->live && b->live);
  if(status==0) {
    assert(out->pointer!=left->pointer && out->pointer!=right->pointer && out->length==left->length+right->length);
    assert(memcmp((const void *)out->pointer,(const void *)left->pointer,(size_t)left->length)==0);
    assert(memcmp((const void *)(out->pointer+left->length),(const void *)right->pointer,(size_t)right->length)==0);
    right_source=right->pointer;acquire(out,left->pointer);right_source=0;
  } else {
    assert(status==1 && out->pointer==0 && out->length==0 && out->capacity==0);
    unchanged(left,&before_left);unchanged(right,&before_right);
  }
  return status;
}
uint32_t __wrap_zryna_rt_o1_string_release(const zryna_rt_o1_handle *value) {
  acquired *owner=find(value->pointer);assert(value->length==owner->length && value->capacity==owner->bytes);
  record("release-entry","String",identity(owner),value->pointer,(uintptr_t)value,0,(const uint8_t *)value->pointer,value->length);
  inside_runtime++;uint32_t status=__real_zryna_rt_o1_string_release(value);inside_runtime--;
  assert(status==0 && !owner->live);return status;
}
