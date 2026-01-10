# Chapter 7: Working with Text

Text (called **strings**) is everywhere in programming. Let's learn to manipulate it!

## Strings Recap

You've been using strings since Chapter 1:

```hlx
let name = "Alice";
let message = "Hello, World!";
```

**Remember:**
- Use double quotes: `"text"`
- Strings are just text data
- Can contain letters, numbers, spaces, punctuation

## String Length

How long is a string? Count the characters:

```hlx
program string_length {
    fn main() {
        let word = "Hello";
        // H e l l o
        // 1 2 3 4 5

        let length = 5;
        print("Length:");
        print(length);

        return 0;
    }
}
```

**Note:** In HLX, you track string length yourself (or calculate it by looping through characters).

## Accessing Characters

Strings are like arrays of characters:

```hlx
program string_chars {
    fn main() {
        let word = "Hello";

        print("First character:");
        print(word[0]);  // Should print 'H'

        print("Last character:");
        print(word[4]);  // Should print 'o'

        return 0;
    }
}
```

**Note:** Character access syntax may vary in HLX. Check if your version supports `string[index]`.

## Comparing Strings

Check if strings are equal:

```hlx
program string_compare {
    fn main() {
        let password = "secret";
        let input = "secret";

        if (password == input) {
            print("Access granted!");
        } else {
            print("Access denied!");
        }

        return 0;
    }
}
```

**Important:** String comparison is case-sensitive:
- `"Hello"` != `"hello"`
- `"ABC"` != `"abc"`

## Building Strings from Numbers

Convert numbers to strings for display:

```hlx
program number_to_string {
    fn main() {
        let age = 25;
        let year = 2026;

        print("Age:");
        print(age);

        print("Year:");
        print(year);

        // Building a message
        print("You will be");
        print(age + 1);
        print("in");
        print(year + 1);

        return 0;
    }
}
```

**Output:**
```
"Age:"
25
"Year:"
2026
"You will be"
26
"in"
2027
0
```

## Working with Multiple Lines

Create formatted output:

```hlx
program formatted_output {
    fn main() {
        let name = "Alice";
        let age = 25;
        let city = "Boston";

        print("==== User Profile ====");
        print("Name:");
        print(name);
        print("Age:");
        print(age);
        print("City:");
        print(city);
        print("=====================");

        return 0;
    }
}
```

**Output:**
```
"==== User Profile ===="
"Name:"
"Alice"
"Age:"
25
"City:"
"Boston"
"====================="
0
```

## String Patterns

### Creating Headers

```hlx
fn print_header(title) {
    print("======================");
    print(title);
    print("======================");
}

print_header("Welcome to My Program");
```

### Creating Separators

```hlx
fn print_separator() {
    print("----------------------");
}

print("Section 1");
print_separator();
print("Section 2");
```

## Practical Example: Receipt Printer

```hlx
program receipt {
    fn print_line(item, price) {
        print(item);
        print("$");
        print(price);
    }

    fn main() {
        print("====== RECEIPT ======");
        print("");

        print_line("Coffee", 4);
        print_line("Donut", 2);
        print_line("Sandwich", 8);

        print("");
        print("--------------------");

        let total = 4 + 2 + 8;
        print("Total: $");
        print(total);

        print("");
        print("Thank you!");
        print("====================");

        return 0;
    }
}
```

**Output:**
```
"====== RECEIPT ======"
""
"Coffee"
"$"
4
"Donut"
"$"
2
"Sandwich"
"$"
8
""
"--------------------"
"Total: $"
14
""
"Thank you!"
"===================="
0
```

## Practice: Exercise 1 - Name Formatter

Write a function that formats a name nicely:

```hlx
fn format_name(first, last) {
    // Print: "Name: FirstName LastName"
}
```

Test:
```hlx
format_name("John", "Smith");  // "Name: John Smith"
format_name("Alice", "Johnson");  // "Name: Alice Johnson"
```

<details>
<summary>Click to see solution</summary>

```hlx
program name_formatter {
    fn format_name(first, last) {
        print("Name:");
        print(first);
        print(last);
    }

    fn main() {
        format_name("John", "Smith");
        print("");
        format_name("Alice", "Johnson");

        return 0;
    }
}
```
</details>

## Practice: Exercise 2 - Grade Report

Create a function that prints a formatted grade report:

```hlx
fn print_grade_report(name, math, english, science) {
    // Print nicely formatted report with name and grades
}
```

<details>
<summary>Click to see solution</summary>

```hlx
program grade_report {
    fn print_grade_report(name, math, english, science) {
        print("===== Grade Report =====");
        print("Student:");
        print(name);
        print("");
        print("Mathematics:");
        print(math);
        print("English:");
        print(english);
        print("Science:");
        print(science);
        print("");

        let total = math + english + science;
        let average = total / 3;

        print("Average:");
        print(average);
        print("========================");
    }

    fn main() {
        print_grade_report("Alice", 92, 88, 95);
        print("");
        print_grade_report("Bob", 78, 85, 82);

        return 0;
    }
}
```
</details>

## Practice: Exercise 3 - Menu System

