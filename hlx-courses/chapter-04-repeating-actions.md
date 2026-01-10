# Chapter 4: Repeating Actions

Writing the same code over and over is boring. Computers are great at repetition - let them do the work!

## Your First Loop

Want to print numbers 1 through 5? You could do this:

```hlx
print(1);
print(2);
print(3);
print(4);
print(5);
```

But what if you want 1 through 100? Or 1000? There's a better way: **loops!**

```hlx
program count_to_five {
    fn main() {
        let i = 1;
        loop (i <= 5, 10) {
            print(i);
            i = i + 1;
        }
        return 0;
    }
}
```

**Output:**
```
1
2
3
4
5
0
```

**Magic!** Let's understand how it works.

## Understanding Loops

A loop has three parts:

### 1. The Counter Variable
```hlx
let i = 1;
```
- Start at 1
- This is where the counting begins

### 2. The Condition
```hlx
loop (i <= 5, 10) {
```
- Keep going while `i <= 5`
- When `i` becomes 6, stop

### 3. The Update
```hlx
i = i + 1;
```
- After each time through, add 1 to `i`
- Eventually `i` will be 6, and the loop stops

### The Safety Limit
```hlx
loop (i <= 5, 10) {
      ^       ^
   condition  max iterations
```

The second number (10) is a **safety limit**. It says "no matter what, stop after 10 times." This prevents infinite loops that run forever.

## How Loops Work (Step by Step)

Let's trace through the loop:

```hlx
let i = 1;
loop (i <= 5, 10) {
    print(i);
    i = i + 1;
}
```

**First time:**
1. Check: Is `i <= 5`? (Is 1 <= 5?) YES
2. Print 1
3. Update: `i = i + 1` → `i` becomes 2

**Second time:**
1. Check: Is `i <= 5`? (Is 2 <= 5?) YES
2. Print 2
3. Update: `i = i + 1` → `i` becomes 3

**Third time:**
1. Check: Is `i <= 5`? (Is 3 <= 5?) YES
2. Print 3
3. Update: `i = i + 1` → `i` becomes 4

**Fourth time:**
1. Check: Is `i <= 5`? (Is 4 <= 5?) YES
2. Print 4
3. Update: `i = i + 1` → `i` becomes 5

**Fifth time:**
1. Check: Is `i <= 5`? (Is 5 <= 5?) YES
2. Print 5
3. Update: `i = i + 1` → `i` becomes 6

**Sixth time:**
1. Check: Is `i <= 5`? (Is 6 <= 5?) NO
2. **STOP! Exit the loop.**

## Counting Patterns

### Count Up
```hlx
program count_up {
    fn main() {
        let i = 0;
        loop (i < 10, 10) {
            print(i);
            i = i + 1;
        }
        return 0;
    }
}
```
**Output:** 0, 1, 2, 3, 4, 5, 6, 7, 8, 9

### Count Down
```hlx
program countdown {
    fn main() {
        let i = 10;
        loop (i > 0, 10) {
            print(i);
            i = i - 1;
        }
        print("Blastoff!");
        return 0;
    }
}
```
**Output:** 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, "Blastoff!"

### Count by Twos
```hlx
program evens {
    fn main() {
        let i = 0;
        loop (i <= 20, 11) {
            print(i);
            i = i + 2;
        }
        return 0;
    }
}
```
**Output:** 0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20

## Loops with Math

### Sum of Numbers
```hlx
program sum_to_ten {
    fn main() {
        let i = 1;
        let sum = 0;

        loop (i <= 10, 10) {
            sum = sum + i;
            i = i + 1;
        }

        print("Sum of 1 to 10:");
        print(sum);
        return 0;
    }
}
```
**Output:**
```
"Sum of 1 to 10:"
55
0
```

**How it works:**
- Start: `sum = 0`
- First loop: `sum = 0 + 1 = 1`
- Second loop: `sum = 1 + 2 = 3`
- Third loop: `sum = 3 + 3 = 6`
- ... and so on until sum = 55

### Multiplication Table
```hlx
program times_table {
    fn main() {
        let i = 1;
        let number = 7;

        print("7 times table:");

        loop (i <= 10, 10) {
            let result = number * i;
            print(i);
            print("x");
            print(number);
            print("=");
            print(result);
            i = i + 1;
        }

        return 0;
    }
}
```
**Output:**
```
"7 times table:"
1
"x"
7
"="
7
2
"x"
7
"="
14
...
```

