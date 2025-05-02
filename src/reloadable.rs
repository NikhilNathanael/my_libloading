use crate::library::*;
use std::ffi::{CStr, CString};
use std::sync::Arc;
use std::marker::PhantomData;
use arc_atomic::AtomicArc;

pub struct ReloadableLibrary {
	name: &'static CStr,
	inner: AtomicArc<Inner>,
	symbols: Box<[CString]>,
}

struct Inner {
	_lib: Library,
	pointers: Box<[*mut ()]>,
}

impl Inner {
	pub fn new(_lib: Library, symbols: &[CString]) -> Self {
		let pointers = symbols.into_iter().map(|symbol| _lib.get_ptr(&symbol)
				.unwrap_or_else(|| panic!("Could not find symbol: {:?}", symbol))
			)
			.collect::<Box<[_]>>();
		Self {
			_lib,
			pointers,
		}
	}
}

impl ReloadableLibrary {
	/// Create a new ReloadableLibrary with the given name and load the given symbols from it
	/// Panics if the library is not found or if any of the symbols are not found.
	///
	/// symbols are not deduplicated so each instance of a duplicated symbol must be loaded,
	/// but only the first instance can be obtained through [Self::get_symbol], so consider 
	/// depduplicating symbols before passing them in
	pub fn new<const N: usize>(name: &'static CStr, symbols: [CString;N]) -> Self {
		// Load library
		let lib = Library::load(name)
			.unwrap_or_else(|| panic!("Could not load library {:?}", name));

		// turn library into Inner and put it in an atomic arc
		let inner = AtomicArc::new(
			Arc::new(
				Inner::new(lib, &symbols)
			)
		);

		Self {
			name,
			symbols: (&symbols as &[CString]).into(),
			inner,
		}
	}

	pub unsafe fn get_symbol<T>(&self, symbol: &CStr) -> Option<ReloadableSymbol<T>> {
		// Get symbol index in list
		let Some(symbol_index) = self.symbols.iter().enumerate().find(|(_, x)| &***x == symbol)
			.map(|x| x.0) else {
				return None;
		};

		// Arc::clone the Library
		let raw_lib = AtomicArc::new(self.inner.load());
		
		Some(ReloadableSymbol {
			symbol_index,
			reloadable_lib: self,
			raw_lib,
			_marker: PhantomData,
		})
	}

	pub fn reload(&self) {
		// Load new library
		let lib = Library::load(self.name)
			.unwrap_or_else(|| panic!("Could not reload library {:?}", self.name));

		self.inner.store(Arc::new(Inner::new(lib, &*self.symbols)));
	}
}

pub struct ReloadableSymbol<'a, T> {
	symbol_index: usize,
	reloadable_lib: &'a ReloadableLibrary,
	raw_lib: AtomicArc<Inner>,
	_marker: PhantomData<T>,
}

impl<'a, T> ReloadableSymbol<'a, T> {
	pub fn get_loaded(&self) -> LoadedSymbol<T> {
		let my_lib = self.raw_lib.load();
		let other_lib = self.reloadable_lib.inner.load();
		let lib = if !Arc::ptr_eq(&my_lib, &other_lib) {
			self.raw_lib.store(Arc::clone(&other_lib));
			other_lib
		} else {
			my_lib
		};

		let raw_symbol = unsafe{RawSymbol::from_ptr(lib.pointers[self.symbol_index])};
		unsafe{LoadedSymbol::new(lib, raw_symbol)}
	}
}

/// Holds a symbol and an Arc to the library it comes from
pub struct LoadedSymbol<T> {
	_lib: Arc<Inner>,
	symbol: RawSymbol<T>,
}
unsafe impl<T> Send for LoadedSymbol<T> {}
unsafe impl<T> Sync for LoadedSymbol<T> {}

impl<T> LoadedSymbol<T> {
	/// # SAFETY 
	/// - The symbol input must have come from the library input
	unsafe fn new(_lib: Arc<Inner>, symbol: RawSymbol<T>) -> Self {
		Self {
			_lib,
			symbol,
		}
	}
}

impl<T> std::ops::Deref for LoadedSymbol<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		// SAFETY: Library is loaded in an Arc
		unsafe {
			self.symbol.get()
		}
	}
}

impl<T> std::ops::DerefMut for LoadedSymbol<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		// SAFETY: Library is loaded in an Arc
		unsafe {
			self.symbol.get_mut()
		}
	}
}
