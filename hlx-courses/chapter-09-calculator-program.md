# Chapter 9: Calculator Program

Build a professional-looking calculator with multiple operations and a menu system!

## The Program

**Features:**
- Multiple operations (add, subtract, multiply, divide, power)
- Clean menu interface
- Error handling
- Repeat calculations
- Operation history

## Version 1: Basic Calculator

```hlx
program calculator_v1 {
    fn add(a, b) {
        return a + b;
    }

    fn subtract(a, b) {
        return a - b;
    }

    fn multiply(a, b) {
        return a * b;
    }

    fn divide(a, b) {
        if (b == 0) {
            print("Error: Cannot divide by zero!");
            return 0;
        }
        return a / b;
    }

    fn print_menu() {
        print("===== CALCULATOR =====");
        print("1. Add");
        print("2. Subtract");
        print("3. Multiply");
        print("4. Divide");
        print("5. Quit");
        print("======================");
    }

    fn main() {
        print_menu();
        print("");

        let choice = 1;  // Simulated choice
        let num1 = 10;
        let num2 = 5;

        print("Choice:");
        print(choice);
        print("First number:");
        print(num1);
        print("Second number:");
        print(num2);
        print("");

        if (choice == 1) {
            let result = add(num1, num2);
            print("Result:");
            print(result);
        } else if (choice == 2) {
            let result = subtract(num1, num2);
            print("Result:");
            print(result);
        } else if (choice == 3) {
            let result = multiply(num1, num2);
            print("Result:");
            print(result);
        } else if (choice == 4) {
            let result = divide(num1, num2);
            print("Result:");
            print(result);
        } else if (choice == 5) {
            print("Goodbye!");
        } else {
            print("Invalid choice!");
        }

        return 0;
    }
}
```

**Output:**
```
"===== CALCULATOR ====="
"1. Add"
"2. Subtract"
"3. Multiply"
"4. Divide"
"5. Quit"
"======================"
""
"Choice:"
1
"First number:"
10
"Second number:"
5
""
"Result:"
15
0
```

## Version 2: With Power Function

Add an exponent operation:

```hlx
program calculator_v2 {
    fn add(a, b) {
        return a + b;
    }

    fn subtract(a, b) {
        return a - b;
    }

    fn multiply(a, b) {
        return a * b;
    }

    fn divide(a, b) {
        if (b == 0) {
            print("Error: Cannot divide by zero!");
            return 0;
        }
        return a / b;
    }

    fn power(base, exponent) {
        let result = 1;
        let i = 0;

        loop (i < exponent, 100) {
            result = result * base;
            i = i + 1;
        }

        return result;
    }

    fn print_menu() {
        print("===== CALCULATOR =====");
        print("1. Add");
        print("2. Subtract");
        print("3. Multiply");
        print("4. Divide");
        print("5. Power");
        print("6. Quit");
        print("======================");
    }

    fn perform_operation(choice, num1, num2) {
        if (choice == 1) {
            return add(num1, num2);
        } else if (choice == 2) {
            return subtract(num1, num2);
        } else if (choice == 3) {
            return multiply(num1, num2);
        } else if (choice == 4) {
            return divide(num1, num2);
        } else if (choice == 5) {
            return power(num1, num2);
        }
        return 0;
    }

    fn main() {
        print_menu();
        print("");

        let choice = 5;  // Test power function
        let num1 = 2;
        let num2 = 8;

        print("Choice:");
        print(choice);
        print("First number:");
        print(num1);
        print("Second number:");
        print(num2);
        print("");

        if (choice >= 1 && choice <= 5) {
            let result = perform_operation(choice, num1, num2);
            print("Result:");
            print(result);
        } else if (choice == 6) {
            print("Goodbye!");
        } else {
            print("Invalid choice!");
        }

        return 0;
    }
}
```

## Version 3: Multiple Calculations

Let users do many calculations in a row:

