use anyhow::Result;

async fn run_main<T>(source: &str) -> Result<T>
where
    T: st::FromValue,
{
    Ok(rune_cli::run_program::<_, T>(source, "main", ()).await?)
}

#[tokio::test]
async fn test_arrays() {
    assert_eq! {
        run_main::<()>(r#"fn main() { let v = [1, 2, 3, 4, 5]; }"#).await.unwrap(),
        (),
    };
}
