use anyhow::Result;

async fn run_main<T>(source: &str) -> Result<T>
where
    T: st::FromValue,
{
    Ok(rune_cli::run_program::<_, T>(source, "main", ()).await?)
}

#[tokio::test]
async fn test_small_programs() {
    assert_eq! {
        run_main::<u64>(r#"fn main() { 42 }"#).await.unwrap(),
        42u64,
    };

    assert_eq! {
        run_main::<()>(r#"fn main() {}"#).await.unwrap(),
        (),
    };

    assert_eq! {
        run_main::<i64>(r#"
            fn main() {
                let a = 1;
                let b = 2;
                let c = a + b;
                let d = c * 2;
                let e = d / 3;
                e
            }
        "#).await.unwrap(),
        2,
    };
}

#[tokio::test]
async fn test_if() {
    assert_eq! {
        run_main::<i64>(r#"
            fn main() {
                let n = 2;

                if n > 5 {
                    10
                } else {
                    0
                }
            }
        "#).await.unwrap(),
        0,
    };

    assert_eq! {
        run_main::<i64>(r#"
            fn main() {
                let n = 6;

                if n > 5 {
                    10
                } else {
                    0
                }
            }
        "#).await.unwrap(),
        10,
    };
}
