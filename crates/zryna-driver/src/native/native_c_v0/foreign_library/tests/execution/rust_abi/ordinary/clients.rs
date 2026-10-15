//! Independent C assertions and selected-allocation ledger; no global leak/retention proof.
use super::*;
pub(super) const PRELUDE: &str = "#include <assert.h>\n#include <limits.h>\n#include <stddef.h>\n#include <stdint.h>\n#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n";
const SCALAR: &str = r#"
static volatile size_t shim_calls;
int32_t __real_add(int32_t,int32_t);
int32_t __wrap_add(int32_t left,int32_t right) {
 ++shim_calls;
 return __real_add(left,right);
}
static void scalar_clean(const struct zryna_c_v0_context *context,
                         const struct zryna_c_v0_outcome *outcome) {
 assert(outcome->tag==0 && outcome->unresolved==0 && outcome->reserved==0 && outcome->padding==0);
 assert(context->busy==0 && context->live==0 && context->reserved==0 && context->poisoned==0);
 for(size_t i=0;i<64;++i) {
  assert(context->owners[i].pointer==0 && context->owners[i].function==0);
  assert(context->owners[i].owner==0 && context->owners[i].release==0 && context->owners[i].state==0);
 }
}
int main(void) {
 for(size_t repeat=0;repeat<3;++repeat) {
  struct zryna_c_v0_context context; zryna_c_v0_context_initialize(&context);
  struct zryna_c_v0_inputs inputs={.count=2,.values={20,22}};
  struct zryna_c_v0_outcome outcome;
  shim_calls=0;
  assert(ENTRY(&context,&inputs,&outcome)==0 && outcome.tag==0 && outcome.value==42);
  assert(shim_calls==1);scalar_clean(&context,&outcome);
  inputs.values[0]=INT32_MAX;inputs.values[1]=1;shim_calls=0;
  assert(ENTRY(&context,&inputs,&outcome)==0 && outcome.value==INT32_MIN);
  assert(shim_calls==1);scalar_clean(&context,&outcome);
  inputs.values[0]=(uint32_t)INT32_MIN;inputs.values[1]=(uint32_t)-1;shim_calls=0;
  assert(ENTRY(&context,&inputs,&outcome)==0 && outcome.value==INT32_MAX);
  assert(shim_calls==1);scalar_clean(&context,&outcome);
 }
 puts("ordinary-rust-scalar:9calls");return 0;
}
"#;
const BYTES: &str = r#"
static volatile size_t shim_calls;
static void *owners[16];
static size_t ended[16],owner_count,attempts,fail_at,releases;
static size_t expected_length;static int32_t *input_values;
void *__real_malloc(size_t);
void __real_free(void *);
void *__wrap_malloc(size_t n) {
 ++attempts;if(fail_at && attempts==fail_at)return NULL;
 void *p=__real_malloc(n);assert(p && owner_count<16);
 owners[owner_count]=p;ended[owner_count++]=0;return p;
}
void __wrap_free(void *p) {
 assert(p);size_t i=owner_count;
 while(i && (ended[i-1] || owners[i-1]!=p))--i;
 assert(i && !ended[i-1]);ended[i-1]=++releases;__real_free(p);
}
static void clear_observed(void) {
 assert(releases==owner_count);owner_count=attempts=fail_at=releases=shim_calls=0;
}
int32_t __real_sum_bytes(const uint8_t *,size_t,int32_t *);
int32_t __wrap_sum_bytes(const uint8_t *p,size_t n,int32_t *out) {
 ++shim_calls;assert(n==expected_length && out);
 assert((n==0 && p==NULL) || (n>0 && p));
 for(size_t i=0;i<n;++i)assert(p[i]==(uint8_t)input_values[i]);
 int32_t status=__real_sum_bytes(p,n,out);
 assert(status==0);int32_t sum=0;
 for(size_t i=0;i<n;++i){assert(p[i]==(uint8_t)input_values[i]);sum+=input_values[i];}
 assert(*out==sum);return status;
}
int main(void) {
 const size_t lengths[]={0,3,4096,4097,1,1,3,3};
 for(size_t test=0;test<8;++test) {
  clear_observed();size_t length=lengths[test];
  struct zryna_c_v0_context context;zryna_c_v0_context_initialize(&context);
  struct zryna_c_v0_inputs inputs={.count=1};struct zryna_c_v0_outcome outcome;
  zryna_rt_o1_handle input={0};assert(zryna_rt_o1_vec_allocate(1,length?length:1,&input)==0);
  input_values=(int32_t *)input.pointer;input.length=length;expected_length=length;
  for(size_t i=0;i<length;++i)input_values[i]=length==4096?255:(int32_t)(i+1);
  if(test==4){input_values[0]=-1;}
  if(test==5){input_values[0]=256;}
  memcpy(&inputs.owned[0],&input,sizeof(input));
  if(test==6)fail_at=attempts+1;
  uint32_t tag=ENTRY(&context,&inputs,&outcome);
  if(test<3 || test==7) {
   int32_t sum=length==0?0:length==4096?INT32_C(1044480):INT32_C(6);
   assert(tag==0 && outcome.tag==0 && outcome.value==sum && shim_calls==1);
   assert(owner_count==(length?2U:1U));
  } else {
   assert(tag==2 && outcome.tag==2 && outcome.value==0 && shim_calls==0);
   assert(outcome.trap==(test==3?2U:test==6?4U:3U));assert(owner_count==1);
  }
  /* Input is consumed by the generated entry; never dereference it after return. */
  assert(outcome.unresolved==0 && outcome.owned.pointer==0);
  assert(context.private_unresolved==0 && context.live==0 && context.reserved==0 && context.poisoned==0);
  assert(releases==owner_count);
 }
 clear_observed();puts("ordinary-rust-bytes:4entered4prechecks");return 0;
}
"#;
pub(super) fn import(request: &ImportRequest<'_>, mode: Mode) -> Result<String, Diagnostic> {
    let (name, foreign, storage) = match mode {
        Mode::Scalar => ("imported", "add", false),
        Mode::Bytes { .. } => ("sum", "sum_bytes", true),
        Mode::Reverse => return Err(rejected()),
    };
    let object = request.requirements.object();
    let selected = object
        .program()
        .functions()
        .enumerate()
        .filter(|(_, f)| f.name() == name)
        .collect::<Vec<_>>();
    let [(ordinal, function)] = selected.as_slice() else {
        return Err(rejected());
    };
    if object.entries() != [*ordinal]
        || object.uses_storage_channel() != storage
        || request.requirements.private_runtime_source().is_some() != storage
        || object.imported_runtime_operations().next().is_some() != storage
        || object.imported_operations().map(|o| o.declaration().symbol.as_str()).collect::<Vec<_>>()
            != [foreign]
    {
        return Err(rejected());
    }
    let entry = function.entry().symbol.clone();
    let mut source = String::from(PRELUDE);
    if let Some(runtime) = request.requirements.private_runtime_source() {
        if sha(runtime)
            != request.requirements.private_runtime_source_sha256().copied().ok_or_else(rejected)?
        {
            return Err(rejected());
        }
        let header = request
            .requirements
            .object()
            .program()
            .source()
            .runtime_abi()
            .native_linux_x86_64_header();
        if sha(header)
            != request.requirements.private_runtime_header_sha256().copied().ok_or_else(rejected)?
        {
            return Err(rejected());
        }
        source.push_str(std::str::from_utf8(header).map_err(|_| rejected())?);
    }
    source.push_str(request.requirements.object().header());
    let body = if mode == Mode::Scalar { SCALAR } else { BYTES };
    let body = if mode == (Mode::Bytes { missing_increment: true }) {
        if body.matches("++shim_calls;").count() != 1 {
            return Err(rejected());
        }
        body.replace("++shim_calls;", "(void)shim_calls;")
    } else {
        body.to_owned()
    };
    source.push_str(&body.replace("ENTRY", &entry));
    Ok(source)
}
pub(super) fn reverse() -> &'static str {
    include_str!("../../../../../../../../../../tests/native-c-rust-v0/reverse-client.c")
}