```hlx
program calculator_v3 {
    fn add(a, b) {
        return a + b;
    }

    fn subtract(a, b) {
        return a - b;
    }

    fn multiply(a, b) {
        return a * b;
    }

    fn divide(a, b) {
        if (b == 0) {
            print("Error: Cannot divide by zero!");
            return 0;
        }
        return a / b;
    }

    fn power(base, exponent) {
        let result = 1;
        let i = 0;

        loop (i < exponent, 100) {
            result = result * base;
            i = i + 1;
        }

        return result;
    }

    fn print_menu() {
        print("");
        print("===== CALCULATOR =====");
        print("1. Add");
        print("2. Subtract");
        print("3. Multiply");
        print("4. Divide");
        print("5. Power");
        print("6. Quit");
        print("======================");
    }

    fn perform_operation(choice, num1, num2) {
        if (choice == 1) {
            return add(num1, num2);
        } else if (choice == 2) {
            return subtract(num1, num2);
        } else if (choice == 3) {
            return multiply(num1, num2);
        } else if (choice == 4) {
            return divide(num1, num2);
        } else if (choice == 5) {
            return power(num1, num2);
        }
        return 0;
    }

    fn main() {
        print("Welcome to Calculator!");

        // Simulate multiple operations
        let operations = [1, 3, 5];  // Add, Multiply, Power
        let nums1 = [10, 4, 2];
        let nums2 = [5, 3, 10];
        let i = 0;
        let running = 1;

        loop (i < 3, 10) {
            if (running == 0) {
                break;
            }

            print_menu();

            let choice = operations[i];
            print("Choice:");
            print(choice);

            if (choice == 6) {
                print("Goodbye!");
                running = 0;
            } else if (choice >= 1 && choice <= 5) {
                let num1 = nums1[i];
                let num2 = nums2[i];

                print("First number:");
                print(num1);
                print("Second number:");
                print(num2);

                let result = perform_operation(choice, num1, num2);
                print("Result:");
                print(result);
            } else {
                print("Invalid choice!");
            }

            i = i + 1;
        }

        print("");
        print("Thank you for using Calculator!");

        return 0;
    }
}
```

## Version 4: With History

Track all calculations:

```hlx
program calculator_v4 {
    fn add(a, b) {
        return a + b;
    }

    fn subtract(a, b) {
        return a - b;
    }

    fn multiply(a, b) {
        return a * b;
    }

    fn divide(a, b) {
        if (b == 0) {
            print("Error: Cannot divide by zero!");
            return 0;
        }
        return a / b;
    }

    fn get_operation_name(choice) {
        if (choice == 1) {
            print("Addition");
        } else if (choice == 2) {
            print("Subtraction");
        } else if (choice == 3) {
            print("Multiplication");
        } else if (choice == 4) {
            print("Division");
        }
    }

    fn print_menu() {
        print("");
        print("===== CALCULATOR =====");
        print("1. Add");
        print("2. Subtract");
        print("3. Multiply");
        print("4. Divide");
        print("5. Show History");
        print("6. Clear History");
        print("7. Quit");
        print("======================");
    }

    fn print_history(operations, nums1, nums2, results, count) {
        print("");
        print("===== HISTORY =====");

        if (count == 0) {
            print("No calculations yet.");
        } else {
            let i = 0;
            loop (i < count, 20) {
                print("");
                print("Calculation #");
                print(i + 1);
                print(":");

                get_operation_name(operations[i]);

                print(nums1[i]);
                print("and");
                print(nums2[i]);
                print("=");
                print(results[i]);

                i = i + 1;
            }
        }

        print("===================");
    }

    fn perform_operation(choice, num1, num2) {
        if (choice == 1) {
            return add(num1, num2);
        } else if (choice == 2) {
            return subtract(num1, num2);
        } else if (choice == 3) {
            return multiply(num1, num2);
        } else if (choice == 4) {
            return divide(num1, num2);
        }
        return 0;
    }

    fn main() {
        print("Welcome to Calculator!");

        // History tracking (max 10 operations)
        let history_ops = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let history_nums1 = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let history_nums2 = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let history_results = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let history_count = 0;

        // Simulate some operations
        let operations = [1, 3, 5, 7];  // Add, Multiply, History, Quit
        let nums1 = [10, 4, 0, 0];
        let nums2 = [5, 3, 0, 0];
        let i = 0;
        let running = 1;

        loop (i < 4, 10) {
            if (running == 0) {
                break;
            }

            print_menu();

            let choice = operations[i];
            print("Choice:");
            print(choice);

            if (choice == 7) {
                print("Goodbye!");
                running = 0;
            } else if (choice == 5) {
                print_history(history_ops, history_nums1, history_nums2, history_results, history_count);
            } else if (choice == 6) {
                print("History cleared!");
                history_count = 0;
            } else if (choice >= 1 && choice <= 4) {
                let num1 = nums1[i];
                let num2 = nums2[i];

                print("First number:");
                print(num1);
                print("Second number:");
                print(num2);

                let result = perform_operation(choice, num1, num2);
                print("Result:");
                print(result);

                // Add to history
                history_ops[history_count] = choice;
                history_nums1[history_count] = num1;
                history_nums2[history_count] = num2;
                history_results[history_count] = result;
                history_count = history_count + 1;
            } else {
                print("Invalid choice!");
            }

            i = i + 1;
        }

        print("");
        print("Thank you for using Calculator!");

        return 0;
    }
}
```

