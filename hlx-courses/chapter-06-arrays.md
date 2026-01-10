# Chapter 6: Arrays (Lists)

So far, each variable holds one value. What if you need to store 100 values? You need **arrays**!

## What is an Array?

An **array** is like a numbered list of items:

```
Shopping List (Array):
[0] Milk
[1] Eggs
[2] Bread
[3] Butter
```

In code:
```hlx
let shopping = ["Milk", "Eggs", "Bread", "Butter"];
```

Each item has a **position** (called an **index**). Important: **indexes start at 0**, not 1!

## Creating Arrays

```hlx
program first_array {
    fn main() {
        let numbers = [10, 20, 30, 40, 50];
        let names = ["Alice", "Bob", "Charlie"];
        let mixed = [1, "hello", 42];  // Can mix types

        print(numbers);
        print(names);
        print(mixed);

        return 0;
    }
}
```

**Syntax:**
- Square brackets: `[ ]`
- Comma-separated values
- Can store numbers, strings, or mix

## Accessing Elements

Use the index (position) to get an element:

```hlx
program access_demo {
    fn main() {
        let fruits = ["Apple", "Banana", "Cherry", "Date"];

        print("First fruit:");
        print(fruits[0]);  // Apple (index 0)

        print("Second fruit:");
        print(fruits[1]);  // Banana (index 1)

        print("Last fruit:");
        print(fruits[3]);  // Date (index 3)

        return 0;
    }
}
```

**Output:**
```
"First fruit:"
"Apple"
"Second fruit:"
"Banana"
"Last fruit:"
"Date"
0
```

**Remember:** First element is at index 0!

```
Array: ["Apple", "Banana", "Cherry", "Date"]
Index:    0        1         2         3
```

## Array Length

How many items in an array? Count them or use a variable:

```hlx
program array_length {
    fn main() {
        let numbers = [10, 20, 30, 40, 50];
        let length = 5;  // We know it has 5 items

        print("Array has");
        print(length);
        print("items");

        return 0;
    }
}
```

**Note:** HLX doesn't have a built-in `length()` function yet, so you track the size yourself.

## Looping Through Arrays

This is where arrays get powerful! Visit each element:

```hlx
program loop_array {
    fn main() {
        let scores = [85, 92, 78, 95, 88];
        let i = 0;

        loop (i < 5, 5) {
            print("Score");
            print(i);
            print(":");
            print(scores[i]);
            i = i + 1;
        }

        return 0;
    }
}
```

**Output:**
```
"Score"
0
":"
85
"Score"
1
":"
92
...
```

**How it works:**
- `i` starts at 0
- `scores[0]` gets first element (85)
- `i` becomes 1
- `scores[1]` gets second element (92)
- And so on...

## Calculating with Arrays

### Find the Sum

```hlx
program array_sum {
    fn main() {
        let numbers = [10, 20, 30, 40, 50];
        let sum = 0;
        let i = 0;

        loop (i < 5, 5) {
            sum = sum + numbers[i];
            i = i + 1;
        }

        print("Sum:");
        print(sum);  // 150

        return 0;
    }
}
```

### Find the Maximum

```hlx
program array_max {
    fn main() {
        let numbers = [23, 67, 12, 89, 45];
        let max = numbers[0];  // Start with first element
        let i = 1;

        loop (i < 5, 5) {
            if (numbers[i] > max) {
                max = numbers[i];  // Found a bigger number!
            }
            i = i + 1;
        }

        print("Maximum:");
        print(max);  // 89

        return 0;
    }
}
```

### Calculate Average

```hlx
program array_average {
    fn main() {
        let scores = [85, 92, 78, 95, 88];
        let sum = 0;
        let i = 0;

        // Calculate sum
        loop (i < 5, 5) {
            sum = sum + scores[i];
            i = i + 1;
        }

        // Calculate average
        let average = sum / 5;

        print("Average score:");
        print(average);  // 87

        return 0;
    }
}
```

## Modifying Array Elements

You can change values in an array:

```hlx
program modify_array {
    fn main() {
        let numbers = [10, 20, 30];

        print("Before:");
        print(numbers[1]);  // 20

        numbers[1] = 99;  // Change second element

        print("After:");
        print(numbers[1]);  // 99

        return 0;
    }
}
```

