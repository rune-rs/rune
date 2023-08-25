$Path = "D:\Repo\hashbrown"
Copy-Item $Path\src\raw\ -Destination crates\rune\src\hashbrown\fork\ -Recurse -Force
Copy-Item $Path\src\scopeguard.rs -Destination crates\rune\src\hashbrown\fork\scopeguard.rs -Force
Copy-Item $Path\src\macros.rs -Destination crates\rune\src\hashbrown\fork\macros.rs -Force

$template = Get-Content -Path crates\rune\src\hashbrown\fork\raw\mod.rs -Encoding UTF8 -Raw
$template = $template -replace 'crate::(?!alloc)', 'crate::hashbrown::fork::'
Set-Content -Path crates\rune\src\hashbrown\fork\raw\mod.rs -Value $template -Encoding UTF8
