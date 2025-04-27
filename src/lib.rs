use std::ffi::{c_char, c_void, CStr, c_int};
use std::marker::PhantomData;

type HMODULE = HINSTANCE;
type HINSTANCE = HANDLE;
type HANDLE = *mut c_void;

type LPCSTR = *mut CHAR;
type CHAR = c_char;
type BOOL = c_int;

#[link(name = "kernel32", kind="static")]
unsafe extern "system" {
	pub fn LoadLibraryA(lpLibFileName: LPCSTR) -> HMODULE;
	pub fn GetProcAddress(hmodule: HMODULE, lpProcName: LPCSTR) -> *mut c_void;
	pub fn FreeLibrary(hLibModule: HMODULE) -> BOOL;
}

/// Wrapper over windows dll
/// 
/// Automatically unloads the library when dropped
pub struct Library {
	module: HMODULE,
}
unsafe impl Send for Library {}
unsafe impl Sync for Library {}

impl Library {
	/// Load a library at the given path
	///
	/// Returns None if the library does not exist
	pub fn load<A: AsRef<CStr>>(name: A) -> Option<Self> {
		let module = unsafe{LoadLibraryA(name.as_ref().as_ptr().cast_mut())};
		if !module.is_null() {
			Some(Self {
				module,
			})
		} else {
			None
		}
	}

	/// Gets the address of a symbol from the input library. 
	/// Returns None if the symbol is not found. The type of the returned 
	/// symbol cannot be checked and must be verified by the caller
	///
	/// The symbol borrows from the library which ensures it cannot be 
	/// used after the library is unloaded.
	///
	/// # SAFETY 
	/// The type of the symbol `T` MUST be verified.
	///
	/// If it is a function pointer, then then ABI, arguments and return type must be correct
	///
	/// If it is a pointer to a static member, then the type MUST be *const T or *mut T
	pub unsafe fn get<A: AsRef<CStr>, T>(&self, symbol_name: A) -> Option<Symbol<T>> {
		// SAFETY: FFI
		let ptr = unsafe{GetProcAddress(self.module, symbol_name.as_ref().as_ptr().cast_mut())};

		if !ptr.is_null() {
			Some(Symbol {
				ptr: ptr as *mut (),
				_marker: PhantomData,
			})
		} else {
			None
		}
	}
}

impl Drop for Library {
	fn drop (&mut self) {
		// SAFETY: FFI
		unsafe{FreeLibrary(self.module)};
	}
}

/// Holds a pointer to some symbol retrieved from a library.
/// It can be used with function pointers (fn(...) -> T) or static variables
///
/// When being used with function pointers, the 
pub struct Symbol<'a, T> {
	ptr: *mut (),
	_marker: PhantomData<&'a T>,
}

unsafe impl<'a, T> Send for Symbol<'a, T> {}
unsafe impl<'a, T> Sync for Symbol<'a, T> {}

impl<'a, T> std::ops::Deref for Symbol<'a, T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		unsafe {
			&*std::mem::transmute::<& *mut (), *mut T>(&self.ptr)
		}
	}
}

impl<'a, T> std::ops::DerefMut for Symbol<'a, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe {
			&mut *std::mem::transmute::<& *mut (), *mut T>(&mut self.ptr)
		}
	}
}
