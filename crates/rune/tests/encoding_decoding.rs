// A poorly named, integrated test module.
//
// This performs tests based on which serde features
// are enabled.

#[cfg(all(not(miri), any(feature = "serde", feature = "musli")))]
fn assert_ok<T, E>(item: Result<T, E>) -> T
where
    E: core::fmt::Display
{
    match item
    {
        Ok(t) => t,
        Err(e) => panic!("{}", e)
    }
}

#[cfg(all(not(miri), any(feature = "serde", feature = "musli")))]
fn test_decoded_sources(mut sources: rune::Sources)
{
    use rune::{Vm, Context, Diagnostics};
    use rune::sync::Arc;

    let context = assert_ok(Context::with_default_modules());

    let runtime = assert_ok(Arc::try_new(assert_ok(context.runtime())));

    let mut diagnostics = Diagnostics::new();

    let unit =
    assert_ok(
        rune::prepare(&mut sources)
        .with_diagnostics(&mut diagnostics)
        .build()
    );

    let mut vm = Vm::new(runtime, assert_ok(Arc::try_new(unit)));

    let output : i64 =
    {
        let ret = assert_ok(vm.call(["add"], (10_i64, 20_i64)));

        assert_ok(rune::from_value(ret))
    };

    assert_eq!(output, 30);
}

#[cfg(all(not(miri), feature = "serde"))]
mod serde
{
    // Use Ron as a format for serde

    use rune::{Source, Sources};

    use super::{assert_ok, test_decoded_sources};

    // Basic Ron serialization of Sources
    const RON_SERIALIZED : &str = r##"[(source:"pub fn add(a, b) { a + b }")]"##;

    #[test]
    fn encode_ron()
    {
        let mut sources = Sources::new();

        use ron::ser::{to_string_pretty, to_string, PrettyConfig};

        let source = assert_ok(Source::memory("pub fn add(a, b) { a + b }"));

        let _ = assert_ok(sources.insert(source));

        {
            let contents = assert_ok(to_string(&sources));

            // Check if serialization matches with Ron without pretty config
            assert_eq!(&contents, RON_SERIALIZED);
        }

        let config = PrettyConfig::default();

        let pretty_contents = assert_ok(to_string_pretty(&sources, config));

        // Check deserialization with pretty config
        check_ron_serialization(&pretty_contents)
    }

    fn check_ron_serialization(contents: &str)
    {
        use ron::de::from_str;

        let sources : Sources =
        assert_ok(from_str(contents));

        test_decoded_sources(sources);
    }

    #[test]
    fn decode_ron()
    {
        let ron_buf = RON_SERIALIZED;

        check_ron_serialization(ron_buf);
    }
}

#[cfg(all(not(miri), feature = "musli"))]
mod musli
{
    // Use wire as a format for musli

    use rune::{Source, Sources};

    use super::{assert_ok, test_decoded_sources};

    // Basic Wire encoding of Sources
    const ENCODED_WIRE : &[u8] =
    &[129, 130, 70, 115, 111, 117, 114, 99, 101, 90, 112, 117, 98, 32, 102, 110, 32, 97, 100, 100, 40, 97, 44, 32, 98, 41, 32, 123, 32, 97, 32, 43, 32, 98, 32, 125];

    #[test]
    fn encode_wire()
    {
        let mut sources = Sources::new();

        use musli::wire::to_vec;

        let source = assert_ok(Source::memory("pub fn add(a, b) { a + b }"));

        let _ = assert_ok(sources.insert(source));

        let wire = assert_ok(to_vec(&sources));

        assert_eq!(ENCODED_WIRE, &wire);

        check_wire_encoding(&wire);
    }

    fn check_wire_encoding(contents: &[u8])
    {
        use musli::wire::decode;

        let sources : Sources =
        assert_ok(decode(contents));

        test_decoded_sources(sources);
    }

    #[test]
    fn decode_wire()
    {
        let wire_buf = ENCODED_WIRE;

        check_wire_encoding(wire_buf);
    }

}
