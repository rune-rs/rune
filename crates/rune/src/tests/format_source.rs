prelude!();

use crate::fmt::format_source;

fn assert_format_source(source: &str, expected: Option<&str>) -> Result<()> {
    let formated = format_source(source)?;
    let expected = expected.as_deref().unwrap_or(source);
    assert_eq!(formated, expected);

    Ok(())
}

/// https://github.com/rune-rs/rune/issues/684
#[test]
#[ignore]
fn bug_684() -> Result<()> {
    let source = r#"pub fn main() {
    /*
    test
    */
}"#;

    assert_format_source(source, None)
}

/// https://github.com/rune-rs/rune/issues/693
#[test]
#[ignore]
fn bug_693() -> Result<()> {
    let source = r#"pub fn main() {
    if true {
        // test
    }
}"#;

    assert_format_source(source, None)
}

/// https://github.com/rune-rs/rune/issues/703
#[test]
#[ignore]
fn bug_703() -> Result<()> {
    let source = r#"pub fn main() {
    const TEST = 1;
}"#;

    assert_format_source(source, None)
}

#[test]
#[ignore]
fn fmt_println() -> Result<()> {
    let source = r#"pub fn main(){println!("The value is {}",42);}"#;
    let expected = r#"pub fn main() {
    println!("The value is {}", 42);
}
"#;

    assert_format_source(source, Some(expected))
}

#[test]
fn fmt_while_loop() -> Result<()> {
    let source = r#"pub fn main(){let value=0;while value<100{if value>=50{break;}value=value+1;}println!("The value is {}",value);// => The value is 50
}"#;
    let expected = r#"pub fn main() {
    let value = 0;
    while value < 100 {
        if value >= 50 {
            break;
        }
        value = value + 1;
    }
    println!("The value is {}",value); // => The value is 50
}
"#;

    assert_format_source(source, Some(expected))
}

#[test]
fn fmt_async_http_timeout() -> Result<()> {
    let source = r#"struct Timeout;

async fn request(timeout) {
    let request = http::get(`http://httpstat.us/200?sleep=${timeout}`);
    let timeout = time::sleep(time::Duration::from_secs(2));

    let result = select {
        _ = timeout => Err(Timeout),
        res = request => res,
    }?;

    println!("{}", result.status());
    Ok(())
}

pub async fn main() {
    if let Err(Timeout) = request(1000).await {
        println("Request timed out!");
    }

    if let Err(Timeout) = request(4000).await {
        println("Request timed out!");
    }
}
"#;

    assert_format_source(source, None)
}
