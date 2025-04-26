use std::collections::HashMap;
use std::ffi::{c_char, c_void, CString, CStr, c_int};
use std::marker::PhantomData;

type HMODULE = HINSTANCE;
type HINSTANCE = HANDLE;
type HANDLE = *mut c_void;

type LPCSTR = *mut CHAR;
type CHAR = c_char;
type BOOL = c_int;

#[link(name = "kernel32", kind="static")]
unsafe extern "C" {
	pub fn LoadLibraryA(lpLibFileName: LPCSTR) -> HMODULE;
	pub fn GetProcAddress(hmodule: HMODULE, lpProcName: LPCSTR) -> *mut c_void;
	pub fn FreeLibrary(hLibModule: HMODULE) -> BOOL;
}

pub struct Library {
	module: HMODULE,
}

impl Drop for Library {
	fn drop (&mut self) {
		// SAFETY: FFI
		unsafe{FreeLibrary(self.module)};
	}
}

pub struct Symbol<'a, T> {
	ptr: *mut (),
	_marker: PhantomData<&'a T>,
}

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

impl Library {
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

	pub unsafe fn get<A: AsRef<CStr>, T>(&self, symbol_name: A) -> Option<Symbol<T>> {
		// SAFETY: FFI
		let ptr = unsafe{GetProcAddress(self.module, symbol_name.as_ref().as_ptr().cast_mut())};

		if !ptr.is_null() {
			Some(Symbol {
				ptr: std::mem::transmute(ptr),
				_marker: PhantomData,
			})
		} else {
			None
		}
	}
}

#[cfg(test)]
mod test {
}
