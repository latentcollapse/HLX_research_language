# Chapter 2: Variables and Math

In Chapter 1, you learned to make the computer print messages. Now you'll learn to make it **remember things** and **do calculations**.

## What is a Variable?

A **variable** is like a box with a label. You can:
1. Put something in the box
2. Look at what's in the box
3. Replace what's in the box

In programming, variables store information so you can use it later.

## Your First Variable

```hlx
program first_variable {
    fn main() {
        let age = 25;
        print(age);
        return 0;
    }
}
```

**Output:**
```
25
0
```

Let's break it down:
- **let** - Means "create a new variable"
- **age** - The name of the variable (the box's label)
- **=** - Means "store this value"
- **25** - The value to store
- **;** - End of instruction

## Rules for Variable Names

You can name variables almost anything, but:

**✓ Good names:**
- `age`
- `my_score`
- `player_health`
- `number_of_cookies`

**✗ Bad names:**
- `123age` (can't start with a number)
- `my-score` (no dashes, use underscore instead)
- `let` (can't use special words like let, if, return)
- `player health` (no spaces!)

**Best practice:** Use descriptive names that explain what the variable stores.

## Doing Math

Computers are great at math! Let's try it:

```hlx
program calculator {
    fn main() {
        let x = 10;
        let y = 5;
        let sum = x + y;
        print("The sum is:");
        print(sum);
        return 0;
    }
}
```

**Output:**
```
"The sum is:"
15
0
```

### Math Operations

HLX can do all the basic math:

```hlx
program math_demo {
    fn main() {
        let a = 10;
        let b = 3;

        let sum = a + b;        // Addition: 13
        let difference = a - b;  // Subtraction: 7
        let product = a * b;     // Multiplication: 30
        let quotient = a / b;    // Division: 3 (integer division)

        print("Sum:");
        print(sum);
        print("Difference:");
        print(difference);
        print("Product:");
        print(product);
        print("Quotient:");
        print(quotient);

        return 0;
    }
}
```

**Output:**
```
"Sum:"
13
"Difference:"
7
"Product:"
30
"Quotient:"
3
0
```

### Important: Order of Operations

**Remember PEMDAS from math class?** Computers use it too!
- **P**arentheses first
- **E**xponents (powers)
- **M**ultiplication and **D**ivision (left to right)
- **A**ddition and **S**ubtraction (left to right)

```hlx
let result = 2 + 3 * 4;  // Result: 14 (not 20!)
// 3 * 4 happens first (12), then + 2 (14)
```

**Use parentheses to be clear:**
```hlx
let result = (2 + 3) * 4;  // Result: 20
// Parentheses force addition first: (5) * 4 = 20
```

## Changing Variables

Variables can change their value:

```hlx
program counter {
    fn main() {
        let count = 0;
        print("Count is:");
        print(count);

        count = count + 1;  // Add 1 to count
        print("Count is now:");
        print(count);

        count = count + 1;  // Add 1 again
        print("Count is now:");
        print(count);

        return 0;
    }
}
```

**Output:**
```
"Count is:"
0
"Count is now:"
1
"Count is now:"
2
0
```

**How `count = count + 1` works:**
1. Look at current value of `count` (0)
2. Add 1 to it (0 + 1 = 1)
3. Store the result back in `count`

## Working with Text (Strings)

Variables can store text too!

```hlx
program names {
    fn main() {
        let first_name = "Alice";
        let last_name = "Smith";

        print("First name:");
        print(first_name);
        print("Last name:");
        print(last_name);

        return 0;
    }
}
```

**Output:**
```
"First name:"
"Alice"
"Last name:"
"Smith"
0
```

## Practice: Exercise 1 - Age Calculator

Write a program that:
1. Stores your current age in a variable
2. Calculates your age next year
3. Calculates your age in 5 years
4. Prints all three ages

<details>
<summary>Click to see solution</summary>

```hlx
program age_calculator {
    fn main() {
        let current_age = 25;
        let next_year = current_age + 1;
        let five_years = current_age + 5;

        print("Current age:");
        print(current_age);
        print("Age next year:");
        print(next_year);
        print("Age in 5 years:");
        print(five_years);

        return 0;
    }
}
```

**Output:**
```
"Current age:"
25
"Age next year:"
26
"Age in 5 years:"
30
0
```
</details>

## Practice: Exercise 2 - Rectangle Area

Write a program that calculates the area of a rectangle:
- Width: 12
- Height: 8
- Area = width × height

<details>
<summary>Click to see solution</summary>

```hlx
program rectangle {
    fn main() {
        let width = 12;
        let height = 8;
        let area = width * height;

        print("Rectangle dimensions:");
        print("Width:");
        print(width);
        print("Height:");
        print(height);
        print("Area:");
        print(area);

        return 0;
    }
}
```

**Output:**
```
"Rectangle dimensions:"
"Width:"
12
"Height:"
8
"Area:"
96
0
```
</details>

## Practice: Exercise 3 - Shopping Cart

You're buying items online:
- Item 1 costs $15
- Item 2 costs $23
- Item 3 costs $8
- Shipping costs $5

Calculate the total cost.

<details>
<summary>Click to see solution</summary>

```hlx
program shopping {
    fn main() {
        let item1 = 15;
        let item2 = 23;
        let item3 = 8;
        let shipping = 5;

        let subtotal = item1 + item2 + item3;
        let total = subtotal + shipping;

        print("Item 1: $");
        print(item1);
        print("Item 2: $");
        print(item2);
        print("Item 3: $");
        print(item3);
        print("Subtotal: $");
        print(subtotal);
        print("Shipping: $");
        print(shipping);
        print("Total: $");
        print(total);

        return 0;
    }
}
```

**Output:**
```
"Item 1: $"
15
"Item 2: $"
23
"Item 3: $"
8
"Subtotal: $"
46
"Shipping: $"
5
"Total: $"
51
0
```
</details>

## Numbers vs. Text

**Important difference:**

```hlx
program numbers_vs_text {
    fn main() {
        let number = 42;      // This is a number
        let text = "42";      // This is text

        print(number);        // Shows: 42
        print(text);          // Shows: "42"

        let doubled = number * 2;  // This works: 84
        // let doubled = text * 2;  // This DOESN'T work!

        print(doubled);

        return 0;
    }
}
```

**Rule:** You can do math with numbers, but not with text (even if the text looks like a number).

## Common Mistakes

### Mistake 1: Forgot `let`
```hlx
age = 25;  // ✗ Won't work
```
**Fix:**
```hlx
let age = 25;  // ✓ Use 'let' for new variables
```

### Mistake 2: Using Variable Before Creating It
```hlx
print(score);  // ✗ What is 'score'?
let score = 100;
```
**Fix:** Create variable first, use it second:
```hlx
let score = 100;
print(score);  // ✓ Now it exists
```

### Mistake 3: Confusing Math Order
```hlx
let result = 10 - 5 + 2;  // Is it 3 or 7?
```
It's 7, because: (10 - 5) + 2 = 5 + 2 = 7

**Better:** Use parentheses to be clear:
```hlx
let result = (10 - 5) + 2;  // Clearly 7
```

## Challenge: Temperature Converter

Write a program that converts temperature from Celsius to Fahrenheit:
- Formula: `F = (C * 9 / 5) + 32`
- Convert 25°C to Fahrenheit

Bonus: Convert 0°C and 100°C too!

<details>
<summary>Click to see solution</summary>

```hlx
program temperature {
    fn main() {
        print("Temperature Converter (Celsius to Fahrenheit)");

        let celsius1 = 0;
        let fahrenheit1 = (celsius1 * 9 / 5) + 32;
        print("0°C =");
        print(fahrenheit1);
        print("°F");

        let celsius2 = 25;
        let fahrenheit2 = (celsius2 * 9 / 5) + 32;
        print("25°C =");
        print(fahrenheit2);
        print("°F");

        let celsius3 = 100;
        let fahrenheit3 = (celsius3 * 9 / 5) + 32;
        print("100°C =");
        print(fahrenheit3);
        print("°F");

        return 0;
    }
}
```

**Output:**
```
"Temperature Converter (Celsius to Fahrenheit)"
"0°C ="
32
"°F"
"25°C ="
77
"°F"
"100°C ="
212
"°F"
0
```
</details>

## What You Learned

✅ Creating variables with `let`
✅ Naming variables properly
✅ Doing math (+, -, *, /)
✅ Order of operations (use parentheses!)
✅ Changing variable values
✅ Difference between numbers and text
✅ Using variables in calculations

## Next Steps

In **Chapter 3**, you'll learn how to make **decisions** in your code!

Preview:
```hlx
if (age >= 18) {
    print("You can vote!");
} else {
    print("Too young to vote");
}
```

Your programs will start getting really interesting! 🎯

---

**Before moving on:** Can you write a program from memory that:
1. Creates two number variables
2. Calculates their sum, difference, product, and quotient
3. Prints all four results

If you can do this, you're ready for Chapter 3!
