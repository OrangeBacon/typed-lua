//! Generic tree printing functions, used for the specialised tree printers in
//! each individual pass.

use std::fmt::{self, Formatter};

use crate::utils::{Size, SizeOf};

/// Data required for printing a tree
#[derive(Debug, Clone)]
pub struct TreeCtx {
    top_level: bool,
    top_size: usize,
    prefix: String,
    child_prefix: String,
    name: String,
}

impl TreeCtx {
    /// Create a new tree printer, at the root level.  The object passed is
    /// used for the size of the whole tree.
    pub fn new(n: impl SizeOf) -> Self {
        Self {
            top_level: true,
            top_size: n.size(),
            prefix: String::new(),
            child_prefix: String::new(),
            name: String::new(),
        }
    }

    /// Create a tree printer for a non-last-child tree node
    pub fn child(&self) -> Self {
        Self {
            top_level: false,
            top_size: self.top_size,
            prefix: format!("{}|- ", self.child_prefix),
            child_prefix: format!("{}|  ", self.child_prefix),
            name: String::new(),
        }
    }

    /// Create a tree printer for a last-child tree node
    pub fn last(&self) -> Self {
        Self {
            top_level: false,
            top_size: self.top_size,
            prefix: format!("{}`- ", self.child_prefix),
            child_prefix: format!("{}   ", self.child_prefix),
            name: String::new(),
        }
    }

    /// Set the name of this tree node
    pub fn name(&mut self, name: &str) {
        self.name = if self.name.is_empty() {
            name.to_string()
        } else {
            format!("{}: {name}", self.name)
        };
    }

    /// Format this node, using the provided formatter.
    /// `value` is the text to be written to describe the node, `len` is the
    /// number of child elements expected, `size` is the byte size of this individual
    /// node.
    pub fn print(
        &self,
        f: &mut Formatter<'_>,
        value: &str,
        len: impl Into<Option<usize>>,
        size: &(impl ?Sized + SizeOf),
    ) -> fmt::Result {
        if !self.prefix.is_empty() {
            write!(f, "{}", self.prefix)?;
        }

        if !self.name.is_empty() {
            write!(f, "{}: ", self.name)?;
        }

        write!(f, "{value}")?;

        if let Some(len) = len.into() {
            write!(f, "[{len}]")?;
        }

        let size = size.size();
        if self.top_level || (size as f64) > (self.top_size as f64) * 0.2 {
            write!(f, " <{}>", Size(size))?;
        }

        writeln!(f)?;

        Ok(())
    }
}
