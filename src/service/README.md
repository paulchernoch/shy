# Shy Web Service Restful API

The Shy Web Service presents a **REST API** interface to its functionality. It holds a cache of Rulesets, which may be _added_, _retrieved_, _updated_, _deleted_, and _executed_.

The essential objects are:

  - **Ruleset** - A named list of compiled **Rules**.
  - **Rule** - A named, compiled **Expression**.
  - **Expression** - An unnamed formula which may be in the form of source text or compiled form.
  - **Context** - Data with properties. A Context may be used as the source of variables whose values are needed as inputs when evaluating Expressions and Rules. A Context may also serve as the target for results to be written.
  - **Service Cache** - Holds compiled **Rulesets**, which may be reused in subsequent calls.

## REST Commands

1. **List Rulesets** - Get a list of the names of all **Rulesets** stored in the service cache. 
2. **Add Ruleset** - Add a **Ruleset** having the given name to the **service cache**, or completely replace the existing **Ruleset**.
3. **Get Ruleset** - Get a **Ruleset** from the service cache that has the given name.
4. **Update Ruleset** - Update the service cache with new versions of one or more rules  in a **Ruleset**. **Rules** belonging to the previously cached **Ruleset** in the cache which are not part of the request are left alone.
5. **Delete Ruleset** - Delete the given **Ruleset** from the service cache.
6. **Execute Expression** - Compile and execute an **Expression** without a data **context**, bypassing the **service cache**. Return the result of the evaluation.
7. **Execute Expression with Context** - Compile and execute an **Expression** with a data **context**, bypassing the **service cache**.
8. **Execute Ruleset with Context** - Name a **Ruleset** expected to be in the **service cache** and supply a **context**.

## Endpoint Syntax

| Command                    | REST Syntax                    | Posted Data         |
| -------------------------- | ------------------------------ | ------------------- |
| List Rulesets              | GET /rulesets                  | N/A                 |
| List Rulesets for category | GET /rulesets?category={cat}   | N/A                 |
| Add Ruleset                | PUT /rulesets/{name}           | Ruleset             |
| Get Ruleset                | GET /rulesets/{name}           | N/A                 |
| Update Ruleset             | POST /rulesets/{name}          | Partial Ruleset     |
| Delete Ruleset             | DELETE /rulesets/{name}        | N/A                 |
| Execute Expression         | POST /expression/execute       | Expression, Context |
| Exec Ruleset               | POST /rulesets/{name}/execute  | Context             |


## Examples

1. Evaluate an expression without a context:

  - Call: POST /expression/execute
  - JSON Request Body:

```
{
  "expression": "r = 5; area = π * r²"
}
```
  - Response:

```
{
    "result": 78.53981633974483,
    "context": null,
    "error": null
}
```

2. Evaluate an expression with a context:

  - Call: POST /expression/execute
  - JSON Request Body:

{
  "expression": "result = well.depth > 1500",
  "context" : { "depth": 2000 },
  "context_name" : "well"
}

  - Response:

```
{
    "result": true,
    "context": null,
    "error": null
}
```

3. Evaluate an expression with a context, also return debugging information:

  - JSON Request Body:

{
  "expression": "result = well.depth > 1500",
  "context" : { "depth": 2000 },
  "context_name" : "well",
  "trace_on" : true
}

  - Response:

```
{
    "result": true,
    "context": "Exec Context: {\"π\": Scalar(Rational(3.141592653589793)), \"φ\": Scalar(Rational(1.618033988749895)), \"e\": Scalar(Rational(2.718281828459045)), \"well\": Object(  {\n    depth: Scalar(Integer(2000))\n  }\n), \"PI\": Scalar(Rational(3.141592653589793)), \"PHI\": Scalar(Rational(1.618033988749895)), \"result\": Scalar(Boolean(true))}",
    "error": null
}
```

This case currently also logs the whole process of executing the expression to the console. (Eventually this will go to a log file.)