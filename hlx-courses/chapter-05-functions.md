# Chapter 5: Functions

You've been using `main()` since Chapter 1 - that's a function! Now you'll learn to create your own functions and make your code organized and reusable.

## What is a Function?

A **function** is a named block of code that does a specific job. Think of it like a recipe:
- Recipe name: "Make Pancakes"
- Inputs: flour, eggs, milk
- Steps: mix, pour, flip
- Output: pancakes

In code:
```hlx
fn make_pancakes(flour, eggs, milk) {
    // Steps to make pancakes
    return pancakes;
}
```

## Why Use Functions?

### 1. Avoid Repetition (DRY: Don't Repeat Yourself)

**Without functions:**
```hlx
program bad_example {
    fn main() {
        // Calculate area of rectangle 1
        let width1 = 5;
        let height1 = 3;
        let area1 = width1 * height1;
        print(area1);

        // Calculate area of rectangle 2
        let width2 = 8;
        let height2 = 4;
        let area2 = width2 * height2;
        print(area2);

        // Calculate area of rectangle 3
        let width3 = 6;
        let height3 = 2;
        let area3 = width3 * height3;
        print(area3);

        return 0;
    }
}
```

**With functions:**
```hlx
program good_example {
    fn calculate_area(width, height) {
        return width * height;
    }

    fn main() {
        let area1 = calculate_area(5, 3);
        print(area1);

        let area2 = calculate_area(8, 4);
        print(area2);

        let area3 = calculate_area(6, 2);
        print(area3);

        return 0;
    }
}
```

**Output (both programs):**
```
15
32
12
0
```

Much cleaner! And if you need to change how area is calculated, you only change one place.

### 2. Organize Code

Functions let you break big problems into small pieces:
```hlx
fn calculate_tax(price) { ... }
fn calculate_shipping(weight) { ... }
fn calculate_total(price, weight) { ... }
```

Each function has one job and does it well.

## Creating Your First Function

```hlx
program first_function {
    fn greet() {
        print("Hello from a function!");
    }

    fn main() {
        greet();  // Call the function
        greet();  // Call it again!
        return 0;
    }
}
```

**Output:**
```
"Hello from a function!"
"Hello from a function!"
0
```

**Parts of a function:**
- **`fn`** - Keyword meaning "function"
- **`greet`** - Function name (you choose this)
- **`()`** - Parentheses (hold parameters, empty for now)
- **`{}`** - Curly braces containing the code

**Calling a function:**
- Write the function name
- Add parentheses: `greet()`
- Add semicolon: `greet();`

## Functions with Parameters

**Parameters** are inputs to a function:

```hlx
program greeting {
    fn greet(name) {
        print("Hello,");
        print(name);
    }

    fn main() {
        greet("Alice");
        greet("Bob");
        greet("Charlie");
        return 0;
    }
}
```

**Output:**
```
"Hello,"
"Alice"
"Hello,"
"Bob"
"Hello,"
"Charlie"
0
```

**How it works:**
1. `greet("Alice")` calls the function
2. Inside the function, `name` becomes "Alice"
3. Function prints "Hello," and "Alice"
4. Same for Bob and Charlie

## Multiple Parameters

Functions can take many inputs:

```hlx
program calculator {
    fn add(a, b) {
        let sum = a + b;
        print("Sum:");
        print(sum);
    }

    fn multiply(a, b) {
        let product = a * b;
        print("Product:");
        print(product);
    }

    fn main() {
        add(5, 3);
        multiply(5, 3);
        return 0;
    }
}
```

**Output:**
```
"Sum:"
8
"Product:"
15
0
```

**Parameter order matters!**
```hlx
fn subtract(a, b) {
    return a - b;  // Order matters: a minus b
}

subtract(10, 3);  // Returns 7
subtract(3, 10);  // Returns -7 (different!)
```

## Returning Values

So far, our functions just print. But functions can **return** values:

```hlx
program return_demo {
    fn add(a, b) {
        return a + b;  // Give back the result
    }

    fn main() {
        let result = add(10, 5);
        print("The result is:");
        print(result);
        return 0;
    }
}
```

**Output:**
```
"The result is:"
15
0
```

**How return works:**
1. `add(10, 5)` is called
2. Inside function: `a = 10`, `b = 5`
3. Calculate: `10 + 5 = 15`
4. `return 15` sends 15 back
5. `result` gets the value 15

## Return Stops the Function

When you `return`, the function exits immediately:

```hlx
program early_return {
    fn check_age(age) {
        if (age < 18) {
            return 0;  // Too young, return 0
        }

        if (age > 65) {
            return 2;  // Senior, return 2
        }

        return 1;  // Adult, return 1
    }

    fn main() {
        let status1 = check_age(15);
        let status2 = check_age(30);
        let status3 = check_age(70);

        print("Status 1:");
        print(status1);
        print("Status 2:");
        print(status2);
        print("Status 3:");
        print(status3);

        return 0;
    }
}
```