## Breaking Out Early

Sometimes you want to exit a loop before it finishes:

```hlx
program find_number {
    fn main() {
        let i = 1;
        let target = 7;

        loop (i <= 100, 100) {
            if (i == target) {
                print("Found it!");
                print(i);
                break;
            }
            i = i + 1;
        }

        print("Done searching");
        return 0;
    }
}
```

**Output:**
```
"Found it!"
7
"Done searching"
0
```

**`break`** exits the loop immediately, even if the condition is still true.

## Skipping Iterations

**`continue`** skips the rest of this iteration and goes to the next one:

```hlx
program odd_numbers {
    fn main() {
        let i = 0;

        loop (i < 10, 10) {
            i = i + 1;

            if (i % 2 == 0) {
                continue;  // Skip even numbers
            }

            print(i);
        }

        return 0;
    }
}
```

**Output:** 1, 3, 5, 7, 9

**How it works:**
- When `i` is even (2, 4, 6, 8), `continue` skips the `print(i)`
- When `i` is odd (1, 3, 5, 7, 9), it prints normally

## Nested Loops

You can put loops inside loops:

```hlx
program multiplication_grid {
    fn main() {
        let row = 1;

        loop (row <= 3, 3) {
            let col = 1;

            loop (col <= 3, 3) {
                let result = row * col;
                print(result);
                col = col + 1;
            }

            print("---");  // Separate rows
            row = row + 1;
        }

        return 0;
    }
}
```

**Output:**
```
1
2
3
"---"
2
4
6
"---"
3
6
9
"---"
0
```

**How it works:**
- Outer loop: goes through rows (1, 2, 3)
- Inner loop: for each row, goes through columns (1, 2, 3)
- Prints row × column for each combination

## Practice: Exercise 1 - Sum Calculator

Write a program that calculates the sum of all numbers from 1 to 50.

Expected output:
```
"Sum of 1 to 50:"
1275
```

<details>
<summary>Click to see solution</summary>

```hlx
program sum_fifty {
    fn main() {
        let i = 1;
        let sum = 0;

        loop (i <= 50, 50) {
            sum = sum + i;
            i = i + 1;
        }

        print("Sum of 1 to 50:");
        print(sum);

        return 0;
    }
}
```

**Math check:** Formula is n(n+1)/2 = 50(51)/2 = 1275 ✓
</details>

## Practice: Exercise 2 - Fizz Buzz

Classic programming challenge! Print numbers 1-20, but:
- If divisible by 3, print "Fizz"
- If divisible by 5, print "Buzz"
- If divisible by both, print "FizzBuzz"
- Otherwise, print the number

Expected output:
```
1
2
"Fizz"
4
"Buzz"
"Fizz"
7
8
"Fizz"
"Buzz"
11
"Fizz"
13
14
"FizzBuzz"
16
...
```

**Hint:** Check divisible by 15 first (both 3 and 5)!

<details>
<summary>Click to see solution</summary>

```hlx
program fizzbuzz {
    fn main() {
        let i = 1;

        loop (i <= 20, 20) {
            if (i % 15 == 0) {
                print("FizzBuzz");
            } else if (i % 3 == 0) {
                print("Fizz");
            } else if (i % 5 == 0) {
                print("Buzz");
            } else {
                print(i);
            }

            i = i + 1;
        }

        return 0;
    }
}
```
</details>

## Practice: Exercise 3 - Factorial

Calculate factorial of 5: `5! = 5 × 4 × 3 × 2 × 1 = 120`

Write a program that calculates factorial using a loop.

<details>
<summary>Click to see solution</summary>

```hlx
program factorial {
    fn main() {
        let n = 5;
        let result = 1;
        let i = n;

        loop (i > 0, 10) {
            result = result * i;
            i = i - 1;
        }

        print("Factorial of 5:");
        print(result);

        return 0;
    }
}
```

**Output:**
```
"Factorial of 5:"
120
0
```
</details>

## Practice: Exercise 4 - Find Primes

Print all prime numbers from 2 to 20.

A prime number is only divisible by 1 and itself.