**Output:**
```
"Before:"
20
"After:"
99
0
```

## Arrays and Functions

Pass arrays to functions:

```hlx
program array_function {
    fn print_array(arr, size) {
        let i = 0;
        loop (i < size, 100) {
            print(arr[i]);
            i = i + 1;
        }
    }

    fn sum_array(arr, size) {
        let sum = 0;
        let i = 0;

        loop (i < size, 100) {
            sum = sum + arr[i];
            i = i + 1;
        }

        return sum;
    }

    fn main() {
        let numbers = [5, 10, 15, 20];

        print("Array contents:");
        print_array(numbers, 4);

        let total = sum_array(numbers, 4);
        print("Sum:");
        print(total);

        return 0;
    }
}
```

**Output:**
```
"Array contents:"
5
10
15
20
"Sum:"
50
0
```

## Practice: Exercise 1 - Count Evens

Write a program that counts how many even numbers are in an array.

Test array: `[12, 7, 8, 15, 22, 9]`

Expected output: `3` (12, 8, and 22 are even)

<details>
<summary>Click to see solution</summary>

```hlx
program count_evens {
    fn main() {
        let numbers = [12, 7, 8, 15, 22, 9];
        let count = 0;
        let i = 0;

        loop (i < 6, 6) {
            if (numbers[i] % 2 == 0) {
                count = count + 1;
            }
            i = i + 1;
        }

        print("Even numbers:");
        print(count);

        return 0;
    }
}
```
</details>

## Practice: Exercise 2 - Find Minimum

Write a program that finds the smallest number in an array.

Test array: `[34, 12, 56, 8, 23]`

Expected output: `8`

<details>
<summary>Click to see solution</summary>

```hlx
program find_min {
    fn main() {
        let numbers = [34, 12, 56, 8, 23];
        let min = numbers[0];
        let i = 1;

        loop (i < 5, 5) {
            if (numbers[i] < min) {
                min = numbers[i];
            }
            i = i + 1;
        }

        print("Minimum:");
        print(min);

        return 0;
    }
}
```
</details>

## Practice: Exercise 3 - Reverse Array

Write a program that prints an array in reverse order.

Test array: `[1, 2, 3, 4, 5]`

Expected output: `5, 4, 3, 2, 1`

**Hint:** Start from the last index and count down.

<details>
<summary>Click to see solution</summary>

```hlx
program reverse_print {
    fn main() {
        let numbers = [1, 2, 3, 4, 5];
        let i = 4;  // Start at last index

        print("Reversed:");
        loop (i >= 0, 5) {
            print(numbers[i]);
            i = i - 1;
        }

        return 0;
    }
}
```

**Output:**
```
"Reversed:"
5
4
3
2
1
0
```
</details>

## Practice: Exercise 4 - Search Array

Write a function that searches for a value in an array:
- Returns the index if found
- Returns -1 if not found

```hlx
fn find(arr, size, target) {
    // Your code here
}
```

Test:
```hlx
let arr = [10, 20, 30, 40, 50];
print(find(arr, 5, 30));  // Should print 2
print(find(arr, 5, 99));  // Should print -1
```

<details>
<summary>Click to see solution</summary>

```hlx
program array_search {
    fn find(arr, size, target) {
        let i = 0;

        loop (i < size, 100) {
            if (arr[i] == target) {
                return i;  // Found it!
            }
            i = i + 1;
        }

        return -1;  // Not found
    }

    fn main() {
        let arr = [10, 20, 30, 40, 50];

        print("Finding 30:");
        print(find(arr, 5, 30));

        print("Finding 99:");
        print(find(arr, 5, 99));

        return 0;
    }
}
```

**Output:**
```
"Finding 30:"
2
"Finding 99:"
-1
0
```
</details>

## Nested Arrays (2D Arrays)

Arrays can contain arrays (like a grid or table):

```hlx
program grid {
    fn main() {
        // 2x3 grid (2 rows, 3 columns)
        let row1 = [1, 2, 3];
        let row2 = [4, 5, 6];

        print("First row:");
        print(row1[0]);
        print(row1[1]);
        print(row1[2]);

        print("Second row:");
        print(row2[0]);
        print(row2[1]);
        print(row2[2]);

        return 0;
    }
}
```

