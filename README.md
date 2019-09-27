# Shy - A Shunting Yard Rules Engine

This is the beginnings of an IOT Rules processing and alerting system. The goals are:

  - **Simple syntax**. Engineers can write rule expressions using a simple infix notation.
  - **Simple file format**. Engineers store the rules in simple text files (such as CSV or YAML)
  - **External rules files**. Application loads the rules from these external files, so recompilation is not necessary and formulas are not embedded in the code where they are inaccessible to non-programmers.
  - **Low memory**. Rules engine will require little memory...
  - **Real-time** ... and will be fast enough to use for real-time applications (thus must not use a garbage collector, a feature of **Rust**).
  - **Cached rules**. Compiled rules can be cached and reused.
  - **Business Context**. The rules can read and write from variables stored in business objects (the execution context) that conform to a simple interface.
  - **Sensor statistics**. Streams of measurement data can be processed efficiently, with small amounts of memory required to gather basic statistics like the mean, median, min, max, various quantiles, standard deviation, etc. This requires the use of **frugal streaming** algorithms. 
  - **Off-line mode**. Since sensors may only be connected to the Internet sporadically and lack enough memory to store data for several days, low memory usage is critical!
  - **Sensor rules**. The rules can access the sensor statistics to make judgments.
  - **Actions**. Passing rules can trigger the execution of alerting actions.
  - **Remote update**. If the application is running on an edge device, new rules can be sent to it remotely and hot-swapped with the previous rules.

The following parts are in working order:

  - Rule lexical analysis, parsing, and compilation
  - Rule execution
  - Rule caching
  - Creation of Context objects

For an overview of the rule syntax and how to use the REPL to test expressions, see this file: 

[Shy Rule Syntax and Usage](./src/README.md)