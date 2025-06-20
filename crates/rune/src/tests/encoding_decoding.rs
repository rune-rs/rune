// A poorly named test module.
use crate::{Source, Sources};

// A short-hand way for unwrapping sources, while at the
// same time, proving that they are ok.
//
// Panics occur if it is an error.
macro_rules! assert_ok
{
    ($res:expr)
    =>
    {
        match $res
        {
            Ok(t) => t,
            Err(e) => panic!("{}", e)
        }
    }
}

pub(crate) use assert_ok;

// Sample sources for serde/musli to use.
fn sample_memory_source() -> Source
{
    assert_ok!(Source::memory("pub fn add(a, b) { a + b }"))
}

fn sample_name_source() -> Source
{
    assert_ok!(Source::new("hello world", r#"pub fn hello() { "Hello World!" }"#))
}

fn sample_path_source() -> Source
{
    assert_ok!(Source::with_path("if_check", "pub fn if_check(a) { if a > 128 { true } else { false } }", "fake.rn"))
}

fn sample_sources() -> Sources
{
    let mut sources = Sources::new();

    let _ = assert_ok!(sources.insert(sample_memory_source()));
    let _ = assert_ok!(sources.insert(sample_name_source()));
    let _ = assert_ok!(sources.insert(sample_path_source()));

    sources
}

#[cfg(feature = "serde")]
mod serde
{
    use crate::{Source, Sources};

    use ron::{
        de::from_str,
        ser::{PrettyConfig, self}
    };

    use super::{
        assert_ok,
        sample_memory_source,
        sample_name_source,
        sample_path_source,
        sample_sources
    };

    // Macro for checking ron serialization.
    macro_rules! serde_verify_ser
    {
        ($($a:expr; $b:ty),*) =>
        {
            $(
                let _original_source = &$a;

                let _ron_string = assert_ok!(ser::to_string(&$a));

                let _ron_pretty_string = assert_ok!(ser::to_string_pretty(&$a, PrettyConfig::default()));

                assert_ne!(&_ron_string, &_ron_pretty_string);

                let _source_from_regular : $b = assert_ok!(from_str(&_ron_string));

                let _source_from_pretty : $b = assert_ok!(from_str(&_ron_pretty_string));

                assert_eq!(&_source_from_regular, &_source_from_pretty);

                assert_eq!(_source_from_regular, $a);
            )*
        }
    }

    #[test]
    fn serde_serialization()
    {

        serde_verify_ser!(sample_memory_source(); Source,
                          sample_name_source(); Source,
                          sample_path_source(); Source,
                          sample_sources(); Sources);
    }

    // This test goes over a manually-constructed string of Sources, testing deserialization for each source added.
    #[test]
    fn serde_deserialization()
    {
        use crate::alloc::String;

        let mut sources = assert_ok!(String::try_from("["));

        {
            let memory_check = r#"Source(source:"pub fn add(a, b) { a + b }",)"#;

            let deserialized_memory : Source = assert_ok!(from_str(memory_check));

            assert_eq!(deserialized_memory, sample_memory_source());

            assert_ok!(sources.try_push_str(memory_check));
            assert_ok!(sources.try_push(','));
        }

        {
            let name_check = r#"Source(name:"hello world",source:"pub fn hello() { \"Hello World!\" }",)"#;

            let deserialized_name : Source = assert_ok!(from_str(name_check));

            assert_eq!(deserialized_name, sample_name_source());

            assert_ok!(sources.try_push_str(name_check));
            assert_ok!(sources.try_push(','));
        }

        {
            let path_check = r#"Source (name:"if_check",source:"pub fn if_check(a) { if a > 128 { true } else { false } }",path:"fake.rn",)"#;

            let deserialized_path : Source = assert_ok!(from_str(path_check));

            assert_eq!(deserialized_path, sample_path_source());

            assert_ok!(sources.try_push_str(path_check));
            assert_ok!(sources.try_push(','));
        }

        assert_ok!(sources.try_push(']'));

        let output : Sources = assert_ok!(from_str(&sources));

        assert_eq!(output, sample_sources());
    }
}

#[cfg(feature = "musli")]
mod musli
{
    use musli::assert_roundtrip_eq;

    use super::{
        sample_memory_source,
        sample_name_source,
        sample_path_source,
        sample_sources
    };

    macro_rules! roundtrip
    {
        ($($a:expr),*) =>
        {
            $(
                assert_roundtrip_eq!(full, $a);
            )*
        }
    }

    #[test]
    fn musli_roundtrip()
    {
        roundtrip!(
            sample_memory_source(),
            sample_name_source(),
            sample_path_source(),
            sample_sources());
    }
}
