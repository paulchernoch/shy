use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Write;

/// Indent a string, adding spaces at the start of the string and after every newline,
/// except the final newline if it is the last character.
fn indent_str<S: Into<String>>(unindented: S, indent_by: usize) -> String {
    let indent = &" ".repeat(indent_by);
    let mut indented = String::new();
    indented.push_str(indent);
    let mut hold = "";
    for c in unindented.into().chars() {
        if hold.len() > 0 {
            indented.push_str(hold);
            hold = "";
        }
        if c == '\n' {
            hold = indent;
        } 
        indented.push(c);
    }
    indented
}

pub trait IndentDebug {
    fn indent_debug(&self, indent_by: usize) -> String;
}

pub trait IndentDisplay {
    fn indent_display(&self, indent_by: usize) -> String;
}

pub fn write_debug<T: Debug>(value : T, err_value : &'static str) -> String {
    let mut s = String::new();
    match write!(s, "{:?}", value) {
        Err(_) => err_value.into(),
        _ => s
    }
}

pub fn write_display<T: Display>(value : T, err_value : &'static str) -> String {
    let mut s = String::new();
    match write!(s, "{}", value) {
        Err(_) => err_value.into(),
        _ => s
    }
}


impl<T: Debug> IndentDebug for T {
    fn indent_debug(&self, indent_by: usize) -> String {
        indent_str(write_debug(self, "Error"), indent_by)
    }
}

impl<T: Display> IndentDisplay for T {
    fn indent_display(&self, indent_by: usize) -> String {
        let mut s = String::new();
        match write!(s, "{}", self) {
            Err(_) => indent_str("Error", indent_by),
            _ => indent_str(s, indent_by)
        }
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
