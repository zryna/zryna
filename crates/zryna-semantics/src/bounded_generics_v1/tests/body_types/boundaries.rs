//! Diagnostic ordering, bounded storage, and symbolic phase boundaries.

use super::*;

#[test]
fn nongeneric_type_diagnostics_keep_existing_meaning() {
    assertions::legacy_constructors();
    rejects("function score():i32 {return missing;}", "ZRYNA-M3002", "missing");
    rejects("function score():i32 {return true;}", "ZRYNA-M3007", "true");
    rejects("function score():i32 {return 2147483648;}", "ZRYNA-M3008", "2147483648");
    rejects("function score():i32 { \"x\" === 1; return 0; }", "ZRYNA-M3007", "===");
    let errors = errors("function score(x:FixedArray<Foo,2>):i32 {return 0;}");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-M3002");
    assert_eq!(errors[0].message(), "type 'Foo' does not name a module-local aggregate");
}

#[test]
fn source_order_and_terminal_diagnostics() {
    let source = format!("function score():i32 {{{} return 0;}}", "missing;".repeat(260));
    let errors = errors(&source);
    assert_eq!(errors.len(), 256);
    assert_eq!(errors[255].code(), "ZRYNA-M7201");
    assert!(
        errors.windows(2).all(|pair| pair[0]
            .primary_span()
            .expect("original fixture invariant")
            .start()
            < pair[1].primary_span().expect("original fixture invariant").start())
    );
}

#[test]
fn derived_storage_cache_eviction_and_pristine_replay() {
    assert_eq!(
        resources::comparison_domain(262_144, 65_536, 16_384, 4096, 16_384)
            .expect("original fixture invariant"),
        (7_466_307_588, 55_745_748_998_626_377_744, 5_316_866)
    );
    assert!(matches!(
        resources::reserve::<u64>(usize::MAX),
        Err(BodyTypeFailure::AllocationFailure)
    ));
    let input = project(&[(
        "main.zry",
        &format!(
            "{IDENTITY} function relay<T extends ZrynaValue>(x:Result<T,T>):Result<T,T> {{ return identity<Result<T,T>>(x); }}"
        ),
    )]);
    let context = declarations(&input);
    for capacity in [0, 1, 2, 256] {
        let checked = check_with_cache(&context, capacity).expect("original fixture invariant");
        assert!(checked.storage().table_capacity_bytes > 0);
        assert_eq!(checked.storage().environments, 1);
    }
    let chain = ".next".repeat(32);
    let source = format!(
        "interface Link<T extends ZrynaValue> extends ZrynaStruct {{next:Link<Result<T,T>>;}} function compare(a:Link<i32>,b:Link<i32>):i32 {{let value=a{chain}; value=b{chain}; value=b{chain}; return 0;}}"
    );
    let input = project(&[("main.zry", &source)]);
    let context = declarations(&input);
    for capacity in [0, 1, 2, 256] {
        let checked = check_with_cache(&context, capacity).expect("repeated substitution DAG");
        eprintln!("cache={capacity}, storage={:?}", checked.storage());
    }
    assertions::pristine_replay();
}

#[test]
fn all_v5_variants_have_semantic_rules() {
    assertions::all_variants(
        "interface Pair extends ZrynaStruct {value:i32;} interface Choice extends ZrynaEnum {some:i32;none:ZrynaNone;} function helper(x:i32):i32 {return x;} function score(flag:bool,owner:String,values:Vec<i32>,weak:Weak<String>):i32 {let n:i32 = 1; {const x:i32 = 2;} if(flag){n=2;}else{n=3;}while(flag){n=n+1;}true;false;\"s\";-n;n+n;n-n;n*n;n===1;n!==1;n<1;n<=1;n>1;n>=1;helper(n);clone(owner);shared(owner);downgrade(shared(owner));borrow(owner);borrowMut(values);push(values,1);Pair({value:n});Choice.some(n);Vec<i32>([1]);FixedArray<i32,1>([1]);values[0];upgradeWeak(weak,(strong)=>{clone(strong);},()=>{n=0;});match(Choice.some(n),{\"Choice.some\":(v)=>v,\"Choice.none\":()=>0});return Pair({value:n}).value;}",
    );
    let source = "function score():i32 {return unknown();}";
    rejects(source, "ZRYNA-M3002", "unknown");
}

#[test]
fn no_instance_or_ownership_certificate_is_created() {
    passes(
        "interface Nest<T extends ZrynaValue> extends ZrynaStruct {next:Vec<Nest<Vec<T>>>;} function recurse<T extends ZrynaValue>(x:T):T {return recurse<T>(x);}",
    );
}

#[test]
fn recursive_indirection_clone_keeps_greatest_fixed_point() {
    passes(
        "interface Node<T extends ZrynaValue> extends ZrynaStruct {next:Vec<Node<T>>;value:T;} function copy(x:Node<i32>):Node<i32> {return clone(x);}",
    );
    rejects(
        "interface Node<T extends ZrynaValue> extends ZrynaStruct {next:Vec<Node<T>>;value:T;} function bad<T extends ZrynaValue>(x:Node<T>):Node<T> {return clone(x);}",
        "ZRYNA-M7002",
        "clone",
    );
    rejects(
        "interface Node<T extends ZrynaValue> extends ZrynaStruct {next:Vec<Node<T>>;loan:Borrow<i32>;} function bad(x:Node<i32>):Node<i32> {return clone(x);}",
        "ZRYNA-M3008",
        "clone",
    );
}
