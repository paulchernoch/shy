# Shy Web Service Restful API

The Shy Web Service presents a **REST API** interface to its functionality. It holds a cache of **Rulesets**, which may be _added_, _retrieved_, _updated_, _deleted_, and _executed_.

The essential objects are:

  - **Ruleset** - A named list of compiled **Rules**.
  - **Rule** - A named, compiled **Expression**.
  - **Expression** - An unnamed formula which may be in the form of source text or compiled form.
  - **Context** - Data with properties. A Context may be used as the source of variables whose values are needed as inputs when evaluating Expressions and Rules. A Context may also serve as the target for results to be written.
  - **Service Cache** - Holds compiled **Rulesets**, which may be reused in subsequent calls.

## Starting the Web Service

Run the following command to start the Web Service on the default IP address of 127.0.0.1:8088:

```
    > cargo run service
```

To change the IP address and port:

```
    > cargo run service <ip-address> <port>
```

The logging middleware for the Web Service may be configured by setting two environment variables:

  - **RUST_LOG** - This defines which severity of messages to log and is specified by the `Actix` framework and defaults to `actix_web=info`. You can also set it to `actix_web=debug`, `actix_web=trace`, `actix_web=warn` or `actix_web=error`.
  - **RUST_LOG_FORMAT** - This defines the log message format. This environment variable is not specified by `Actix`, but the format string is. See here for a list of the valid format specifiers: https://docs.rs/actix-web/1.0.0/actix_web/middleware/struct.Logger.html
  
One useful feature for debugging the `RuleSet` execution is to trace the entire process of evaluating the rules.
The caller must set "trace_on" in the JSON request but also you must configure the service to enable trace messages to be shown.
Here is one way to accomplish that:

```
    > RUST_LOG="info,parser::expression=trace,actix_web=info" cargo run service
```
  
You must match targets in the trace!, debug!, info!, warn! and error! macros to set the logging level.

  - The relevant target for a detailed execution trace is `parser::expression`.
  - The target for info and warning messages in the route handlers is `service::routes`.
  - The target that Actix uses for its messages is `actix_web`.

Additionally, the **cargo.toml** file sets a compile time limit for the **log** level, and can distinguish between release and debug builds.
Settings made there cause the log messages to be compiled out of the executable, overruling the environment variable settings.

## REST Commands

1. **List Rulesets** - Get a list of the names of all **Rulesets** stored in the service cache. 
2. **Create Ruleset** - Add a **Ruleset** having the given name to the **service cache**, or completely replace the existing **Ruleset**.
3. **Get Ruleset** - Get a **Ruleset** from the service cache that has the given name.
4. **Delete Ruleset** - Delete the given **Ruleset** from the service cache.
5. **Execute Expression** - Compile and execute an **Expression** without a data **context**, bypassing the **service cache**. Return the result of the evaluation.
6. **Execute Expression with Context** - Compile and execute an **Expression** with a data **context**, bypassing the **service cache**.
7. **Execute Ruleset with Context** - Name a **Ruleset** expected to be in the **service cache** and supply a **context**.

## Endpoint Syntax

| Command                    | REST Syntax                    | Posted Data         |
| -------------------------- | ------------------------------ | ------------------- |
| List Rulesets              | GET /rulesets                  | N/A                 |
| List Rulesets for category | GET /rulesets?category={cat}   | N/A                 |
| Create Ruleset             | PUT /rulesets/{name}           | Ruleset             |
| Read Ruleset               | GET /rulesets/{name}           | N/A                 |
| Delete Ruleset             | DELETE /rulesets/{name}        | N/A                 |
| Execute Expression         | POST /expression/execute       | Expression, Context |
| Execute Ruleset            | POST /rulesets/{name}          | Context             |