Create a menu system for a program:

```
===== MAIN MENU =====
1. New Game
2. Load Game
3. Settings
4. Exit
=====================
Enter choice:
```

<details>
<summary>Click to see solution</summary>

```hlx
program menu_system {
    fn print_menu() {
        print("===== MAIN MENU =====");
        print("1. New Game");
        print("2. Load Game");
        print("3. Settings");
        print("4. Exit");
        print("=====================");
        print("Enter choice:");
    }

    fn main() {
        print_menu();

        let choice = 1;  // Simulated user input

        if (choice == 1) {
            print("Starting new game...");
        } else if (choice == 2) {
            print("Loading game...");
        } else if (choice == 3) {
            print("Opening settings...");
        } else if (choice == 4) {
            print("Goodbye!");
        } else {
            print("Invalid choice!");
        }

        return 0;
    }
}
```
</details>

## String Arrays

Store multiple strings:

```hlx
program string_array {
    fn main() {
        let fruits = ["Apple", "Banana", "Cherry", "Date"];

        print("Fruit list:");

        let i = 0;
        loop (i < 4, 4) {
            print(i + 1);
            print(".");
            print(fruits[i]);
            i = i + 1;
        }

        return 0;
    }
}
```

**Output:**
```
"Fruit list:"
1
"."
"Apple"
2
"."
"Banana"
3
"."
"Cherry"
4
"."
"Date"
0
```

## Practical Example: Simple Quiz

```hlx
program quiz {
    fn ask_question(question, answer, user_answer) {
        print(question);

        if (user_answer == answer) {
            print("Correct!");
            return 1;
        } else {
            print("Wrong! The answer is:");
            print(answer);
            return 0;
        }
    }

    fn main() {
        print("===== QUIZ TIME =====");
        let score = 0;

        // Question 1
        let q1 = "What is 2 + 2?";
        let result1 = ask_question(q1, 4, 4);
        score = score + result1;

        print("");

        // Question 2
        let q2 = "What is 5 * 3?";
        let result2 = ask_question(q2, 15, 14);  // Wrong answer
        score = score + result2;

        print("");

        // Question 3
        let q3 = "What is 10 / 2?";
        let result3 = ask_question(q3, 5, 5);
        score = score + result3;

        print("");
        print("Final score:");
        print(score);
        print("out of 3");

        return 0;
    }
}
```

**Output:**
```
"===== QUIZ TIME ====="
"What is 2 + 2?"
"Correct!"
""
"What is 5 * 3?"
"Wrong! The answer is:"
15
""
"What is 10 / 2?"
"Correct!"
""
"Final score:"
2
"out of 3"
0
```

## Common Patterns

### Table Headers

```hlx
fn print_table_header() {
    print("|  Name  | Score | Grade |");
    print("|--------|-------|-------|");
}
```

### Progress Indicator

```hlx
fn print_progress(current, total) {
    print("Progress:");
    print(current);
    print("/");
    print(total);
}

print_progress(5, 10);  // "Progress: 5 / 10"
```

### Status Messages

```hlx
fn print_status(message, is_success) {
    if (is_success == 1) {
        print("[SUCCESS]");
    } else {
        print("[FAILED]");
    }
    print(message);
}

print_status("File loaded", 1);   // "[SUCCESS] File loaded"
print_status("Network error", 0);  // "[FAILED] Network error"
```

## Challenge: Simple Text Adventure

Create a simple text adventure game:

```hlx
program adventure {
    fn print_scene(description) {
        print("========================");
        print(description);
        print("========================");
    }

    fn main() {
        let player_name = "Hero";

        print("Welcome to the Adventure!");
        print("What is your name?");
        // In real program, would get input
        // For now, use variable

        print("");
        print("Hello,");
        print(player_name);

        print("");
        print_scene("You are in a dark forest. You see two paths.");

        print("");
        print("1. Take the left path");
        print("2. Take the right path");

        let choice = 1;  // Simulate choice

        print("");
        if (choice == 1) {
            print_scene("You encounter a friendly wizard!");
            print("The wizard gives you a magic sword.");
        } else {
            print_scene("You find a treasure chest!");
            print("You gain 100 gold coins.");
        }

        print("");
        print("The End!");

        return 0;
    }
}
```

## What You Learned

✅ Working with strings (text)
✅ String comparison
✅ Building formatted output
✅ String arrays
✅ Creating menus and interfaces
✅ Building interactive programs
✅ Common text patterns

## Next Steps

**Congratulations!** You've completed Part 2 (The Basics + Getting Serious).

In **Chapter 8**, you'll build **your first real game** - a number guessing game!

Preview:
```
I'm thinking of a number between 1 and 100
Guess: 50
Too high!
Guess: 25
Too low!
Guess: 37
Correct! You won in 3 guesses!
```

Time to build something fun! 🎮

---

**Self-check:** Can you create a program that:
1. Prints a formatted menu with 3 options
2. "Accepts" a choice (use a variable)
3. Prints different messages based on choice
4. Has nice formatting (headers, separators)

If yes, you're ready for Chapter 8!