**Output:**
```
"Status 1:"
0
"Status 2:"
1
"Status 3:"
2
0
```

## Practical Example: Temperature Converter

```hlx
program temperature_converter {
    fn celsius_to_fahrenheit(celsius) {
        let fahrenheit = (celsius * 9 / 5) + 32;
        return fahrenheit;
    }

    fn fahrenheit_to_celsius(fahrenheit) {
        let celsius = (fahrenheit - 32) * 5 / 9;
        return celsius;
    }

    fn main() {
        print("Converting 0°C to Fahrenheit:");
        let f1 = celsius_to_fahrenheit(0);
        print(f1);

        print("Converting 100°C to Fahrenheit:");
        let f2 = celsius_to_fahrenheit(100);
        print(f2);

        print("Converting 32°F to Celsius:");
        let c1 = fahrenheit_to_celsius(32);
        print(c1);

        print("Converting 212°F to Celsius:");
        let c2 = fahrenheit_to_celsius(212);
        print(c2);

        return 0;
    }
}
```

**Output:**
```
"Converting 0°C to Fahrenheit:"
32
"Converting 100°C to Fahrenheit:"
212
"Converting 32°F to Celsius:"
0
"Converting 212°F to Celsius:"
100
0
```

## Functions Calling Functions

Functions can call other functions:

```hlx
program function_chain {
    fn double(n) {
        return n * 2;
    }

    fn triple(n) {
        return n * 3;
    }

    fn double_then_triple(n) {
        let doubled = double(n);
        let tripled = triple(doubled);
        return tripled;
    }

    fn main() {
        let result = double_then_triple(5);
        // 5 → double → 10 → triple → 30
        print(result);
        return 0;
    }
}
```

**Output:**
```
30
0
```

## Scope: Where Variables Live

Variables inside a function only exist inside that function:

```hlx
program scope_demo {
    fn calculate() {
        let x = 10;  // x only exists here
        print(x);
    }

    fn main() {
        let y = 20;  // y only exists here

        calculate();

        // print(x);  // ✗ Error! x doesn't exist in main
        print(y);     // ✓ y exists here

        return 0;
    }
}
```

**Output:**
```
10
20
0
```

**Important rules:**
- Variables in `main()` can't be used in other functions
- Variables in a function can't be used in `main()`
- Each function has its own "world" of variables
- Use parameters to pass values between functions
- Use return to send values back

## Practice: Exercise 1 - Max Function

Write a function that returns the larger of two numbers:

```hlx
fn max(a, b) {
    // Your code here
}
```

Test it:
```hlx
print(max(10, 5));   // Should print 10
print(max(3, 8));    // Should print 8
print(max(7, 7));    // Should print 7
```

<details>
<summary>Click to see solution</summary>

```hlx
program max_function {
    fn max(a, b) {
        if (a > b) {
            return a;
        } else {
            return b;
        }
    }

    fn main() {
        print(max(10, 5));
        print(max(3, 8));
        print(max(7, 7));
        return 0;
    }
}
```

**Shorter version:**
```hlx
fn max(a, b) {
    if (a > b) {
        return a;
    }
    return b;
}
```
</details>

## Practice: Exercise 2 - Is Even

Write a function that returns 1 if a number is even, 0 if odd:

```hlx
fn is_even(n) {
    // Your code here
}
```

Test it:
```hlx
print(is_even(4));   // Should print 1
print(is_even(7));   // Should print 0
print(is_even(0));   // Should print 1
```

<details>
<summary>Click to see solution</summary>

```hlx
program is_even_function {
    fn is_even(n) {
        if (n % 2 == 0) {
            return 1;
        } else {
            return 0;
        }
    }

    fn main() {
        print(is_even(4));
        print(is_even(7));
        print(is_even(0));
        return 0;
    }
}
```

**Shorter version:**
```hlx
fn is_even(n) {
    if (n % 2 == 0) {
        return 1;
    }
    return 0;
}
```
</details>

## Practice: Exercise 3 - Power Function

Write a function that calculates `base` raised to `exponent`:
- `power(2, 3)` = 2³ = 8
- `power(5, 2)` = 5² = 25
- `power(10, 0)` = 10⁰ = 1

**Hint:** Use a loop to multiply `base` by itself `exponent` times.

<details>
<summary>Click to see solution</summary>

```hlx
program power_function {
    fn power(base, exponent) {
        let result = 1;
        let i = 0;

        loop (i < exponent, 100) {
            result = result * base;
            i = i + 1;
        }

        return result;
    }

    fn main() {
        print("2^3 =");
        print(power(2, 3));

        print("5^2 =");
        print(power(5, 2));

        print("10^0 =");
        print(power(10, 0));

        return 0;
    }
}
```

