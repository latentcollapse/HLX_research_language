# Chapter 3: Making Decisions

So far, your programs do the same thing every time. Now you'll learn to make programs that **make choices** based on conditions.

## The `if` Statement

An `if` statement lets your program choose what to do:

```hlx
program voting_age {
    fn main() {
        let age = 20;

        if (age >= 18) {
            print("You can vote!");
        }

        return 0;
    }
}
```

**Output:**
```
"You can vote!"
0
```

**How it works:**
1. Check the condition `age >= 18`
2. If TRUE, run the code inside the `{ }`
3. If FALSE, skip that code

Try changing `age` to 15 and run it again - nothing prints!

## Adding an `else`

What if you want to do something when the condition is FALSE?

```hlx
program voting_check {
    fn main() {
        let age = 15;

        if (age >= 18) {
            print("You can vote!");
        } else {
            print("Too young to vote");
        }

        return 0;
    }
}
```

**Output:**
```
"Too young to vote"
0
```

**How it works:**
1. Check `age >= 18`
2. If TRUE → run first block
3. If FALSE → run `else` block

**One or the other runs, never both!**

## Comparison Operators

You can compare values in different ways:

| Operator | Meaning | Example | Result |
|----------|---------|---------|--------|
| `==` | Equal to | `5 == 5` | TRUE |
| `!=` | Not equal | `5 != 3` | TRUE |
| `>` | Greater than | `10 > 5` | TRUE |
| `<` | Less than | `3 < 7` | TRUE |
| `>=` | Greater or equal | `5 >= 5` | TRUE |
| `<=` | Less or equal | `4 <= 3` | FALSE |

```hlx
program comparisons {
    fn main() {
        let score = 85;

        if (score == 100) {
            print("Perfect score!");
        }

        if (score >= 90) {
            print("Grade: A");
        }

        if (score >= 80) {
            print("Grade: B or better");
        }

        if (score < 60) {
            print("Need to study more");
        }

        return 0;
    }
}
```

**Output:**
```
"Grade: B or better"
0
```

**Why only one message?** Because score is 85:
- `score == 100` → FALSE (skip)
- `score >= 90` → FALSE (skip)
- `score >= 80` → TRUE (print!)
- `score < 60` → FALSE (skip)

## Multiple Conditions with `else if`

Check multiple conditions in order:

```hlx
program grade_calculator {
    fn main() {
        let score = 85;

        if (score >= 90) {
            print("Grade: A");
        } else if (score >= 80) {
            print("Grade: B");
        } else if (score >= 70) {
            print("Grade: C");
        } else if (score >= 60) {
            print("Grade: D");
        } else {
            print("Grade: F");
        }

        return 0;
    }
}
```

**Output:**
```
"Grade: B"
0
```

**How it works:**
1. Check first condition (score >= 90) → FALSE
2. Check second condition (score >= 80) → TRUE
3. **Stop checking!** Run that block and skip the rest
4. Only ONE block ever runs

**Important:** Order matters! Check highest values first.

## Combining Conditions

Sometimes you need to check multiple things at once.

### AND: Both Must Be True

```hlx
program ticket_price {
    fn main() {
        let age = 10;
        let is_student = 1;  // 1 = true, 0 = false

        if (age < 18 && is_student == 1) {
            print("Discount ticket: $5");
        } else {
            print("Regular ticket: $12");
        }

        return 0;
    }
}
```

**Output:**
```
"Discount ticket: $5"
0
```

**AND operator (`&&`):** BOTH conditions must be true
- `age < 18` → TRUE
- `is_student == 1` → TRUE
- Both true? YES → run first block

### OR: At Least One Must Be True

```hlx
program free_entry {
    fn main() {
        let age = 70;
        let is_member = 0;

        if (age >= 65 || is_member == 1) {
            print("Free entry!");
        } else {
            print("Entry fee: $10");
        }

        return 0;
    }
}
```

**Output:**
```
"Free entry!"
0
```

**OR operator (`||`):** At least ONE condition must be true
- `age >= 65` → TRUE
- `is_member == 1` → FALSE
- At least one true? YES → run first block

## Practice: Exercise 1 - Even or Odd

Write a program that checks if a number is even or odd.

**Hint:** Use the modulo operator `%` (remainder after division)
- Even numbers: `number % 2 == 0`
- Odd numbers: `number % 2 == 1`

Test with: 10, 7, 0

<details>
<summary>Click to see solution</summary>

```hlx
program even_odd {
    fn main() {
        let number = 10;

        if (number % 2 == 0) {
            print("The number is EVEN");
        } else {
            print("The number is ODD");
        }

        return 0;
    }
}
```

**Test outputs:**
- number = 10: "The number is EVEN"
- number = 7: "The number is ODD"
- number = 0: "The number is EVEN"
</details>

## Practice: Exercise 2 - Password Checker

Write a program that checks a password:
- If password is "secret123", print "Access granted"
- Otherwise, print "Access denied"

**Hint:** Compare strings with `==` just like numbers!

