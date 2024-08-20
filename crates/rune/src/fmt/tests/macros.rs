pub(super) use rust_alloc::vec::Vec;

fn find_prefix(input: &str) -> (usize, &str) {
    let mut prefix = 0;
    let mut count = 0;

    for c in input.chars() {
        if c == '\n' {
            prefix += count + c.len_utf8();
            count = 0;
            continue;
        }

        if c.is_whitespace() {
            count += c.len_utf8();
        } else {
            break;
        }
    }

    (prefix, &input[prefix..prefix + count])
}

pub(super) fn lines(input: &str) -> Vec<&str> {
    let mut out = Vec::new();

    let (prefix, indent) = find_prefix(input);

    input[prefix..].split('\n').for_each(|line| {
        if let Some(line) = line.get(indent.len()..) {
            out.push(line);
        } else {
            out.push("");
        }
    });

    while matches!(out.last().copied(), Some("")) {
        out.pop();
    }

    out
}

macro_rules! assert_format_with {
    ({ $($option:literal),* $(,)? }, $input:expr $(,)?) => {
        assert_format_with!({ $($option),* }, $input, $input)
    };

    ({ $($option:literal),* $(,)? }, $input:expr, $expected:expr $(,)?) => {{
        use $crate::fmt::tests::macros::lines;
        use $crate::fmt::tests::macros::Vec;

        let input = lines($input).join("\n");

        #[allow(unused_mut)]
        let mut options = $crate::compile::Options::from_default_env().expect("constructing options from env");

        $(options.parse_option($option).unwrap();)*

        let mut diagnostics = $crate::Diagnostics::new();

        let actual = match super::layout_source_with(&input, $crate::SourceId::EMPTY, &options, &mut diagnostics) {
            Ok(actual) => actual,
            Err(err) => {
                panic!("Failed to format source: {:?}", err);
            }
        };

        let actual = lines(actual.as_str());
        let expected = lines($expected);

        let mut mismatches = Vec::new();

        for (n, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
            if actual != expected {
                mismatches.push(format!(
                    "  {n:02}: {actual:?} (actual) != {expected:?} (expected)"
                ));
            }
        }

        if !mismatches.is_empty() {
            let mismatches = mismatches.join("\n");
            panic!("Mismatches:\n{mismatches}\n");
        }

        if actual.len() != expected.len() {
            let s = actual.len().min(expected.len());

            let actual = actual[s..]
                .iter()
                .enumerate()
                .map(|(n, line)| format!("{:02}: {line}", s + n))
                .collect::<Vec<_>>();

            let actual = actual.join("\n");

            let expected = expected[s..]
                .iter()
                .enumerate()
                .map(|(n, line)| format!("{:02}: {line}", s + n))
                .collect::<Vec<_>>();

            let expected = expected.join("\n");
            panic!("Mismatched:\nActual:\n{actual}\nExpected:\n{expected}\n");
        }
    }};
}

macro_rules! assert_format {
    ($input:expr $(,)?) => {
        assert_format_with!({}, $input)
    };

    ($input:expr, $expected:expr $(,)?) => {
        assert_format_with!({}, $input, $expected)
    };
}
