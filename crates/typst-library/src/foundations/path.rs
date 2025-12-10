use typst_syntax::path::{PathError, VirtualPath, VirtualRoot};
use typst_utils::Id;

use crate::diag::{HintedStrResult, HintedString, error};
use crate::foundations::{Repr, Str, cast};

/// A path string.
///
/// This type is commonly accepted by functions that read from a path.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PathStr(pub Str);

impl PathStr {
    /// Resolves this path or string relative to the file that resides at
    /// `within`.
    ///
    /// This path string may be absolute or relative. If relative, it's resolved
    /// relative to the parent directory of this file path (`self` is supposed
    /// to be a file path, not a directory path).
    pub fn resolve(&self, within: &VirtualPath) -> HintedStrResult<VirtualPath> {
        match within.parent() {
            Some(parent) => parent.join(&self.0),
            None => within.join(&self.0),
        }
        .map_err(|err| format_path_error(err, within.root(), &self.0))
    }

    /// [Resolves](Self::resolve) the path if `within` is `Some(_)` or return
    /// an error that the file system cannot be accessed, otherwise.
    pub fn resolve_if_some(
        &self,
        within: Option<Id<VirtualPath>>,
    ) -> HintedStrResult<VirtualPath> {
        self.resolve(within.as_ref().ok_or("cannot access file system from here")?)
    }
}

cast! {
    PathStr,
    self => self.0.into_value(),
    v: Str => Self(v),
}

/// Format the user-facing YAML path message.
fn format_path_error(err: PathError, root: &VirtualRoot, path: &str) -> HintedString {
    match err {
        PathError::Escapes => {
            let kind = match root {
                VirtualRoot::Project => "project",
                VirtualRoot::Package(_) => "package",
            };
            let mut diag = error!(
                "path would escape the {kind} root";
                hint: "cannot access files outside of the {kind} sandbox";
            );
            if *root == VirtualRoot::Project {
                diag.hint("you can adjust the project root with the --root argument");
            }
            diag
        }
        PathError::Backslash => error!(
            "path must not contain a backslash";
            hint: "use forward slashes instead: `{}`",
            path.replace("\\", "/").repr();
            hint: "in earlier Typst versions, backslashes indicated path separators on Windows";
            hint: "this behavior is no longer supported as it is not portable";
        ),
    }
}

#[cfg(test)]
mod tests {
    use typst_syntax::path::VirtualRoot;

    use super::*;

    #[test]
    fn test_resolve() {
        let path = |p| VirtualPath::new(VirtualRoot::Project, p).unwrap();
        let p1 = path("src/main.typ");
        let resolve =
            |s: &str| PathStr(s.into()).resolve(&p1).map_err(|err| err.message().clone());
        assert_eq!(resolve("works.bib"), Ok(path("src/works.bib")));
        assert_eq!(resolve(""), Ok(path("/src")));
        assert_eq!(resolve("."), Ok(path("/src")));
        assert_eq!(resolve(".."), Ok(path("/")));
        assert_eq!(resolve("../.."), Err("path would escape the project root".into()));
        assert_eq!(resolve("a\\b"), Err("path must not contain a backslash".into()));
    }
}
