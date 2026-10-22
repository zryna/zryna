//! Independent raw/status/output/physical-owner C oracle; never manufactures Rust results.
use super::*;
const BODY: &str = r#"
static volatile size_t shim_calls;
static void *owners[16];static size_t ended[16],owner_count,releases;
static int32_t observed_before,observed_after,observed_status;
void *__real_malloc(size_t);
void __real_free(void *);
void *__wrap_malloc(size_t n) {
 void *p=__real_malloc(n);assert(p && owner_count<16);
 owners[owner_count]=p;ended[owner_count++]=0;return p;
}
void __wrap_free(void *p) {
 assert(p);size_t i=owner_count;
 while(i && (ended[i-1] || owners[i-1]!=p))--i;
 assert(i && !ended[i-1]);ended[i-1]=++releases;__real_free(p);
}
int32_t __real_sum_bytes(const uint8_t *,size_t,int32_t *);
int32_t __wrap_sum_bytes(const uint8_t *p,size_t n,int32_t *out) {
 ++shim_calls;assert(n==3 && p && out);
 for(size_t i=0;i<3;++i)assert(p[i]==(uint8_t)(i+1));
 observed_before=*out;assert(observed_before==0);
 observed_status=__real_sum_bytes(p,n,out);observed_after=*out;
 assert(observed_status==EXPECTED_STATUS && observed_after==EXPECTED_AFTER);
 for(size_t i=0;i<3;++i)assert(p[i]==(uint8_t)(i+1));
 return observed_status;
}
static void raw_oracle(void) {
 uint8_t small[]={1,2,3},extra[4097];int32_t out=4242;
 assert(__real_sum_bytes(small,3,&out)==EXPECTED_STATUS);
 assert(out==EXPECTED_RAW_AFTER);
 for(size_t i=0;i<3;++i)assert(small[i]==(uint8_t)(i+1));
 for(size_t i=0;i<4097;++i)extra[i]=1;
 out=4242;assert(__real_sum_bytes(extra,4097,&out)==1 && out==4242);
 for(size_t i=0;i<4097;++i)assert(extra[i]==1);
 out=4242;assert(__real_sum_bytes(NULL,1,&out)==2 && out==4242);
 assert(shim_calls==0 && owner_count==0 && releases==0);
}
int main(void) {
 raw_oracle();
 struct zryna_c_v0_context context;zryna_c_v0_context_initialize(&context);
 struct zryna_c_v0_inputs inputs={.count=1};struct zryna_c_v0_outcome outcome;
 zryna_rt_o1_handle input={0};assert(zryna_rt_o1_vec_allocate(1,3,&input)==0);
 input.length=3;int32_t *p=(int32_t *)input.pointer;
 p[0]=1;p[1]=2;p[2]=3;memcpy(&inputs.owned[0],&input,sizeof(input));
 uint32_t fault_tag=ENTRY(&context,&inputs,&outcome);
 /* This is the actual sensitivity assertion, also reached by the nonfaulting control. */
 assert(fault_tag==3);
 assert(outcome.tag==3 && outcome.status==EXPECTED_STATUS && outcome.trap==0);
 assert(outcome.value==0 && outcome.owned.pointer==0 && outcome.owned.length==0);
 assert(outcome.owned.capacity==0 && outcome.unresolved==0 && outcome.reserved==0);
 assert(outcome.padding==0 && shim_calls==1);
 assert(observed_before==0 && observed_after==EXPECTED_AFTER && observed_status==EXPECTED_STATUS);
 assert(context.busy==0 && context.live==0 && context.reserved==0 && context.poisoned==1);
 assert(context.private_unresolved==0);
 for(size_t i=0;i<64;++i) {
  assert(context.owners[i].pointer==0 && context.owners[i].function==0);
  assert(context.owners[i].owner==0 && context.owners[i].release==0 && context.owners[i].state==0);
 }
 /* Input was consumed. Only pointer identity/free counters are read after return. */
 assert(owner_count==2 && releases==2 && ended[1]==1 && ended[0]==2);
 assert(ENTRY(&context,&inputs,&outcome)==3 && shim_calls==1);
 assert(owner_count==2 && releases==2 && context.poisoned==1);
 puts("ordinary-rust-fault:CONTROL_NAME");return 0;
}
"#;
pub(super) fn build(request: &Request<'_>) -> Result<String, Diagnostic> {
    request.check()?;
    let object = request.requirements.object();
    let selected = object
        .program()
        .functions()
        .enumerate()
        .filter(|(_, f)| f.name() == "sum")
        .collect::<Vec<_>>();
    let [(ordinal, function)] = selected.as_slice() else {
        return Err(rejected());
    };
    if object.entries() != [*ordinal]
        || !object.uses_storage_channel()
        || object.imported_operations().map(|o| o.declaration().symbol.as_str()).collect::<Vec<_>>()
            != ["sum_bytes"]
        || object.imported_runtime_operations().next().is_none()
    {
        return Err(rejected());
    }
    let runtime = request.requirements.private_runtime_source().ok_or_else(rejected)?;
    let header = object.program().source().runtime_abi().native_linux_x86_64_header();
    if sha(runtime)
        != request.requirements.private_runtime_source_sha256().copied().ok_or_else(rejected)?
        || sha(header)
            != request.requirements.private_runtime_header_sha256().copied().ok_or_else(rejected)?
    {
        return Err(rejected());
    }
    let mut source = String::from(super::super::clients::PRELUDE);
    source.push_str(std::str::from_utf8(header).map_err(|_| rejected())?);
    source.push_str(object.header());
    let kind = request.hostile.kind();
    source.push_str(
        &BODY
            .replace("ENTRY", &function.entry().symbol)
            .replace("EXPECTED_STATUS", &kind.status().to_string())
            .replace("EXPECTED_RAW_AFTER", if kind == Kind::UnknownStatus { "4242" } else { "6" })
            .replace("EXPECTED_AFTER", &kind.output().to_string())
            .replace("CONTROL_NAME", kind.name()),
    );
    Ok(source)
}
