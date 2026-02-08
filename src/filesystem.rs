use std::path::{PathBuf, Path};
use crate::errors::{PomErrorCode, PomResult};
use std::fs;


// Learning notes: We define a trait = a list of methods a type must provide
// to be considered a `PathList`.
pub trait PathList {
    // Any type implementing `PathList` must provide this method.
    fn iter_paths(&self) -> Box<dyn Iterator<Item = &Path> + '_>;
    // Any type implementing `PathList` must provide this method.
    //
    // Return type dissection:
    // - `Iterator` is a *trait* (not a concrete type). It has an associated type `Item`.
    // - `Iterator<Item = &Path>` means: "an iterator whose yielded items are `&Path`".
    // - `dyn Iterator<...>` means: "a *trait object*": the concrete iterator type is
    //   intentionally hidden/unknown to the caller; calls go through dynamic dispatch
    //   (vtable). This lets us return different concrete iterator types from the same
    //   function (e.g. `once(...)` vs `slice.iter().map(...)`).
    // - `Box<dyn ...>`: a `dyn Trait` value has unknown size at compile time, so it must
    //   live behind a pointer. `Box` is an owning heap pointer, suitable for returning
    //   an iterator created inside this method.
    // - `+ '_`: this ties the trait object's lifetime to `&self`. The iterator may borrow
    //   from `self` (it yields `&Path` that come from paths stored in `self`), therefore
    //   the iterator cannot outlive `self`. Equivalent explicit form:
    //       `fn iter_paths<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Path> + 'a>`
    //
    // TL;DR: "Return an owned (Boxed) trait object iterator over borrowed `&Path` items,
    //         and ensure it can't outlive `self`."
}


impl PathList for Path {
    fn iter_paths(&self) -> Box<dyn Iterator<Item = &Path> + '_> {
        Box::new(std::iter::once(self))
    }
}


impl PathList for PathBuf {
    fn iter_paths(&self) -> Box<dyn Iterator<Item = &Path> + '_> {
        Box::new(  // Placed on the heap so it can be returned by reference
            std::iter::once(  // An iterator that yields exactly one element
                self.as_path() // A borrowed view of `self`
            )
        )
    }
}


impl PathList for [PathBuf] {
    fn iter_paths(&self) -> Box<dyn Iterator<Item = &Path> + '_> {
        Box::new(
            self.iter()  // `self` is `[PathBuf]`, we take an iterator over references to its elements
            .map(|p| p.as_path()))  // Transforms ("maps") each item (`|p|`) of this iterator into something else. Here: `&Path`s
    }
}


impl PathList for Vec<PathBuf> {
    fn iter_paths(&self) -> Box<dyn Iterator<Item = &Path> + '_> {
        self.as_slice().iter_paths()
    }
}


// Read parameter type as "A reference to a type implementing `PathList`"
pub fn create_directories(dirs_paths: &(impl PathList + ?Sized)) -> PomResult<()> {
    for dir_path in dirs_paths.iter_paths() {
        match fs::create_dir_all(dir_path) {
            Ok(()) => { continue },
            Err(src) => {
                let details = format!("{}: {:?}",dir_path.display(), src);
                return Err((
                    PomErrorCode::DirCreationFail,
                    Some(details),
                ));
            }
        };
    }
    Ok(())
}
