$Path = "D:\Repo\rust"
Copy-Item $Path\library\alloc\src\collections\btree\ -Destination third-party -Recurse -Force
Copy-Item $Path\library\alloc\src\vec\ -Destination third-party -Recurse -Force
Copy-Item $Path\library\alloc\src\collections\vec_deque\ -Destination third-party -Recurse -Force
Copy-Item $Path\library\alloc\src\testing\ -Destination third-party -Recurse -Force

$Path = "D:\Repo\hashbrown"
Copy-Item $Path\src\ -Destination third-party\hashbrown -Recurse -Force
