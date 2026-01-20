# HLX Tier 2 Builtins Reference

These are the core string, array, and type conversion operations available in HLX. They're essential for writing practical programs and form the foundation for higher-level constructs.

## String Operations

### Basic String Functions

```hlx
strlen(s: String) -> i64
```
Returns the length of a string in bytes.
```hlx
let greeting = "hello";
let len = strlen(greeting);  // Returns 5
```

---

```hlx
concat(s1: String, s2: String) -> String
```
Concatenates two strings.
```hlx
let full = concat("Hello", "World");  // "HelloWorld"
```

---

```hlx
substring(s: String, start: i64, length: i64) -> String
```
Extracts a substring starting at `start` index for `length` characters.
```hlx
let text = "HelloWorld";
let part = substring(text, 0, 5);  // "Hello"
```

---

```hlx
trim(s: String) -> String
```
Removes leading and trailing whitespace.
```hlx
let padded = "  hello  ";
let clean = trim(padded);  // "hello"
```

---

### Case Conversion

```hlx
to_upper(s: String) -> String
```
Converts string to uppercase.
```hlx
let upper = to_upper("hello");  // "HELLO"
```

---

```hlx
to_lower(s: String) -> String
```
Converts string to lowercase.
```hlx
let lower = to_lower("HELLO");  // "hello"
```

---

### String Searching

```hlx
starts_with(s: String, prefix: String) -> i64
```
Returns 1 if string starts with prefix, 0 otherwise.
```hlx
if (starts_with("HelloWorld", "Hello") == 1) {
    print("Starts with Hello!\n");
}
```

---

```hlx
ends_with(s: String, suffix: String) -> i64
```
Returns 1 if string ends with suffix, 0 otherwise.
```hlx
if (ends_with("HelloWorld", "World") == 1) {
    print("Ends with World!\n");
}
```

---

```hlx
contains(s: String, substring: String) -> i64
```
Returns 1 if string contains substring, 0 otherwise.
```hlx
if (contains("HelloWorld", "low") == 1) {
    print("Contains 'low'!\n");
}
```

---

```hlx
strcmp(s1: String, s2: String) -> i64
```
Compares two strings. Returns:
- 0 if equal
- Negative if s1 < s2
- Positive if s1 > s2
```hlx
if (strcmp(greeting, "hello") == 0) {
    print("Strings are equal!\n");
}
```

---

### String Manipulation

```hlx
split(s: String, delimiter: String) -> [i64]
```
Splits string by delimiter. Returns array of string pointers.
```hlx
let csv = "apple,banana,cherry";
let items = split(csv, ",");  // Array with 3 elements
```

---

```hlx
replace(s: String, old: String, new: String) -> String
```
Replaces first occurrence of `old` with `new`.
```hlx
let result = replace("hello world", "world", "universe");  // "hello universe"
```

---

```hlx
replace_first(s: String, old: String, new: String) -> String
```
Same as `replace` - replaces first occurrence.

---

```hlx
char_at(s: String, index: i64) -> String
```
Returns character at given index as a single-character string.
```hlx
let c = char_at("hello", 0);  // "h"
```

---

```hlx
char_code(c: String) -> i64
```
Returns ASCII code of first character in string.
```hlx
let code = char_code("A");  // 65
```

---

## Array Operations

### Basic Array Functions

```hlx
array_len(arr: [i64]) -> i64
```
Returns the length of an array.
```hlx
let numbers = [1, 2, 3, 4, 5];
let len = array_len(numbers);  // 5
```

---

```hlx
get_at(arr: [i64], index: i64) -> i64
```
Gets element at index (0-based).
```hlx
let first = get_at(numbers, 0);  // 1
```

---

```hlx
set_at(arr: [i64], index: i64, value: i64) -> [i64]
```
Returns new array with element at index replaced.
```hlx
let modified = set_at(numbers, 0, 99);  // [99, 2, 3, 4, 5]
```

---

```hlx
push(arr: [i64], value: i64) -> [i64]
```
Returns new array with value appended.
```hlx
let extended = push(numbers, 6);  // [1, 2, 3, 4, 5, 6]
```

---

### Array Slicing & Extraction

```hlx
slice(arr: [i64], start: i64, length: i64) -> [i64]
```
Extracts a slice starting at `start` index for `length` elements.
```hlx
let slice = slice(numbers, 1, 3);  // [2, 3, 4]
```

---

```hlx
arr_pop(arr: [i64]) -> [i64]
```
Returns new array with last element removed.
```hlx
let shortened = arr_pop(numbers);  // [1, 2, 3, 4]
```

---

### Array Transformation

```hlx
reverse(arr: [i64]) -> [i64]
```
Returns reversed array.
```hlx
let rev = reverse(numbers);  // [5, 4, 3, 2, 1]
```

