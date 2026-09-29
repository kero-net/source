# Function Modules

A return-function module owns one operation.

Use it when naming the file already names the concept clearly enough. Adding a
module table would only repeat the same name around one function.

Required:

- path label and `--!strict`
- returned function

Usually absent:

- module identity table
- exported methods
- constructor
- lifecycle headers

```luau
-- src/app/Clamp.luau
--!strict

return function(value: number, minimum: number, maximum: number): number
	return math.clamp(value, minimum, maximum)
end
```

Why this shape:

- the file has one reason to exist
- the function has no stored state
- there is no public surface to organize beyond the returned callable
- testing can focus on input/output behavior
