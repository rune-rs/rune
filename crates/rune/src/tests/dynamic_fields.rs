use std::collections::HashMap;

prelude!();

const TEST_SCRIPT: &str = r#"
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
        "#;

macro_rules! set_up_vm {
    () => {{
        let mut context = Context::with_default_modules()?;
        let mut module = Module::new();
        module.ty::<TestClass>()?;
        module.function_meta(TestClass::get_meta_field)?;
        module.function_meta(TestClass::set_meta_field)?;
        context.install(module)?;

        let runtime = Arc::new(context.runtime()?);

        let mut sources = Sources::new();
        sources.insert(Source::new("script", TEST_SCRIPT)?)?;

        let result = prepare(&mut sources).with_context(&context).build();

        let unit = result?;
        let mut vm = Vm::new(runtime, Arc::new(unit));
        vm.set_dynamic_fields(true);
        vm
    }};
}

macro_rules! register_type {
    ($mode:ident) => {
        #[derive(Any, Clone)]
        #[rune(meta_fields = $mode)]
        struct TestClass {
            #[rune(get, set)]
            a: i64,
            values: HashMap<String, i64>,
        }
        impl TestClass {
            #[rune::function(instance, protocol = DYNAMIC_FIELD_GET)]
            fn get_meta_field(&self, key: &str) -> Option<i64> {
                self.values.get(key).copied()
            }
            #[rune::function(instance, protocol = DYNAMIC_FIELD_SET)]
            fn set_meta_field(&mut self, key: String, val: i64) {
                use std::collections::hash_map::Entry;
                match self.values.entry(key) {
                    Entry::Occupied(entry) => {
                        *entry.into_mut() = val;
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(val);
                    }
                };
            }
        }
    };
}

#[test]
fn dynamic_fields_never() -> Result<()> {
    register_type!(never);
    let mut vm = set_up_vm!();

    let input = TestClass {
        a: 42,
        values: {
            let mut map = HashMap::with_capacity(1);
            map.insert("b".into(), 69);
            map
        },
    };

    let value = i64::from_value(vm.call(["get_field"], (input.clone(),))?).into_result()?;
    assert_eq!(value, 42);

    let value = vm.call(["set_meta_field"], (input.clone(), 1337));
    assert!(value.is_err());

    let value = vm.call(["get_meta_field"], (input,));
    assert!(value.is_err());

    Ok(())
}

#[test]
fn dynamic_fields_first() -> Result<()> {
    register_type!(first);
    let mut vm = set_up_vm!();
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

    let value = i64::from_value(vm.call(["get_field"], (input.clone(),))?).into_result()?;
    assert_eq!(value, 69);

    vm.set_dynamic_fields(false);
    let value = vm.call(["get_meta_field"], (input.clone(),));
    assert!(value.is_err());

    let value = vm.call(["set_meta_field"], (input, 1337));
    assert!(value.is_err());

    Ok(())
}

#[test]
fn dynamic_fields_last() -> Result<()> {
    register_type!(last);
    let mut vm = set_up_vm!();

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

    let value = i64::from_value(vm.call(["get_field"], (input.clone(),))?).into_result()?;
    assert_eq!(value, 69);

    vm.set_dynamic_fields(false);
    let value = vm.call(["get_meta_field"], (input.clone(),));
    assert!(value.is_err());

    let value = vm.call(["set_meta_field"], (input, 1337));
    assert!(value.is_err());

    Ok(())
}

#[test]
fn dynamic_fields_only() -> Result<()> {
    register_type!(only);
    let mut vm = set_up_vm!();

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

    let value = vm.call(["get_field"], (input.clone(),));
    assert!(value.is_err());

    vm.set_dynamic_fields(false);
    let value = vm.call(["get_meta_field"], (input.clone(),));
    assert!(value.is_err());

    let value = vm.call(["set_meta_field"], (input.clone(), 1337));
    assert!(value.is_err());

    let value = i64::from_value(vm.call(["get_field"], (input,))?).into_result()?;
    assert_eq!(value, 69);

    Ok(())
}
