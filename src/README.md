# Shy - A Shunting Yard Rules Engine

It is convenient for engineers unfamiliar with the programming language used to implement an application to still have a need to write rules that will be used in that application. If the formulas are embedded in the code, adding or editing them becomes tedious and error prone; it is better that they be stored in external text files in a simple format. Such formulas are usually expressed in infix notation, because that is how mathematics is normally notated. This library uses the **Shunting Yard** Algorithm to compile such expressions into a form convenient for execution. This algorithm was discovered by Edsger Dijkstra, a Danish physicist and computer scientist. 

## Modules

The main modules of this application are:

  1. `lexer` - The struct `Lexer` preprocesses the text into a stream of tokens, like number literals, variable names, and math operators. The lexical analyzer uses a pushdown automata state machine to interpret the individual characters and combine them to form tokens.
  2. `parser` - The struct `ShuntingYard` accepts the tokens and applies the **Shunting Yard** algorithm to reorder the tokens into a postfix order `Expression` struct. For example, if the original formula in infix order is "1 + 2", in postfix order it would be "1", "2", "+". This module also evaluates the resulting expression and provides the `ShyAssociation` trait and `ShyObject` struct to perform crude reflection. This permits the caller to use their own structs as context for the formulas. That way variables in the formulas can read from and write to the user supplied context, which is managed by the `ExecutionContext` struct.
  3. `cache` - Parsing the expressions takes almost 90% of the time when executing an expression, but we can cache already compiled expressions so that they may be reused. `ApproximateLRUCache` (which implements the `Cache` trait) can be used to map the uncompiled string form of the expression with its compiled `Expression`. 

## Repl

The file **main.rs** contains the function `repl()`, which means **read-execute-print-loop**. You can use it to test the formula parser and evaluator. (It does not use the cache.)

To run the repl, type this:

```
> cargo run
```

## Expression Syntax

Expressions may be written using the following elements:

  - **variables** - Variables may be read from the user supplied context or written back to it, depending on whether they appear on the left or right hand side of an assignment operator like '='. Variable names must start with a letter or underscore, and may consist of any number of letters, digits, and underscores. Those letters may be Latin or Greek.
  - **property chains** - A series of variable names separated by periods (with no intervening spaces) is a property chain. It will lookup a variable from the context using the first part of the chain, use the second part as a property to navigate, etc. following all properties as deep as necessary to get to the final value. When setting a value using a property chain, if any parts of the chain refer to objects that are missing, it will attempt to create them.
  - **numbers** - Numeric literals may be integers, decimal numbers, or numbers using exponential notation.
  - **strings** - String literals are enclosed in double quotes. If the string requires an embedded double quote, it may be escaped with a backslash. Other escape sequences are recognized for newlines (\n) and tabs (\t).
  - **booleans** - The values `true` and `false` are boolean literals.
  - `( )` - Use parentheses to group expressions.
  - **regular expressions** - Write these the same as strings (between double quotes) but follow **Rust** language regular expression syntax.
  - **operators** - Lots of them! They mostly follow the same precedence and associativity as popular computer languages.
    
     * `!` - This may be the **logical not** if it comes before an expression, or **factorial**, if it comes after.
     * `,` - The **comma operator** is used to collect multiple values into a list, to be used as the argument to a function.
     * `;` - **Semicolons** separate one subexpression from another. You can have one expression set a variable, then use that variable in the next expression.
     * `^` - **Exponentiation**. This raises a number to a power.
     * `¹ ² ³ ⁴ ⁵ ⁶ ⁷ ⁸ ⁹ ⁰` - **Superscripted numbers** can be used to raise a value to a power in place of the exponentiation operator.
     * `~` - The **match operator** matches the string on the left to the regex pattern on the right and returns true if the pattern on the right matches the string on the left.
     * `+ - * / %` - The basic arithmetic operators are supported. The percent sign is the modulus operator, which finds the remainder of a division.
     * `== < > <= >= && ||` - The logical and relational operators are supported.
     * `=` - The assignment operator will store values into the context.
     * `+= -= *= /= %= &&= ||=` - The compound assignment operators change a variable then store the new value. For example, `x += 1` will take the current value of x, add one, then store the new value back into x.
     * `√` - Square root operator.

  - **function calls** - If a token resembling a variable name immediately precedes an opening parenthesis, that name will be interpreted as a function name. Shy recognizes the common trigonometric functions, like `sin`, `cos`, and `tan`, as well as `exp`, `ln`, `sqrt` and `abs`. The caller can also define their own functions and bind them to an `ExecutionContext`. One useful function is `if(test, a, b)`, which takes three expressions: a test returning true or false, a second to return if the test is true, and a third to return if the test is false. See method `ExecutionContext::standard_functions` for the full list of predefined functions. (Also see method `standard_variables` for the list of predefined constants, including `π, e and φ`.)

If an expression fails due to numbers that are out of range or any other problem, a special error value is returned.
