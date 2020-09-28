use crate::testing::*;

#[test]
fn test_paths_may_contain_crate() {
    // root level names accessible with `crate`
    assert_parse!(r#"const MSG = "A"; fn main() {println(crate::MSG)}"#);

    // `*` imports work as expected with `crate`
    assert_parse!(include_str!("./001-use_name_cycles_with_crate.rn"));

    // `use` name cycles work with crate
    assert_parse!(include_str!("./002-crate_prefixed_star_imports.rn"));
}

#[test]
fn test_paths_may_contain_super() {
    assert_compile_error! {
        r#"fn main() { super::abc; }"#,
           span, UnresolvedTypeOrModule { name } => {
            assert_eq!(name,  "super");
            assert_eq!(span, Span::new(12, 17));
        }
    }
    assert_parse!(include_str!("./003-basic_super_path_resolution.rn"));
}

#[test]
fn test_paths_may_contain_self_type() {
    assert_parse!(include_str!("./004-basic_Self_path_resolution.rn"));

    // an impl for a struct defined in an instance function of another
    // function
    assert_parse!(include_str!("./000-nested_impl_items.rn"));
}

#[test]
fn test_paths_may_contain_self_value() {
    // root level names accessible with `self`
    assert_parse!(r#"const MSG = "xyz"; fn main() {self::MSG}"#);
}
