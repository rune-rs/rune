use std::collections::HashMap;

prelude!();

#[test]
fn dynamic_fields_never() -> Result<()> {
    #[derive(Any, Clone)]
    struct TestClass {
        #[rune(get, set)]
        a: i64,
    }
    let mut context = Context::with_default_modules()?;
    let mut module = Module::new();
    module.ty::<TestClass>()?;
    context.install(module)?;

    let runtime = Arc::new(context.runtime()?);

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "script",
        r#"
        pub fn get_meta_field(value) {
            value.b
        }

        pub fn get_field(value) {
            value.a
        }
        "#,
    )?)?;

    let result = prepare(&mut sources).with_context(&context).build();

    let unit = result?;
    let mut vm = Vm::new(runtime, Arc::new(unit));

    vm.set_dynamic_fields(true);

    let input = TestClass { a: 42 };

    let value = i64::from_value(vm.call(["get_field"], (input.clone(),))?).into_result()?;
    assert_eq!(value, 42);

    let value = vm.call(["get_meta_field"], (input,));
    assert!(value.is_err());

    Ok(())
}

#[test]
fn dynamic_fields_first() -> Result<()> {
    #[derive(Any, Clone)]
    #[rune(meta_fields = first)]
    struct TestClass {
        #[rune(get, set)]
        a: i64,
        values: HashMap<String, i64>,
    }
    let mut context = Context::with_default_modules()?;
    let mut module = Module::new();
    module.ty::<TestClass>()?;
    module.inst_fn(
        Protocol::DYNAMIC_FIELD_GET,
        |this: &TestClass, val: &str| this.values.get(val).copied(),
    )?;
    context.install(module)?;

    let runtime = Arc::new(context.runtime()?);

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "script",
        r#"
        pub fn get_meta_field(value) {
            value.b
        }

        pub fn get_field(value) {
            value.a
        }

        pub fn set_meta_field(value, into) {
            value.b = into;
            value.b
        }
        "#,
    )?)?;

    let result = prepare(&mut sources).with_context(&context).build();

    let unit = result?;
    let mut vm = Vm::new(runtime, Arc::new(unit));
    vm.set_dynamic_fields(true);

    let input = TestClass {
        a: 69,
        values: {
            let mut map = HashMap::with_capacity(1);
            map.insert("b".into(), 42);
            map
        },
    };

    let value = i64::from_value(vm.call(["get_meta_field"], (input.clone(),))?).into_result()?;
    assert_eq!(value, 42);

    let value = i64::from_value(vm.call(["get_field"], (input,))?).into_result()?;
    assert_eq!(value, 69);
    Ok(())
}

#[test]
fn dynamic_fields_last() -> Result<()> {
    #[derive(Any, Clone)]
    #[rune(meta_fields = last)]
    struct TestClass {
        #[rune(get, set)]
        a: i64,
        values: HashMap<String, i64>,
    }
    let mut context = Context::with_default_modules()?;
    let mut module = Module::new();
    module.ty::<TestClass>()?;
    module.inst_fn(
        Protocol::DYNAMIC_FIELD_GET,
        |this: &TestClass, val: &str| this.values.get(val).copied(),
    )?;
    module.inst_fn(
        Protocol::DYNAMIC_FIELD_SET,
        |this: &mut TestClass, key: &str, value: i64| {
            this.values.insert(key.into(), value);
            Some(())
        },
    )?;
    context.install(module)?;

    let runtime = Arc::new(context.runtime()?);

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "script",
        r#"
        pub fn get_meta_field(value) {
            value.b
        }

        pub fn get_field(value) {
            value.a
        }

        pub fn set_meta_field(value, into) {
            value.b = into;
            value.b
        }
        "#,
    )?)?;

    let result = prepare(&mut sources).with_context(&context).build();

    let unit = result?;
    let mut vm = Vm::new(runtime, Arc::new(unit));
    vm.set_dynamic_fields(true);

    let input = TestClass {
        a: 69,
        values: {
            let mut map = HashMap::with_capacity(1);
            map.insert("b".into(), 42);
            map
        },
    };

    let value = i64::from_value(vm.call(["get_meta_field"], (input.clone(),))?).into_result()?;
    assert_eq!(value, 42);

    let value =
        i64::from_value(vm.call(["set_meta_field"], (input.clone(), 1337))?).into_result()?;
    assert_eq!(value, 1337);

    let value = i64::from_value(vm.call(["get_field"], (input,))?).into_result()?;
    assert_eq!(value, 69);
    Ok(())
}

#[test]
fn dynamic_fields_only() -> Result<()> {
    #[derive(Any, Clone)]
    #[rune(meta_fields = only)]
    struct TestClass {
        #[rune(get, set)]
        a: i64,
        values: HashMap<String, i64>,
    }
    let mut context = Context::with_default_modules()?;
    let mut module = Module::new();
    module.ty::<TestClass>()?;
    module.inst_fn(
        Protocol::DYNAMIC_FIELD_GET,
        |this: &TestClass, val: &str| this.values.get(val).copied(),
    )?;
    context.install(module)?;

    let runtime = Arc::new(context.runtime()?);

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "script",
        r#"
        pub fn get_meta_field(value) {
            value.b
        }

        pub fn get_field(value) {
            value.a
        }

        pub fn set_meta_field(value, into) {
            value.b = into;
            value.b
        }
        "#,
    )?)?;

    let result = prepare(&mut sources).with_context(&context).build();

    let unit = result?;
    let mut vm = Vm::new(runtime, Arc::new(unit));
    vm.set_dynamic_fields(true);

    let input = TestClass {
        a: 69,
        values: {
            let mut map = HashMap::with_capacity(1);
            map.insert("b".into(), 42);
            map
        },
    };

    let value = i64::from_value(vm.call(["get_meta_field"], (input.clone(),))?).into_result()?;
    assert_eq!(value, 42);

    let value = vm.call(["get_field"], (input,));
    assert!(value.is_err());
    Ok(())
}
