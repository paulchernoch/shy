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

NOTE: At this time, only these routes are supported: 

  - Index page: **GET /**
  - Expression tester: **POST /expression/execute**
  - List all RuleSets: **GET /rulesets**
  - List RuleSets for category: **GET /rulesets?category=name**
  - Add RuleSet: **PUT /rulesets/{name}**
  
This covers the cases **Execute Expression** and **Execute Expression with Context** from above.

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