**Output:**
```
"2^3 ="
8
"5^2 ="
25
"10^0 ="
1
0
```
</details>

## Practice: Exercise 4 - Factorial Function

Convert your factorial loop from Chapter 4 into a function:

```hlx
fn factorial(n) {
    // Calculate n!
}
```

Test it:
```hlx
print(factorial(5));   // 120
print(factorial(3));   // 6
print(factorial(0));   // 1
```

<details>
<summary>Click to see solution</summary>

```hlx
program factorial_function {
    fn factorial(n) {
        let result = 1;
        let i = n;

        loop (i > 0, 100) {
            result = result * i;
            i = i - 1;
        }

        return result;
    }

    fn main() {
        print("5! =");
        print(factorial(5));

        print("3! =");
        print(factorial(3));

        print("0! =");
        print(factorial(0));

        return 0;
    }
}
```

**Output:**
```
"5! ="
120
"3! ="
6
"0! ="
1
0
```
</details>

## Common Mistakes

### Mistake 1: Forgetting to Return
```hlx
fn add(a, b) {
    let sum = a + b;
    // ✗ Forgot to return!
}

let result = add(5, 3);
// result is undefined/null
```

**Fix:**
```hlx
fn add(a, b) {
    let sum = a + b;
    return sum;  // ✓
}
```

### Mistake 2: Using Wrong Variable Names
```hlx
fn greet(name) {
    print(username);  // ✗ Parameter is called 'name', not 'username'
}
```

**Fix:**
```hlx
fn greet(name) {
    print(name);  // ✓ Use the parameter name
}
```

### Mistake 3: Wrong Number of Arguments
```hlx
fn add(a, b) {
    return a + b;
}

add(5);  // ✗ Missing second argument!
add(5, 3, 7);  // ✗ Too many arguments!
```

**Fix:**
```hlx
add(5, 3);  // ✓ Correct number of arguments
```

### Mistake 4: Calling Before Defining
```hlx
program bad_order {
    fn main() {
        greet();  // ✗ Function not defined yet!
        return 0;
    }

    fn greet() {
        print("Hello");
    }
}
```

In HLX, you can actually call functions before defining them (as long as they're defined somewhere), but it's good practice to define functions before `main()`.

**Better:**
```hlx
program good_order {
    fn greet() {
        print("Hello");
    }

    fn main() {
        greet();  // ✓ Function already defined
        return 0;
    }
}
```

## Challenge: Circle Calculator

Create a program with these functions:
1. `circle_area(radius)` - Returns area (π × r²)
2. `circle_circumference(radius)` - Returns circumference (2 × π × r)
3. `circle_diameter(radius)` - Returns diameter (2 × r)

Use π ≈ 3.14 (or 314/100 for integer math).

Test with radius = 5.

<details>
<summary>Click to see solution</summary>

```hlx
program circle_calculator {
    fn circle_area(radius) {
        // Area = π × r²
        let pi = 314;  // 3.14 × 100
        let area = (pi * radius * radius) / 100;
        return area;
    }

    fn circle_circumference(radius) {
        // Circumference = 2 × π × r
        let pi = 314;
        let circ = (2 * pi * radius) / 100;
        return circ;
    }

    fn circle_diameter(radius) {
        // Diameter = 2 × r
        return 2 * radius;
    }

    fn main() {
        let r = 5;

        print("Circle with radius");
        print(r);
        print("");

        print("Diameter:");
        print(circle_diameter(r));

        print("Circumference:");
        print(circle_circumference(r));

        print("Area:");
        print(circle_area(r));

        return 0;
    }
}
```

**Output:**
```
"Circle with radius"
5
""
"Diameter:"
10
"Circumference:"
31
"Area:"
78
0
```

Note: Results are approximate due to integer division.
</details>

## What You Learned

✅ What functions are and why they're useful
✅ Creating functions with `fn`
✅ Parameters (inputs to functions)
✅ Return values (outputs from functions)
✅ Calling functions
✅ Scope (where variables exist)
✅ Functions calling other functions
✅ Common mistakes and how to avoid them

## Next Steps

In **Chapter 6**, you'll learn about **arrays** - storing multiple values in one variable!

Preview:
```hlx
let numbers = [10, 20, 30, 40, 50];
print(numbers[0]);  // Prints 10
print(numbers[4]);  // Prints 50
```

Arrays unlock powerful new programming patterns! 📦

---

**Self-check:** Can you write:
1. A function that takes two numbers and returns their average
2. A function that takes a number and returns 1 if it's positive, -1 if negative, 0 if zero
3. A function that uses a loop to sum numbers from 1 to n

If yes, you're ready for Chapter 6!