NOTE: At this time, only these routes are supported: 

  - Index page: **GET /**
  - Expression tester: **POST /expression/execute**
  - List all RuleSets: **GET /rulesets**
  - List RuleSets for category: **GET /rulesets?category=name**
  - Create RuleSet: **PUT /rulesets/{name}**
  - Read RuleSet: **GET /rulesets/{name}**
  - Delete RuleSet: **DELETE /rulesets/{name}**
  - Execute RuleSet: **POST /rulesets/{name}**
  
The expression tester covers the cases **Execute Expression** and **Execute Expression with Context** from above.

## Examples

1. **Evaluate an expression without a context**:

_HTTP Command_:   POST /expression/execute
   
**JSON Request Body:**

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

2. **Evaluate an expression with a context**, but do not request for the updated context to be sent back:

_HTTP Command_:   POST /expression/execute

**JSON Request Body:**

```
{
  "expression": "result = well.depth > 1500",
  "context" : { "depth": 2000 },
  "context_name" : "well"
}
```

**Response:**

```
{
    "result": true,
    "context": null,
    "error": null
}
```

3. **Evaluate an expression with a context, request the updated context in the response**, and also log debugging information:

_HTTP Command_:   POST /expression/execute

**JSON Request Body:**

```
{
  "expression": "result = well.depth > 1500",
  "context" : { "depth": 2000 },
  "context_name" : "well",
  "return_context" : true,
  "trace_on" : true
}
```

**Response:**

```
{
    "result": true,
    "context": {
        "variables": {
            "e": 2.718281828459045,
            "well": {
                "depth": 2000
            },
            "π": 3.141592653589793,
            "PI": 3.141592653589793,
            "φ": 1.618033988749895,
            "result": true,
            "PHI": 1.618033988749895
        }
    },
    "error": null
}
```

This case currently logs the whole process of executing the expression to the console. 
(Eventually this should go to a log file.)

4. Add a RuleSet named "shopping_rules" to the cache.
   
_HTTP Command_:   **PUT /rulesets/shopping_rules**


**Request body:**

```
{
	"rule_source" : [ 
		"rule.name = \"is-it-way-bigger\";rule.description = \"Is it way too big to fit in my car or what?\";rule.category = \"shopping\";rule.id = 2005;size > 150" 
	]
}
```

5. List the names of all RuleSets present in the cache.

_HTTP Command_:   **GET /rulesets**

**Response:**

```
{
  "ruleset_count":2,
  "ruleset_names":["answer1","answer2"],
  "success":true,
  "error":null
}
```

6. Execute a `RuleSet`. For this example, we will first create a new `RuleSet`, then execute it.

_First HTTP Command_:   **PUT /rulesets/Decide-if-car-worth-buying**

**Request body:**

```
{
	"category" : "transportation",
	"ruleset_source" : "          rule.name = \"RuleSet header\"\n          rule.type = \"Property\";\n          ruleset.name = \"Decide-if-car-worth-buying\";\n          ruleset.context_name = \"car\";\n          ruleset.criteria = \"MajorityPass\";\n          ruleset.category = \"Test\";\n          applicable = false? ;\n\n          rule.name = \"car age\";\n          rule.type = \"Predicate\";\n          not_too_old = car.age < 8 || (car.age < 12 && car.make == \"Honda\");\n\n          rule.name = \"car price\";\n          rule.type = \"Predicate\";\n          good_price = min(50000 / car.age, 30000);\n          not_too_expensive = car.price < good_price;\n\n          rule.name = \"car miles driven\";\n          rule.type = \"Predicate\";\n          good_miles_driven = car.miles_driven < 100000 || (car.miles_driven < 150000 && car.make == \"Honda\");\n          \n          rule.name = \"car accidents\";\n          rule.type = \"Predicate\";\n          not_too_many_accidents = car.accidents == 0 || (car.accidents <= 1 && car.make == \"BMW\");"
}
```

The above request looks ugly, but that is because you need to escape newlines and double quotes in JSON strings. This is what the unescaped formula would look like to a user as they define it in a front-end application, before it is converted into JSON and escaped: ):

```
          rule.name = "RuleSet header";
          rule.type = "Property";
          ruleset.name = "Decide-if-car-worth-buying";
          ruleset.context_name = "car";
          ruleset.criteria = "MajorityPass";
          ruleset.category = "Test";
          applicable = false? ;

          rule.name = "car age";
          rule.type = "Predicate";
          not_too_old = car.age < 8 || (car.age < 12 && car.make == "Honda");

          rule.name = "car price";
          rule.type = "Predicate";
          good_price = min(50000 / car.age, 30000);
          not_too_expensive = car.price < good_price;

          rule.name = "car miles driven";
          rule.type = "Predicate";
          good_miles_driven = car.miles_driven < 100000 || (car.miles_driven < 150000 && car.make == "Honda");
          
          rule.name = "car accidents";
          rule.type = "Predicate";
          not_too_many_accidents = car.accidents == 0 || (car.accidents <= 1 && car.make == "BMW");"
```

Much more readable, no? Observe the structure of a `RuleSet`.

  - Rules can span multiple lines; they are separated by a blank lines.
  - The first rule defines the `RuleSet` itself. It is marked as` rule.type = "Property"`, meaning it does not contribute to the pass/fail decision. It just defines needed properties, namely metadata about the `RuleSet`. It also defines the evaluation criteria. `ruleset.criteria` may equal any of these values:

       * **NeverPass** - When we have decision trees implemented, this can be used for a catchall `RuleSet` for failing decision paths.
       * **AllPass** - If you want to combine the results of all the `Rules` in the `RuleSet` together using a logical _AND_, use this to require that all of them pass.
       * **MajorityPass** - Use this if you want over half the `Rules` to pass, but permit a few to fail.
       * **AnyPass** - If you want to combine the results of all the `Rules` in the `RuleSet` together using a logical _OR_, use this to require that at least one of them pass.
       * **LastPasses** - If you wish to specify in detail through coordinating `Rules` whether the `RuleSet` should pass and intend to make the final `Rule` decide the result, use this. All `Rules` will be run, but only the result of the final `Rule` will be used to decide if the `RuleSet` passes.
       * **AlwaysPass** - When we have decision trees implemented, this can be used for a catchall `RuleSet` for successful decision paths. ALternately, it can be used when the goal is to return a value other than true/false, such as for a cost model formula.

_Now we execute the ruleset._

_Second HTTP Command_:   **POST /rulesets/Decide-if-car-worth-buying**

**Request body:**

```
{
    "context" : {
       "make" : "Honda",
       "age" : 10,
       "miles_driven" : 120000,
       "price" : 4750,
       "accidents" : 2
    },
    "context_name" : "car",
    "return_context" : true,
    "trace_on" : true
}

```

How do we understand the execute request? The context is the input variables available to your formulas. `context_name` is the first part of the variable reference expected in the expressions. So for example, to reference `miles_driven` in a formula, we prefix the reference with the `context_name`, which gives us `car.miles_driven`.

`trace_on` turns on printing of extensive trace statements to the service log.

`return_context` should be `true` if you want the intermediate values stored in the context during the rule execution to be returned, which can help you debug your rules. If `false`, you only get the pass or failure returned.