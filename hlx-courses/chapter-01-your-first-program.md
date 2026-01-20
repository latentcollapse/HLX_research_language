# Chapter 1: Your First Program

Welcome to programming! In this chapter, you'll write your first real program. By the end, you'll understand what code is and how to make computers do things.

## What is a Program?

A **program** is a set of instructions you give to a computer. Think of it like a recipe:
- A recipe tells a person how to make cookies
- A program tells a computer how to do something

The difference? Computers are very literal. They do *exactly* what you tell them, nothing more, nothing less.

## Your First Line of Code

Let's start with the simplest program possible: making the computer say "Hello!"

Create a new file called `hello.hlx` and type this:

```hlx
program hello {
    fn main() {
        print("Hello, World!");
        return 0;
    }
}
```

Now run it:
```bash
cd /path/to/hlx-compiler/hlx
./target/release/hlx run hello.hlx
```

You should see:
```
"Hello, World!"
0
```

**Congratulations!** You just wrote your first program! 🎉

## Understanding What You Wrote

Let's break down every single piece:

### Line 1: `program hello {`
```hlx
program hello {
```
- **program** - This tells the computer "here comes a program"
- **hello** - This is the name of your program (you can pick any name)
- **{** - This curly brace means "start of the program"

Think of it like the title of a recipe: "Recipe: Chocolate Chip Cookies"

### Line 2: `fn main() {`
```hlx
fn main() {
```
- **fn** - Short for "function" (we'll learn more about this later)
- **main** - The special name for "where to start"
- **()** - These parentheses will hold inputs later (none for now)
- **{** - Another curly brace, starting the instructions

Every program needs a `main()` function - it's where the computer starts reading your instructions.

### Line 3: `print("Hello, World!");`
```hlx
print("Hello, World!");
```
- **print** - A command that means "show this on the screen"
- **("Hello, World!")** - What to show (the quotes mean "this is text")
- **;** - Semicolon means "end of instruction"

This is like a recipe step: "Mix flour and sugar"

### Line 4: `return 0;`
```hlx
return 0;
```
- **return** - Means "I'm done, here's the result"
- **0** - The number zero (by convention, 0 means "success")
- **;** - End of instruction

### Lines 5-6: `}`
```hlx
    }
}
```
These closing curly braces close what we opened:
- First `}` closes the `main()` function
- Second `}` closes the `program`

**Rule:** Every `{` needs a matching `}`

## Important Concepts

### 1. Everything is Precise
If you type `Print` instead of `print`, it won't work. Computers are case-sensitive:
- `print` ✓ (lowercase)
- `Print` ✗ (uppercase P)
- `PRINT` ✗ (all caps)

### 2. Punctuation Matters
Missing a semicolon? The program won't run:
- `print("Hi");` ✓
- `print("Hi")` ✗ (missing semicolon)

### 3. Quotes Mean Text
- `print("Hello")` - Shows the word "Hello"
- `print(42)` - Shows the number 42
- `print("42")` - Shows the text "42" (not a number)

## Practice: Exercise 1

**Your turn!** Modify the program to say something different.

Change this line:
```hlx
print("Hello, World!");
```

To:
```hlx
print("Hi, my name is ______!");
```
(Fill in your name)

Run it and see what happens!

<details>
<summary>Click to see solution</summary>

```hlx
program hello {
    fn main() {
        print("Hi, my name is Matt!");
        return 0;
    }
}
```

When you run it, you should see:
```
"Hi, my name is Matt!"
0
```
</details>

## Practice: Exercise 2

**Multiple messages!** You can have multiple print statements:

```hlx
program greetings {
    fn main() {
        print("Hello!");
        print("How are you?");
        print("I'm learning to code!");
        return 0;
    }
}
```

**Your task:** Write a program that prints 5 lines, telling a short story.

Example output:
```
"Once upon a time"
"There was a programmer"
"Who learned HLX"
"And built amazing things"
"The end!"
0
```

<details>
<summary>Click to see solution</summary>

```hlx
program story {
    fn main() {
        print("Once upon a time");
        print("There was a programmer");
        print("Who learned HLX");
        print("And built amazing things");
        print("The end!");
        return 0;
    }
}
```
</details>

## Common Mistakes (and How to Fix Them)

### Mistake 1: Forgot Semicolon
```hlx
print("Hello")  // ✗ Missing semicolon
```
**Error:** Parse error
**Fix:** Add `;` at the end:
```hlx
print("Hello");  // ✓
```

### Mistake 2: Forgot Quotes
```hlx
print(Hello);  // ✗ Computer thinks Hello is a variable
```
**Fix:** Add quotes around text:
```hlx
print("Hello");  // ✓
```

### Mistake 3: Mismatched Braces
```hlx
program test {
    fn main() {
        print("Hi");
    // ✗ Missing closing brace for program
}
```
**Fix:** Count your braces - every `{` needs a `}`

### Mistake 4: Wrong Case
```hlx
PRINT("Hello");  // ✗ PRINT is not a command
Print("Hello");  // ✗ Print is not a command
```
**Fix:** Use lowercase:
```hlx
print("Hello");  // ✓
```

## Challenge: The Bio Program

Write a program that prints a bio about you:
- Your name
- Your age
- Your favorite hobby
- Something you want to learn

Example output:
```
"Name: Alex"
"Age: 25"
"Hobby: Guitar"
"Want to learn: How to build games"
0
```

<details>
<summary>Click to see solution</summary>

```hlx
program bio {
    fn main() {
        print("Name: Alex");
        print("Age: 25");
        print("Hobby: Guitar");
        print("Want to learn: How to build games");
        return 0;
    }
}
```
</details>

## What You Learned

✅ What a program is
✅ How to write and run HLX code
✅ The `print()` command
✅ Program structure (program, main, braces)
✅ Text vs numbers (quotes vs no quotes)
✅ The importance of semicolons and precision

## Next Steps

In **Chapter 2**, you'll learn about **variables** - how to store information and do math!

Preview:
```hlx
let age = 25;
let next_year = age + 1;
print(next_year);  // Shows 26
```

See you there! 🚀

---

**Pro Tip:** Before moving on, make sure you can write a "Hello, World!" program from memory. It's the foundation of everything else!
