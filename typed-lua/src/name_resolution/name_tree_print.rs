use std::fmt::{self, Display, Formatter};

use crate::{
    name_resolution::name_tree::*,
    parser::ast,
    utils::{SizeOf, TreeCtx},
};

/// Pretty printer for all AST nodes
pub struct NtPrint<'a, T: ?Sized> {
    node: &'a T,
    tree: TreeCtx,
    strings: &'a [Vec<u8>],
    variables: &'a [Local],
    labels: &'a [Label],
}

impl<'a, T: SizeOf> NtPrint<'a, NameContainer<T>> {
    /// Create a new node printer
    pub fn new(container: &'a NameContainer<T>) -> Self {
        Self {
            node: container,
            tree: TreeCtx::new(container),
            strings: &container.string_table,
            variables: &container.variable_table,
            labels: &container.label_table,
        }
        .name("Name Tree")
    }
}

impl<'a, T> NtPrint<'a, T> {
    /// Display a non-last-child element of this node
    fn child<U>(&self, child: &'a U) -> NtPrint<'a, U> {
        NtPrint {
            node: child,
            tree: self.tree.child(),
            strings: self.strings,
            variables: self.variables,
            labels: self.labels,
        }
    }

    /// Display the last child of this node
    fn last<U>(&self, last: &'a U) -> NtPrint<'a, U> {
        NtPrint {
            node: last,
            tree: self.tree.last(),
            strings: self.strings,
            variables: self.variables,
            labels: self.labels,
        }
    }

    /// Set the name of this node
    fn name(mut self, name: &str) -> Self {
        self.tree.name(name);
        self
    }

    /// Swap the content without changing levels
    fn swap<U: ?Sized>(&self, swap: &'a U) -> NtPrint<'a, U> {
        NtPrint {
            node: swap,
            tree: self.tree.clone(),
            strings: self.strings,
            variables: self.variables,
            labels: self.labels,
        }
    }

    /// Get a printable version of a string from the name tree.  No quotation
    /// marks, un-printable characters are escaped.
    fn display_string(&self, id: StringId) -> String {
        let mut input = self.strings[id.0 as usize].as_slice();
        let mut out = String::new();

        loop {
            match std::str::from_utf8(input) {
                Ok(valid) => {
                    out.extend(valid.escape_default());
                    break;
                }
                Err(error) => {
                    let (valid, after_valid) = input.split_at(error.valid_up_to());
                    out.extend(std::str::from_utf8(valid).unwrap().escape_default());

                    let invalid_len = error.error_len().unwrap_or(after_valid.len());
                    for &byte in &after_valid[0..=invalid_len] {
                        out.push_str(&format!(r"\x{:X}", byte));
                    }

                    input = &after_valid[invalid_len..];
                }
            }
        }

        out
    }
}

impl<T: ?Sized + SizeOf> NtPrint<'_, T> {
    /// Display the name of this node
    fn print(&self, f: &mut Formatter<'_>, name: &str) -> fmt::Result {
        self.tree.print(f, name, None, self.node)
    }

    /// Display the name of this node, with a node count
    fn print_len(
        &self,
        f: &mut Formatter<'_>,
        name: &str,
        len: impl Into<Option<usize>>,
    ) -> fmt::Result {
        self.tree.print(f, name, len, self.node)
    }
}

impl Display for NtPrint<'_, str> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, self.node)
    }
}

impl Display for NtPrint<'_, &str> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, self.node)
    }
}

impl Display for NtPrint<'_, usize> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, &format!("{}", *self.node))
    }
}

impl<'a, T> Display for NtPrint<'a, Vec<T>>
where
    NtPrint<'a, T>: Display,
    T: SizeOf,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let count = self.node.len();

        self.print_len(f, "List", count)?;
        for (idx, s) in self.node.iter().enumerate() {
            if idx + 1 == count {
                self.last(s).fmt(f)?;
            } else {
                self.child(s).fmt(f)?;
            }
        }

        Ok(())
    }
}

impl<'a, T> Display for NtPrint<'a, NameContainer<T>>
where
    NtPrint<'a, T>: Display,
    T: SizeOf,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, "Container")?;
        self.child(&self.node.env).name("Env").fmt(f)?;
        self.child(&self.node.string_table.len())
            .name("Strings")
            .fmt(f)?;
        self.child(&self.node.variable_table.len())
            .name("Variables")
            .fmt(f)?;
        self.child(&self.node.label_table.len())
            .name("Labels")
            .fmt(f)?;
        self.last(&self.node.tree).fmt(f)
    }
}

impl Display for NtPrint<'_, StringId> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, &self.display_string(*self.node))
    }
}

impl Display for NtPrint<'_, VariableId> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let var = &self.variables[self.node.0 as usize];
        let name = self.display_string(var.name);

        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            self.print(f, &format!("%{name}.{}", var.name.0))
        } else {
            self.print(f, &format!("%\"{name}\".{}", var.name.0))
        }
    }
}

impl Display for NtPrint<'_, LabelId> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let label = &self.labels[self.node.0 as usize];

        if let Some(id) = label.name {
            let name = self.display_string(id);

            if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                self.print(f, &format!("@{name}.{}", self.node.0))
            } else {
                self.print(f, &format!("@\"{name}\".{}", self.node.0))
            }
        } else {
            self.print(f, &format!("@.{}", self.node.0))
        }
    }
}

impl Display for NtPrint<'_, Number> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

// impl Display for NtPrint<'_, Local> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         todo!()
//     }
// }

// impl Display for NtPrint<'_, Label> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         todo!()
//     }
// }

impl Display for NtPrint<'_, Block> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        //todo!()
        Ok(())
    }
}

impl Display for NtPrint<'_, Statement> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, ReturnStatement> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, FunctionName> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, Var> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, Expression> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, PrefixExpression> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, FunctionCall> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, FunctionArgs> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, Function> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, ParameterList> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, FieldList> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, Field> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Display for NtPrint<'_, ast::BinaryOperator> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, &format!("{:?}", self.node))
    }
}

impl Display for NtPrint<'_, ast::UnaryOperator> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.print(f, &format!("{:?}", self.node))
    }
}