**Hint:** For each number, check if any number from 2 to (number-1) divides it evenly. If yes, it's not prime.

<details>
<summary>Click to see solution</summary>

```hlx
program primes {
    fn main() {
        print("Prime numbers from 2 to 20:");

        let num = 2;
        loop (num <= 20, 19) {
            let is_prime = 1;  // Assume prime (1 = true)
            let divisor = 2;

            // Check if anything divides num evenly
            loop (divisor < num, 20) {
                if (num % divisor == 0) {
                    is_prime = 0;  // Not prime
                    break;
                }
                divisor = divisor + 1;
            }

            if (is_prime == 1) {
                print(num);
            }

            num = num + 1;
        }

        return 0;
    }
}
```

**Output:** 2, 3, 5, 7, 11, 13, 17, 19
</details>

## Common Mistakes

### Mistake 1: Infinite Loop
```hlx
let i = 0;
loop (i < 10, 100) {
    print(i);
    // ✗ Forgot to update i!
}
```
This runs forever (or until safety limit)!

**Fix:** Always update your counter:
```hlx
let i = 0;
loop (i < 10, 100) {
    print(i);
    i = i + 1;  // ✓
}
```

### Mistake 2: Off-by-One Error
```hlx
let i = 1;
loop (i < 10, 10) {  // ✗ Goes 1-9, not 1-10
    print(i);
    i = i + 1;
}
```

**Fix:** Use `<=` instead of `<`:
```hlx
let i = 1;
loop (i <= 10, 10) {  // ✓ Goes 1-10
    print(i);
    i = i + 1;
}
```

### Mistake 3: Wrong Safety Limit
```hlx
let i = 0;
loop (i < 100, 10) {  // ✗ Safety limit too low!
    print(i);
    i = i + 1;
}
```
This stops at 10, not 100.

**Fix:** Safety limit should be >= expected iterations:
```hlx
let i = 0;
loop (i < 100, 100) {  // ✓ Limit matches condition
    print(i);
    i = i + 1;
}
```

### Mistake 4: Updating in Wrong Place
```hlx
let i = 0;
loop (i < 10, 10) {
    i = i + 1;  // ✗ Updates BEFORE printing
    print(i);
}
```
This prints 1-10 instead of 0-9.

**Fix:** Update at the end:
```hlx
let i = 0;
loop (i < 10, 10) {
    print(i);
    i = i + 1;  // ✓ Updates AFTER printing
}
```

## Challenge: Pattern Printer

Print this pattern using nested loops:
```
*
**
***
****
*****
```

Then print this pattern:
```
*****
****
***
**
*
```

<details>
<summary>Click to see solution</summary>

```hlx
program patterns {
    fn main() {
        print("Growing triangle:");
        let row = 1;
        loop (row <= 5, 5) {
            let col = 1;
            loop (col <= row, 5) {
                print("*");
                col = col + 1;
            }
            print("");  // New line
            row = row + 1;
        }

        print("");
        print("Shrinking triangle:");
        row = 5;
        loop (row > 0, 5) {
            let col = 1;
            loop (col <= row, 5) {
                print("*");
                col = col + 1;
            }
            print("");  // New line
            row = row - 1;
        }

        return 0;
    }
}
```

Note: In real HLX, each `print("*")` will be on its own line. For actual side-by-side printing, you'd need string concatenation (Chapter 7).
</details>

## What You Learned

✅ Creating loops with `loop(condition, limit)`
✅ Counter variables and updating them
✅ Different counting patterns (up, down, by twos)
✅ Using loops for calculations
✅ Breaking out early with `break`
✅ Skipping iterations with `continue`
✅ Nested loops (loops inside loops)
✅ Common loop mistakes and how to avoid them

## Next Steps

In **Chapter 5**, you'll learn about **functions** - packaging code into reusable pieces!

Preview:
```hlx
fn add(a, b) {
    return a + b;
}

let result = add(5, 3);  // result = 8
```

Functions are one of the most important concepts in programming. Get ready! 🚀

---

**Self-check:** Can you write a loop that:
1. Counts from 10 to 100 by tens (10, 20, 30, ... 100)
2. Calculates the sum of those numbers
3. Prints the result (should be 550)

If yes, you're ready for Chapter 5!
