prelude!();

use crate::fmt::format_source;

#[track_caller]
fn assert_format_source(source: &str, expected: Option<&str>) -> Result<()> {
    let formated = format_source(source)?;
    let expected = expected.unwrap_or(source);
    assert_eq!(formated, expected);

    Ok(())
}

/// https://github.com/rune-rs/rune/issues/684
#[test]
fn bug_684() -> Result<()> {
    let source = r#"pub fn main() {
    /*
    test
    */
}
"#;

    assert_format_source(source, None)
}

#[test]
fn fmt_block_comment() -> Result<()> {
    let source = r#"//test1
/*test2*/"#;
    let expected = format!("{source}\n");

    assert_format_source(source, Some(&expected))
}

#[test]
fn fmt_block_comment_indent() -> Result<()> {
    let source = r#"struct Test {
    a, /* test1
    test2
test 3*/
}
"#;

    assert_format_source(source, None)
}

#[test]
fn fmt_block_comment_indent2() -> Result<()> {
    let source = r#"fn test() {
    /* test1
       test2 */

    if true {
        /*
        if false {
            // test3
        }
        */
    } /* else {
        // test 4
    } */
}
/* test 5.1
    test 5.2
        test 5.3
*/
"#;

    assert_format_source(source, None)
}

/// https://github.com/rune-rs/rune/issues/693
#[test]
fn bug_693() -> Result<()> {
    let source = r#"pub fn main() {
    if true {
        // test
    }
}
"#;

    assert_format_source(source, None)
}

#[test]
fn fmt_comment_line() -> Result<()> {
    let source = r#"pub fn main() {
    // test 1
    if true {
        // test 2.1
        let a = 1;
        // test 2.2
    }
    // test 3
}
"#;

    assert_format_source(source, None)
}

/// https://github.com/rune-rs/rune/issues/703
#[test]
fn bug_703() -> Result<()> {
    let source = r#"pub fn main() {
    const TEST = 1;
}
"#;

    assert_format_source(source, None)
}

#[test]
fn fmt_global_const() -> Result<()> {
    let source = r#"const TEST1=1;const TEST2=2;
const TEST3=1;"#;
    let expected = r#"const TEST1 = 1;
const TEST2 = 2;
const TEST3 = 1;
"#;

    assert_format_source(source, Some(expected))
}

#[test]
fn fmt_len() -> Result<()> {
    let source = r#"pub fn main() {
    let var = 1;
}
"#;

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
