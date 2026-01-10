# Chapter 10: Your Own Project

**Congratulations!** You've learned everything you need to build real programs. Now it's time to create something of your own!

## The Goal

Build a complete program from scratch:
- Plan it yourself
- Write all the code
- Test and debug it
- Make it work

**You choose what to build!** Pick something you're interested in.

## Step 1: Choose a Project

Here are some ideas (or invent your own!):

### Beginner Level
1. **To-Do List** - Add, remove, and display tasks
2. **Quiz Game** - Multiple choice questions with scoring
3. **Unit Converter** - Temperature, distance, weight, etc.
4. **Simple Encryption** - Caesar cipher for messages
5. **Grade Calculator** - Calculate final grades from test scores

### Intermediate Level
6. **Hangman Game** - Guess letters to find a word
7. **Rock Paper Scissors** - Best of 5 against computer
8. **Simple Banking** - Deposits, withdrawals, balance
9. **Contact Book** - Store and search contacts
10. **Mad Libs Generator** - Fill-in-the-blank stories

### Advanced Level
11. **Text-Based RPG** - Small adventure with battles
12. **Tic-Tac-Toe** - Full game with AI
13. **Simple Database** - CRUD operations on records
14. **Math Tutor** - Generate random problems and check answers
15. **Inventory System** - Track items, quantities, values

## Step 2: Plan Your Project

Before writing code, make a plan!

### Planning Template

```
PROJECT: [Name]

PURPOSE: What does it do?
[1-2 sentences explaining the program]

FEATURES: What can users do?
- Feature 1
- Feature 2
- Feature 3
...

DATA: What information do I need to store?
- Variable 1: [type] - [what it stores]
- Array 1: [what it contains]
...

FUNCTIONS: What functions do I need?
- function1() - [what it does]
- function2() - [what it does]
...

STEPS: What happens when program runs?
1. First thing
2. Second thing
3. ...
```

### Example Plan: To-Do List

```
PROJECT: Simple To-Do List

PURPOSE:
Helps users track tasks they need to complete.
Can add tasks, mark them done, and view all tasks.

FEATURES:
- Add a new task
- Mark task as complete
- View all tasks
- Delete a task
- Count how many tasks are left

DATA:
- tasks: array of strings (task descriptions)
- completed: array of integers (1=done, 0=not done)
- task_count: integer (how many tasks)

FUNCTIONS:
- print_menu() - Show options to user
- add_task() - Add new task to list
- mark_complete() - Mark a task as done
- view_tasks() - Display all tasks with status
- delete_task() - Remove a task
- count_remaining() - Count incomplete tasks

STEPS:
1. Show welcome message
2. Loop:
   - Show menu
   - Get user choice
   - Do the chosen action
   - Continue until user quits
3. Say goodbye
```

## Step 3: Start Small

**Don't build everything at once!** Start with the core feature.

### Version 1: Absolute Minimum

For the To-Do List:
```hlx
program todo_v1 {
    fn main() {
        print("===== TO-DO LIST =====");

        let tasks = ["Buy milk", "Call mom", "Study HLX"];
        let count = 3;

        print("Your tasks:");
        let i = 0;
        loop (i < count, 10) {
            print(i + 1);
            print(".");
            print(tasks[i]);
            i = i + 1;
        }

        return 0;
    }
}
```

**Get this working first!** Then add features.

## Step 4: Add Features One at a Time

### Version 2: Add Status

```hlx
program todo_v2 {
    fn print_tasks(tasks, completed, count) {
        print("Your tasks:");
        let i = 0;
        loop (i < count, 20) {
            print(i + 1);
            print(".");
            print(tasks[i]);

            if (completed[i] == 1) {
                print("[DONE]");
            } else {
                print("[TODO]");
            }

            i = i + 1;
        }
    }

    fn main() {
        print("===== TO-DO LIST =====");

        let tasks = ["Buy milk", "Call mom", "Study HLX"];
        let completed = [0, 1, 0];  // 1=done, 0=not done
        let count = 3;

        print_tasks(tasks, completed, count);

        return 0;
    }
}
```

### Version 3: Add Menu