**Output:**
```
"First row:"
1
2
3
"Second row:"
4
5
6
0
```

Note: Full 2D array support (like `grid[0][1]`) may vary in HLX. Check documentation for current syntax.

## Common Array Patterns

### Initialize All to Zero

```hlx
let zeros = [0, 0, 0, 0, 0];
```

### Copy an Array

```hlx
let original = [1, 2, 3];
let copy = [0, 0, 0];
let i = 0;

loop (i < 3, 3) {
    copy[i] = original[i];
    i = i + 1;
}
```

### Swap Two Elements

```hlx
let arr = [10, 20, 30];
let temp = arr[0];
arr[0] = arr[1];
arr[1] = temp;
// Now arr = [20, 10, 30]
```

## Common Mistakes

### Mistake 1: Off-by-One Error
```hlx
let arr = [10, 20, 30];
print(arr[3]);  // ✗ Error! Index 3 doesn't exist
                // Array has indexes 0, 1, 2
```

**Fix:** Remember array of size N has indexes 0 to N-1:
```hlx
print(arr[2]);  // ✓ Last element
```

### Mistake 2: Wrong Loop Limit
```hlx
let arr = [10, 20, 30];
let i = 0;
loop (i <= 3, 10) {  // ✗ Goes to index 3 (out of bounds)
    print(arr[i]);
    i = i + 1;
}
```

**Fix:**
```hlx
loop (i < 3, 10) {  // ✓ Stops at index 2
    print(arr[i]);
    i = i + 1;
}
```

### Mistake 3: Forgetting to Initialize
```hlx
let sum;  // ✗ Uninitialized!
let i = 0;
loop (i < 5, 5) {
    sum = sum + arr[i];  // Error: sum has no value
    i = i + 1;
}
```

**Fix:**
```hlx
let sum = 0;  // ✓ Start at 0
```

### Mistake 4: Modifying Loop Counter Inside
```hlx
let i = 0;
loop (i < 5, 5) {
    print(arr[i]);
    i = i + 2;  // ✗ Skips elements!
    i = i + 1;  // i increases by 3 total
}
```

Be careful modifying loop counters - it's easy to mess up!

## Challenge: Bubble Sort

Implement bubble sort to sort an array in ascending order.

**Algorithm:**
1. Compare each pair of adjacent elements
2. Swap them if they're in wrong order
3. Repeat until no more swaps needed

Test array: `[64, 34, 25, 12, 22]`

Expected result: `[12, 22, 25, 34, 64]`

<details>
<summary>Click to see solution</summary>

```hlx
program bubble_sort {
    fn print_array(arr, size) {
        let i = 0;
        loop (i < size, 100) {
            print(arr[i]);
            i = i + 1;
        }
    }

    fn main() {
        let arr = [64, 34, 25, 12, 22];
        let size = 5;

        print("Before sorting:");
        print_array(arr, size);

        // Bubble sort
        let i = 0;
        loop (i < size, 10) {
            let j = 0;
            loop (j < size - 1, 10) {
                if (arr[j] > arr[j + 1]) {
                    // Swap
                    let temp = arr[j];
                    arr[j] = arr[j + 1];
                    arr[j + 1] = temp;
                }
                j = j + 1;
            }
            i = i + 1;
        }

        print("After sorting:");
        print_array(arr, size);

        return 0;
    }
}
```

**Output:**
```
"Before sorting:"
64
34
25
12
22
"After sorting:"
12
22
25
34
64
0
```
</details>

## What You Learned

✅ Creating arrays with `[ ]`
✅ Accessing elements with index
✅ Indexes start at 0
✅ Looping through arrays
✅ Calculating with arrays (sum, max, average)
✅ Modifying array elements
✅ Passing arrays to functions
✅ Common array patterns and algorithms
✅ Common mistakes and how to avoid them

## Next Steps

In **Chapter 7**, you'll learn about **working with text** - string operations and formatting!

Preview:
```hlx
let name = "Alice";
let greeting = "Hello, " + name;
print(greeting);  // "Hello, Alice"
```

Text processing is essential for most programs! 📝

---

**Self-check:** Can you write a program that:
1. Creates an array of 10 numbers
2. Calculates both the sum and average
3. Finds both the minimum and maximum
4. Prints all results

If yes, you're ready for Chapter 7!
