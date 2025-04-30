use std::ffi::CStr;
use std::marker::PhantomData;
use windows_sys::Win32::System::LibraryLoader::{LoadLibraryA, GetProcAddress};
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};

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
		let module = unsafe{LoadLibraryA(name.as_ref().as_ptr().cast())};
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
	/// Unlike [RawSymbol], `Symbol` borrows from the [Library] which ensures it cannot be 
	/// used after the library is unloaded.
	///
	/// # SAFETY 
	/// The type of the symbol `T` MUST be verified.
	///
	/// If it is a function pointer, then then ABI, arguments and return type must be correct
	///
	/// If it is a pointer to a static member, then the type can be &T or &mut T if and only if the
	/// reference aliasing rules are upheld, otherwise use *const T or *mut T
	pub unsafe fn get<A: AsRef<CStr>, T>(&self, symbol_name: A) -> Option<Symbol<T>> {
		unsafe {
			self.get_raw(symbol_name).map(|raw| {
				Symbol {
					inner: raw,
					_marker: PhantomData,
				}
			})
		}
	}

	/// Gets the address of a symbol from the input library. 
	/// Returns None if the symbol is not found. The type of the returned 
	/// symbol cannot be checked and must be verified by the caller
	///
	/// The lifetime of the symbol is not checked. It is the responsibility of the
	/// caller to ensure that the library is still loaded. Use [Symbol] for a version which
	/// tracks lifetime
	///
	/// # SAFETY 
	/// The type of the symbol `T` MUST be verified.
	///
	/// If it is a function pointer, then then ABI, arguments and return type must be correct
	///
	/// If it is a pointer to a static member, then the type can be &T or &mut T if and only if the
	/// reference aliasing rules are upheld, otherwise use *const T or *mut T
	pub unsafe fn get_raw<A: AsRef<CStr>, T> (&self, symbol_name: A) -> Option<RawSymbol<T>> {
		// SAFETY: FFI
		let ptr = self.get_ptr(symbol_name);

		ptr.map(|ptr| {
			RawSymbol {
				ptr,
				_marker: PhantomData,
			}
		})
	}

	pub fn get_ptr<A: AsRef<CStr>>(&self, symbol_name: A) -> Option<*mut ()> {
		// SAFETY: FFI
		unsafe{GetProcAddress(self.module, symbol_name.as_ref().as_ptr().cast()).map(|x| x as *mut ())}
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
pub struct Symbol<'a, T> {
	inner: RawSymbol<T>,
	_marker: PhantomData<&'a T>,
}

unsafe impl<'a, T> Send for Symbol<'a, T> {}
unsafe impl<'a, T> Sync for Symbol<'a, T> {}

impl<'a, T> std::ops::Deref for Symbol<'a, T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		// SAFETY: 'a lifetime of self ensures library is not unloaded
		unsafe {
			self.inner.get()
		}
	}
}

impl<'a, T> std::ops::DerefMut for Symbol<'a, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		// SAFETY: 'a lifetime of self ensures library is not unloaded
		unsafe {
			self.inner.get_mut()
		}
	}
}

pub struct RawSymbol<T> {
	ptr: *mut (),
	_marker: PhantomData<T>,
}

impl<T> RawSymbol<T> {
	/// Turns a raw pointer into a raw symbol
	///
	/// # SAFETY
	/// See [Library::get_raw]
	pub unsafe fn from_ptr(ptr: *mut ()) -> Self {
		Self {
			ptr,
			_marker: PhantomData,
		}
	}
	
	/// Gets a reference to the pointer returned from get_raw
	/// 
	/// # Safety: 
	/// Library must still be loaded
	pub unsafe fn get(&self) -> &T {
		unsafe {
			std::mem::transmute(&self.ptr)
		}
	}
	
	/// Gets a mutable reference to the pointer returned from get_raw
	/// 
	/// # Safety: 
	/// Library must still be loaded
	pub unsafe fn get_mut(&mut self) -> &mut T {
		unsafe {
			std::mem::transmute(&mut self.ptr)
		}
	}
}
