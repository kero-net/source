# Luau Functions

This document owns language-wide function guidance.

Function names follow `/mnt/data/documents/.scope/knowledge/language/luau/language/naming.md`.

Project-owned receiver methods follow the selected structure. OOP receiver
methods are owned by `/mnt/data/documents/.scope/knowledge/language/luau/structures/oop/objects.md`.

Prefer local helper functions for private behavior. Do not create table-private
methods just to simulate privacy.

## Basic Helper

```luau
local function normalizeText(text: string?): string
	return text or ""
end
```

## Return Function

Use a return-function module when the file owns one operation:

```luau
return function(value: number, minimum: number, maximum: number): number
	return math.clamp(value, minimum, maximum)
end
```
