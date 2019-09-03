use std::fmt::Debug;
use std::fmt::Display;

fn indent_str<S: Into<String>>(unindented: S, indent_by: usize) -> String {
    let first_indent = &format!("{}", " ".repeat(indent_by));
    let indent = &format!("\n{}", " ".repeat(indent_by));
    let mut indented = String::new();
    indented.push_str(&first_indent);
    for c in unindented.into().chars() { 
        match c {
                '\n' => indented.push_str(indent), 
                _ => indented.push(c)
        };
    }
    indented
}

pub trait IndentDebug {
    fn indent_debug(&self, indent_by: usize) -> String;
}

pub trait IndentDisplay {
    fn indent_display(&self, indent_by: usize) -> String;
}


impl<T: Debug> IndentDebug for T {
    fn indent_debug(&self, indent_by: usize) -> String {
        indent_str(format!("{:?}", self), indent_by)
    }
}

impl<T: Display> IndentDisplay for T {
    fn indent_display(&self, indent_by: usize) -> String {
        indent_str(format!("{}", self), indent_by)
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    #[test]
    fn indent() {
        let s = "To err is human,\nTo forgive, divine.";
        let expected = "   To err is human,\n   To forgive, divine.";
        let actual = s.indent_display(3);
        assert!(actual.eq(expected));
    }

}