```hlx
program todo_v3 {
    fn print_menu() {
        print("");
        print("===== MENU =====");
        print("1. View tasks");
        print("2. Add task");
        print("3. Mark complete");
        print("4. Quit");
        print("================");
    }

    fn print_tasks(tasks, completed, count) {
        print("");
        print("Your tasks:");
        if (count == 0) {
            print("No tasks yet!");
        } else {
            let i = 0;
            loop (i < count, 20) {
                print(i + 1);
                print(".");
                print(tasks[i]);

                if (completed[i] == 1) {
                    print("[DONE]");
                } else {
                    print("[TODO]");
                }

                i = i + 1;
            }
        }
    }

    fn main() {
        print("===== TO-DO LIST =====");

        let tasks = ["Buy milk", "Call mom", "Study HLX", "", "", "", "", "", "", ""];
        let completed = [0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
        let count = 3;

        // Simulate user actions
        let actions = [1, 3, 1, 4];  // View, Complete, View, Quit
        let i = 0;
        let running = 1;

        loop (i < 4, 10) {
            if (running == 0) {
                break;
            }

            print_menu();
            let choice = actions[i];
            print("Choice:");
            print(choice);

            if (choice == 1) {
                print_tasks(tasks, completed, count);
            } else if (choice == 2) {
                print("Add task: [simulated]");
                // Would add task here
            } else if (choice == 3) {
                let task_num = 1;  // Simulate marking task 1
                print("Mark task #");
                print(task_num);
                print("as complete");
                completed[task_num - 1] = 1;
            } else if (choice == 4) {
                print("Goodbye!");
                running = 0;
            }

            i = i + 1;
        }

        return 0;
    }
}
```

## Step 5: Test Everything

As you add features, test them!

### Testing Checklist

```
TESTING: To-Do List

Feature: View Tasks
☐ Shows tasks when list is empty
☐ Shows one task correctly
☐ Shows multiple tasks correctly
☐ Shows completed/incomplete status

Feature: Add Task
☐ Adds task to empty list
☐ Adds task to existing list
☐ Handles maximum number of tasks
☐ Task starts as incomplete

Feature: Mark Complete
☐ Marks correct task
☐ Doesn't affect other tasks
☐ Can mark as complete twice (no error)

Feature: Delete Task
☐ Deletes correct task
☐ Shifts other tasks properly
☐ Updates task count

Overall:
☐ Program doesn't crash
☐ Menu always works
☐ Can quit anytime
☐ All features work together
```

## Step 6: Debug Problems

When something goes wrong:

### Debugging Process

1. **Identify the problem**
   - What did you expect?
   - What actually happened?
   - When does it happen?

2. **Isolate the bug**
   - Which function has the problem?
   - Which line of code?

3. **Use print statements**
   ```hlx
   print("DEBUG: count =");
   print(count);
   print("DEBUG: i =");
   print(i);
   ```

4. **Check common mistakes**
   - Off-by-one errors (array indices)
   - Forgot to update a variable
   - Wrong comparison operator
   - Loop going too far/not far enough

5. **Fix and test**
   - Make ONE change
   - Test again
   - Repeat until fixed

## Step 7: Polish It

Make your program professional:

### Polish Checklist

```
POLISH: Make it great!

☐ Clear welcome message
☐ Instructions for user
☐ Nice formatting (headers, separators)
☐ Error messages are helpful
☐ Consistent style throughout
☐ No typos in messages
☐ Goodbye message when quitting
```

## Example Project: Complete Grading System

Here's a complete example project to inspire you:

```hlx
program grading_system {
    fn print_header() {
        print("================================");
        print("     STUDENT GRADE MANAGER");
        print("================================");
    }

    fn print_menu() {
        print("");
        print("===== MENU =====");
        print("1. View all grades");
        print("2. Add grade");
        print("3. Calculate average");
        print("4. Find highest grade");
        print("5. Find lowest grade");
        print("6. Count passing grades");
        print("7. Quit");
        print("================");
    }

    fn print_grades(grades, count) {
        print("");
        print("===== ALL GRADES =====");

        if (count == 0) {
            print("No grades entered yet.");
        } else {
            let i = 0;
            loop (i < count, 50) {
                print("Grade");
                print(i + 1);
                print(":");
                print(grades[i]);
                i = i + 1;
            }
        }

        print("=====================");
    }

    fn calculate_average(grades, count) {
        if (count == 0) {
            print("No grades to average!");
            return 0;
        }

        let sum = 0;
        let i = 0;

        loop (i < count, 50) {
            sum = sum + grades[i];
            i = i + 1;
        }

        return sum / count;
    }

    fn find_highest(grades, count) {
        if (count == 0) {
            return 0;
        }

        let max = grades[0];
        let i = 1;

        loop (i < count, 50) {
            if (grades[i] > max) {
                max = grades[i];
            }
            i = i + 1;
        }

        return max;
    }

    fn find_lowest(grades, count) {
        if (count == 0) {
            return 0;
        }

        let min = grades[0];
        let i = 1;

        loop (i < count, 50) {
            if (grades[i] < min) {
                min = grades[i];
            }
            i = i + 1;
        }

        return min;
    }

    fn count_passing(grades, count) {
        let passing = 0;
        let i = 0;

        loop (i < count, 50) {
            if (grades[i] >= 60) {
                passing = passing + 1;
            }
            i = i + 1;
        }

        return passing;
    }

    fn main() {
        print_header();

        // Initialize grade storage
        let grades = [85, 92, 78, 95, 88, 0, 0, 0, 0, 0];
        let count = 5;
        let max_grades = 10;

        // Simulate user actions
        let actions = [1, 3, 4, 5, 6, 7];
        let i = 0;
        let running = 1;

        loop (i < 6, 10) {
            if (running == 0) {
                break;
            }

            print_menu();
            let choice = actions[i];
            print("Choice:");
            print(choice);

            if (choice == 1) {
                print_grades(grades, count);
            } else if (choice == 2) {
                if (count < max_grades) {
                    let new_grade = 80;  // Simulated input
                    grades[count] = new_grade;
                    count = count + 1;
                    print("Grade added!");
                } else {
                    print("Grade storage full!");
                }
            } else if (choice == 3) {
                let avg = calculate_average(grades, count);
                print("Average grade:");
                print(avg);
            } else if (choice == 4) {
                let highest = find_highest(grades, count);
                print("Highest grade:");
                print(highest);
            } else if (choice == 5) {
                let lowest = find_lowest(grades, count);
                print("Lowest grade:");
                print(lowest);
            } else if (choice == 6) {
                let passing = count_passing(grades, count);
                print("Passing grades:");
                print(passing);
                print("out of");
                print(count);
            } else if (choice == 7) {
                print("");
                print("Thank you for using Grade Manager!");
                print("Goodbye!");
                running = 0;
            } else {
                print("Invalid choice!");
            }

            i = i + 1;
        }

        return 0;
    }
}
```

## Your Turn!

Now it's time to build YOUR project!

### Success Tips

1. **Start simple** - Get the basics working first
2. **Test often** - Don't write 100 lines then test
3. **Save frequently** - Don't lose your work!
4. **Take breaks** - Fresh eyes catch bugs faster
5. **Celebrate progress** - Each feature is a win!
6. **Ask for help** - Stuck for 30 minutes? Take a break or ask
7. **Have fun** - Enjoy building something cool!

## What You've Accomplished

**YOU ARE NOW A PROGRAMMER!** 🎉

You can:
✅ Write programs from scratch
✅ Plan and organize code
✅ Use variables, arrays, functions, loops
✅ Make decisions with if/else
✅ Build interactive programs
✅ Debug problems
✅ Create real, useful software

## What's Next?

### Keep Learning
- Build more projects (practice makes perfect!)
- Learn advanced HLX features
- Study algorithms and data structures
- Explore other programming languages (you'll pick them up fast!)

### Project Ideas for the Future
- Games (card games, word games, puzzle games)
- Utilities (password generator, unit converter)
- Simulations (dice roller, coin flip statistics)
- Tools (file organizer, data processor)

### Share Your Work
- Show friends and family what you built
- Explain how it works (teaching helps you learn!)
- Get feedback and improve
- Help others learn to code

## Final Thoughts

**Programming is a superpower.** You can:
- Automate boring tasks
- Build tools to solve problems
- Create games and art
- Understand how technology works
- Express your creativity
- Help others with software

**You started with zero knowledge.** Now look at you:
- You understand variables, functions, loops
- You can read and write code
- You know how to debug problems
- You've built complete programs

**This is just the beginning.** Every programmer you admire started exactly where you started. The difference? They kept going. Keep building. Keep learning. Keep having fun!

---

## Congratulations! 🎓

You've completed **Learn to Code with HLX**!

You are now officially a programmer. Go build amazing things! 🚀

**Remember:** Every expert was once a beginner. You've got this!

---

**Final Challenge:** Build a project, then come back and build another one. And another. The more you build, the better you get!

Good luck, and happy coding! 💻✨