<details>
<summary>Click to see solution</summary>

```hlx
program password_check {
    fn main() {
        let password = "secret123";

        if (password == "secret123") {
            print("Access granted");
        } else {
            print("Access denied");
        }

        return 0;
    }
}
```

Try changing the password to test both cases!
</details>

## Practice: Exercise 3 - Speed Limit

Write a program for a speed camera:
- Speed limit: 60 mph
- If speed <= 60: "Safe driving"
- If speed 61-75: "Warning: slow down"
- If speed > 75: "TICKET!"

Test with speeds: 55, 70, 85

<details>
<summary>Click to see solution</summary>

```hlx
program speed_camera {
    fn main() {
        let speed = 70;
        let limit = 60;

        if (speed <= limit) {
            print("Safe driving");
        } else if (speed <= 75) {
            print("Warning: slow down");
        } else {
            print("TICKET!");
        }

        return 0;
    }
}
```

**Test outputs:**
- speed = 55: "Safe driving"
- speed = 70: "Warning: slow down"
- speed = 85: "TICKET!"
</details>

## Nested `if` Statements

You can put `if` statements inside other `if` statements:

```hlx
program movie_rating {
    fn main() {
        let age = 15;
        let has_ticket = 1;

        if (has_ticket == 1) {
            print("Ticket validated");

            if (age >= 13) {
                print("Welcome! Enjoy the movie.");
            } else {
                print("This movie is PG-13. Too young.");
            }
        } else {
            print("Please buy a ticket first");
        }

        return 0;
    }
}
```

**Output (age=15, has_ticket=1):**
```
"Ticket validated"
"Welcome! Enjoy the movie."
0
```

**How it works:**
1. Check outer condition (has ticket?)
2. If true, check inner condition (old enough?)
3. Run appropriate message

## Common Mistakes

### Mistake 1: Using `=` Instead of `==`
```hlx
if (age = 18) {  // ✗ This assigns 18 to age!
```
**Fix:**
```hlx
if (age == 18) {  // ✓ This compares age to 18
```

**Remember:**
- `=` assigns a value (like `let x = 5`)
- `==` compares two values

### Mistake 2: Forgetting Parentheses
```hlx
if age > 18 {  // ✗ Missing parentheses
```
**Fix:**
```hlx
if (age > 18) {  // ✓ Condition needs parentheses
```

### Mistake 3: Wrong Order in `else if`
```hlx
if (score >= 60) {
    print("Pass");
} else if (score >= 90) {  // ✗ This will NEVER run!
    print("Excellent");
}
```

Why? If score is 95:
- First check: `95 >= 60` → TRUE, print "Pass", DONE

**Fix:** Put the highest value first:
```hlx
if (score >= 90) {
    print("Excellent");
} else if (score >= 60) {
    print("Pass");
}
```

### Mistake 4: Missing Braces
```hlx
if (x > 5)
    print("Big");
    print("Number");  // This ALWAYS runs!
```

**Fix:** Use braces for clarity:
```hlx
if (x > 5) {
    print("Big");
    print("Number");
}
```

## Challenge: Leap Year Calculator

A year is a leap year if:
- It's divisible by 4 AND
- (Not divisible by 100 OR divisible by 400)

Examples:
- 2024: Leap year (divisible by 4)
- 1900: Not leap (divisible by 100, not by 400)
- 2000: Leap year (divisible by 400)

Write a program that checks if a year is a leap year.

**Hint:** Use `%` (modulo) to check divisibility:
- `year % 4 == 0` means "divisible by 4"

<details>
<summary>Click to see solution</summary>

```hlx
program leap_year {
    fn main() {
        let year = 2024;

        if (year % 400 == 0) {
            print("Leap year!");
        } else if (year % 100 == 0) {
            print("Not a leap year");
        } else if (year % 4 == 0) {
            print("Leap year!");
        } else {
            print("Not a leap year");
        }

        print("Year:");
        print(year);

        return 0;
    }
}
```

**Test cases:**
- 2024: "Leap year!"
- 1900: "Not a leap year"
- 2000: "Leap year!"
- 2023: "Not a leap year"
</details>

## What You Learned

✅ Making decisions with `if`
✅ Doing something else with `else`
✅ Checking multiple conditions with `else if`
✅ Comparison operators (==, !=, >, <, >=, <=)
✅ Combining conditions with AND (`&&`) and OR (`||`)
✅ Nesting if statements
✅ Common pitfalls and how to avoid them

## Next Steps

In **Chapter 4**, you'll learn about **loops** - making your program repeat actions!

Preview:
```hlx
let i = 0;
loop (i < 10, 10) {
    print(i);
    i = i + 1;
}
// Prints: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9
```

Get ready for some powerful programming! 🔁

---

**Self-check:** Can you write a program that:
1. Takes a test score (0-100)
2. Prints letter grade (A/B/C/D/F)
3. Prints "PASS" if >= 60, "FAIL" otherwise

If yes, you're ready for Chapter 4!
