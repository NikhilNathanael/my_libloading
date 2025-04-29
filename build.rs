fn main () {
	// println!("cargo::warn={:?}", std::env::var("path"));
	// if !std::fs::exists("C:/Program Files (x86)/Windows Kits/10/Lib/10.0.26100.0/um/x64/kernel32.lib").unwrap_or(false) {
	// 	println!("cargo::error=Could not find Kernel32.lib")
	// }
	println!("cargo::rustc-link-search=native=C:/Program Files (x86)/Windows Kits/10/Lib/10.0.26100.0/um/x64/");
}