**Output (partial):**
```
"Welcome to Calculator!"
""
"===== CALCULATOR ====="
"1. Add"
"2. Subtract"
"3. Multiply"
"4. Divide"
"5. Show History"
"6. Clear History"
"7. Quit"
"======================"
"Choice:"
1
"First number:"
10
"Second number:"
5
"Result:"
15
""
"===== CALCULATOR ====="
...
"Choice:"
5
""
"===== HISTORY ====="
""
"Calculation #"
1
":"
"Addition"
10
"and"
5
"="
15
""
"Calculation #"
2
":"
"Multiplication"
4
"and"
3
"="
12
"==================="
```

## Advanced Features

### Feature 1: Scientific Functions

```hlx
fn square_root_approx(n) {
    // Newton's method for square root
    let guess = n / 2;
    let i = 0;

    loop (i < 10, 10) {
        guess = (guess + (n / guess)) / 2;
        i = i + 1;
    }

    return guess;
}

fn factorial(n) {
    let result = 1;
    let i = 1;

    loop (i <= n, 100) {
        result = result * i;
        i = i + 1;
    }

    return result;
}

fn percentage(value, percent) {
    return (value * percent) / 100;
}
```

### Feature 2: Memory Functions

```hlx
let memory = 0;

fn memory_store(value) {
    memory = value;
    print("Value stored in memory");
}

fn memory_recall() {
    print("Memory:");
    print(memory);
    return memory;
}

fn memory_clear() {
    memory = 0;
    print("Memory cleared");
}

fn memory_add(value) {
    memory = memory + value;
    print("Added to memory");
}
```

### Feature 3: Error Handling

```hlx
fn is_valid_operation(choice) {
    if (choice >= 1 && choice <= 10) {
        return 1;
    }
    return 0;
}

fn is_valid_number(num) {
    // Check if number is reasonable
    if (num < -1000000 || num > 1000000) {
        print("Warning: Number out of reasonable range");
        return 0;
    }
    return 1;
}

fn handle_division_by_zero(divisor) {
    if (divisor == 0) {
        print("ERROR: Division by zero!");
        print("Please enter a non-zero divisor");
        return 0;  // Error
    }
    return 1;  // OK
}
```

## Practice: Add Features!

Try adding these features to your calculator:

### 1. More Operations
- Square root
- Percentage
- Absolute value
- Maximum/Minimum of two numbers

### 2. Better Interface
- Color-coded output (if supported)
- Operation history with timestamps
- "Last answer" feature (use previous result)

### 3. Validation
- Check for invalid inputs
- Confirm before clearing history
- Warn about large numbers

### 4. Statistics
- Count how many operations of each type
- Show most-used operation
- Calculate average result

## What You Built

✅ Professional calculator program
✅ Clean menu system
✅ Multiple operations
✅ Error handling
✅ Operation history
✅ Organized, maintainable code

## What You Learned

✅ Building menu-driven programs
✅ Managing program state over time
✅ Error handling and validation
✅ Data storage (history)
✅ User experience design
✅ Code organization at scale

## Next Steps

In **Chapter 10** (final chapter!), you'll plan and build **your own project**!

You'll learn:
- How to plan a program
- Breaking down big problems
- Testing and debugging strategies
- Documenting your code
- Sharing your work

Time to build something YOU want to make! 🚀

---

**Challenge:** Before moving on, enhance your calculator with at least TWO features from the list above. Make it yours!
