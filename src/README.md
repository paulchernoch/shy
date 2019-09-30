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

Enter one expression per line. If you set the value of a variable one one line, it will be available to formulas on the next line.

There are a few special commands:

  - **trace on** - Show the entire process of executing the expression, phrase by phase.
  - **trace off** - Turn off the detailed execution tracing.
  - **exit** - Exit the program.
  - **quit** - Exit the program.

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
     * `,` - The **comma operator** is used to collect multiple values into a list, to be used as inputs to a function.
     * `;` - **Semicolons** separate one subexpression from another. You can have one expression set a variable, then use that variable in the next expression.
     * `^` - **Exponentiation**. This raises a number to a power.
     * `¹ ² ³ ⁴ ⁵ ⁶ ⁷ ⁸ ⁹ ⁰` - **Superscripted numbers** can be used to raise a value to a power in place of the exponentiation operator.
     * `~` - The **match operator** matches the string on the left to the regex pattern on the right and returns true if the pattern on the right matches the string on the left.
     * `+ - * / %` - The basic arithmetic operators are supported. The percent sign is the modulus operator, which finds the remainder of a division.
     * `== < > <= >= && ||` - The logical and relational operators are supported.
     * `=` - The assignment operator will store values into the context.
     * `+= -= *= /= %= &&= ||=` - The compound assignment operators change a variable then store the new value. For example, `x += 1` will take the current value of x, add one, then store the new value back into x.
     * `√` - Square root operator.
     * `?` - The **quit-if-false** operator. This is a postfix operator. If the preceding expression evaluates to false, evaluation of the expression ends immediately and no side-effects (such as the assignment of variables) from the rest of the expression are evaluated. Using this operator, you can use the first several statements of an expression to test if a rule is applicable. Only if it is applicable shall the rest of the expression be performed. This means that any side-effects (setting of variables) performed after the test are skipped if the test is false.

  - **function calls** - If a token resembling a variable name immediately precedes an opening parenthesis, that name will be interpreted as a function name. Shy recognizes the common trigonometric functions, like `sin`, `cos`, and `tan`, as well as `exp`, `ln`, `sqrt` and `abs`. The caller can also define their own functions and bind them to an `ExecutionContext`. One useful function is `if(test, a, b)`, which takes three expressions: a test returning true or false, a second to return if the test is true, and a third to return if the test is false. See method `ExecutionContext::standard_functions` for the full list of predefined functions. (Also see method `standard_variables` for the list of predefined constants, including `π, e and φ`.)

One subset of the functions are the voting functions, that take one or more expressions that evaluate to true or false:

  -  none - True if None are true
  -  one - True if Exactly one is true
  -  any - True if One or more are true
  -  minority - True if Less than half (but at least one) are true
  -  half - True if Half or more are true
  -  majority - True if More than half are true
  -  twothirds - True if Two-thirds or more are true
  -  allbutone - True if Exactly one is false
  -  all - True if All are true 
  -  unanimous - True if All are true OR All are false

An example expression that calls the majority function:

`tall = false; dark = true; handsome = true; answer = majority(tall, dark, handsome)`

If an expression fails due to numbers that are out of range or any other problem, a special error value is returned.

## Using the Cache to speed up Expression evaluation

Parsing takes the bulk of the time when executing `Expressions`. On a Windows Tablet, these were the results of a performance test, demonstrating how useful it is to employ a cache:

  - 4,400 evals per sec without cache 
  - 42,600 evals per sec with a cache

Here is an example of how to use a cache:

```
        let mut cache : ApproximateLRUCache<String, Expression> 
            = ApproximateLRUCache::new(10000);
        let mut ctx = ExecutionContext::default();
        ctx.store(&"x1".to_string(), 5.into());
        ctx.store(&"y1".to_string(), 10.into());
        ctx.store(&"x2".to_string(), 1.into());
        ctx.store(&"y3".to_string(), 7.into());
        let expression_text = "distance = √((x1-x2)² + (y1-y2)²)";
        let expression = cache.get_or_add(
            &expression_text.to_string(), 
            & |expr| {
                let shy : ShuntingYard = expr.into();
                let compiled = shy.compile().unwrap();
                compiled
            } ).unwrap();
        let result = expression.exec(ctx).unwrap();
```

The above code, of course, should be rewritten to use match statements and handle errors properly, since the `unwrap` calls could cause a panic if an expression with a syntax error is encountered.

The closure performs the parsing and is only called if the `Expression` is not found in the cache. In that case, the `Expression` is created and added to the cache.

1. The first statement allocates a cache with maximum capacity of 10,000. It is defined to expect a `String` as a key and a compiled `Expression` as a value. 
2. The next statement allocates a default `ExecutionContext` that has the standard math functions and constants defined, as well as the "if" function.
3. The next four statements define and initialize variables in the context that will be available to our expressions. The `to_string()` call is necessary to convert the literal from an unowned string slice into an owned string, 
and the `&` then borrows it for use in the call to `store`. Lastly, the `into` call converts the integer values into `ShyValue` structs using type inference, because the second function argument of `store` is expected to be a `ShyValue`.
4. After defining the string version of a distance formula...
5. ... we attempt to retrieve its compiled `Expression` from the cache by calling `get_or_add`.
6. The call to `get_or_add` requires a closure as its second argument. This closure is only called if the expression is not found in the cache already. It acts as a factory to convert the string expression into a compiled `Expression` object, using the `ShuntingYard` struct's `compile` method. After calling this delegate, `get_or_add` will add it to the cache.
7. Finally, with a compiled `Expression` in hand, we can call `exec` and `unwrap` the returned `Result` enum to get the value of the expression.

Since the result was also written to the context as the variable **distance**, we could also read the result from the context using `ExecutionContext::load`.