---

```hlx
sort(arr: [i64]) -> [i64]
```
Returns sorted array in ascending order.
```hlx
let unsorted = [5, 2, 8, 1, 9];
let sorted = sort(unsorted);  // [1, 2, 5, 8, 9]
```

---

```hlx
unique(arr: [i64]) -> [i64]
```
Returns array with duplicate elements removed.
```hlx
let with_dups = [1, 2, 2, 3, 3, 3];
let unique_arr = unique(with_dups);  // [1, 2, 3]
```

---

### Array Creation & Merging

```hlx
range(start: i64, end: i64) -> [i64]
```
Creates array of integers from `start` to `end` (exclusive).
```hlx
let nums = range(0, 5);  // [0, 1, 2, 3, 4]
```

---

```hlx
arr_concat(arr1: [i64], arr2: [i64]) -> [i64]
```
Concatenates two arrays.
```hlx
let first = [1, 2, 3];
let second = [4, 5, 6];
let combined = arr_concat(first, second);  // [1, 2, 3, 4, 5, 6]
```

---

## Type Checking & Conversion

### Type Inspection

```hlx
type(value) -> String
```
Returns the type of a value as a string.
- `"i64"` for integers
- `"string"` for strings
- `"array"` for arrays
- `"null"` for null
```hlx
let t1 = type(42);        // "i64"
let t2 = type("hello");   // "string"
let t3 = type([1, 2, 3]); // "array"
```

---

### Type Conversion

```hlx
to_int(s: String) -> i64
```
Converts string to integer.
```hlx
let num = to_int("42");  // 42
```

---

```hlx
to_float(value) -> f64
```
Converts value to floating-point number.
```hlx
let f = to_float(42);  // 42.0
```

---

```hlx
bool(value: i64) -> i64
```
Converts integer to boolean (0 → 0, non-zero → 1).
```hlx
let b1 = bool(0);   // 0 (false)
let b2 = bool(42);  // 1 (true)
```

---

## Utility Functions from `hlx_builtins` Module

The `hlx_bootstrap/builtins.hlx` module provides higher-level utilities built on Tier 2 operations:

### String Utilities

```hlx
str_index_of(s: String, search: String) -> i64
```
Finds index of substring, returns -1 if not found.

---

```hlx
str_count_char(s: String, c: String) -> i64
```
Counts occurrences of a character.

---

```hlx
str_pad_left_char(s: String, width: i64, c: String) -> String
```
Pads string on left with character.

---

```hlx
str_join(parts: [i64], separator: String) -> String
```
Joins array of strings with separator.

---

### Array Utilities

```hlx
arr_index_of(arr: [i64], value: i64) -> i64
```
Finds index of value, returns -1 if not found.

---

```hlx
arr_contains(arr: [i64], value: i64) -> i64
```
Returns 1 if array contains value, 0 otherwise.

---

```hlx
arr_min(arr: [i64]) -> i64
```
Returns minimum value in array.

---

```hlx
arr_max(arr: [i64]) -> i64
```
Returns maximum value in array.

---

```hlx
arr_sum(arr: [i64]) -> i64
```
Returns sum of all elements.

---

```hlx
print_array(arr: [i64]) -> i64
```
Pretty-prints array to console.

---

## Complete Example

```hlx
program string_array_demo {
    fn main() -> i64 {
        // String manipulation
        let greeting = "  Hello, World!  ";
        let clean = trim(greeting);
        let upper = to_upper(clean);
        print(upper);  // "HELLO, WORLD!"
        print("\n");

        // Array operations
        let numbers = range(1, 6);  // [1, 2, 3, 4, 5]
        let sorted = sort([5, 2, 8, 1, 9]);
        let sum = arr_sum(sorted);

        print("Sum: ");
        print_int(sum);  // Sum: 25
        print("\n");

        // Combining operations
        let text = "apple,banana,cherry";
        let fruits = split(text, ",");
        let count = array_len(fruits);

        print("Fruit count: ");
        print_int(count);  // Fruit count: 3
        print("\n");

        return 0;
    }
}
```

---

## Performance Notes

- **String operations**: O(n) where n is string length
- **Array operations**: Generally O(n) or O(n log n) for sort
- **Type checking**: O(1)
- **All operations are pure** - they return new values rather than modifying in place

---

## Next Tier (Tier 3)

After mastering Tier 2, explore:
- **Math functions**: `sqrt`, `pow`, `sin`, `cos`, `abs`, `min`, `max`
- **File I/O**: `read_file`, `write_file`, `read_json`, `write_json`
- **Advanced arrays**: `zip`, `unzip`, `chunk`, `take`, `drop`
- **Tensor operations**: `alloc_tensor`, `shape`, `reshape`, etc.
