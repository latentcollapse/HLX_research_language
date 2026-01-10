# Chapter 8: Your First Game - Number Guessing

Time to build something fun! You'll create a complete, working game using everything you've learned.

## The Game

**How it works:**
1. Computer picks a secret number (1-100)
2. Player guesses
3. Computer says "too high" or "too low"
4. Player keeps guessing until correct
5. Game shows how many guesses it took

**Example gameplay:**
```
===== NUMBER GUESSING GAME =====
I'm thinking of a number between 1 and 100
Guess #1: 50
Too high!
Guess #2: 25
Too low!
Guess #3: 37
Too high!
Guess #4: 31
Correct! You won in 4 guesses!
```

## Version 1: Basic Game (Simulated Input)

Since HLX doesn't have user input yet, we'll simulate guesses with an array:

```hlx
program guessing_game_v1 {
    fn main() {
        print("===== NUMBER GUESSING GAME =====");
        print("I'm thinking of a number between 1 and 100");
        print("");

        let secret = 31;  // The secret number
        let guesses = [50, 25, 37, 31];  // Simulated guesses
        let guess_count = 0;
        let found = 0;
        let i = 0;

        loop (i < 4, 10) {
            if (found == 1) {
                break;  // Already won
            }

            guess_count = guess_count + 1;
            let guess = guesses[i];

            print("Guess #");
            print(guess_count);
            print(":");
            print(guess);

            if (guess == secret) {
                print("Correct! You won in");
                print(guess_count);
                print("guesses!");
                found = 1;
            } else if (guess > secret) {
                print("Too high!");
            } else {
                print("Too low!");
            }

            print("");
            i = i + 1;
        }

        if (found == 0) {
            print("Out of guesses! The number was:");
            print(secret);
        }

        return 0;
    }
}
```

**Output:**
```
"===== NUMBER GUESSING GAME ====="
"I'm thinking of a number between 1 and 100"
""
"Guess #"
1
":"
50
"Too high!"
""
"Guess #"
2
":"
25
"Too low!"
""
"Guess #"
3
":"
37
"Too high!"
""
"Guess #"
4
":"
31
"Correct! You won in"
4
"guesses!"
0
```

## Breaking It Down

Let's understand each part:

### Setup
```hlx
let secret = 31;              // Number to guess
let guesses = [50, 25, 37, 31];  // Player's guesses
let guess_count = 0;          // How many guesses so far
let found = 0;                // Have we found it? (0=no, 1=yes)
let i = 0;                    // Loop counter
```

### Game Loop
```hlx
loop (i < 4, 10) {
    if (found == 1) {
        break;  // Stop if already won
    }

    guess_count = guess_count + 1;
    let guess = guesses[i];
    // ... process guess ...
}
```

### Checking the Guess
```hlx
if (guess == secret) {
    // Correct!
} else if (guess > secret) {
    // Too high
} else {
    // Too low
}
```

## Version 2: With Functions

Let's organize with functions:

```hlx
program guessing_game_v2 {
    fn print_header() {
        print("===== NUMBER GUESSING GAME =====");
        print("I'm thinking of a number between 1 and 100");
        print("");
    }

    fn print_guess(count, value) {
        print("Guess #");
        print(count);
        print(":");
        print(value);
    }

    fn check_guess(guess, secret) {
        if (guess == secret) {
            return 0;  // Correct
        } else if (guess > secret) {
            return 1;  // Too high
        } else {
            return -1;  // Too low
        }
    }

    fn print_result(result, count) {
        if (result == 0) {
            print("Correct! You won in");
            print(count);
            print("guesses!");
        } else if (result == 1) {
            print("Too high!");
        } else {
            print("Too low!");
        }
    }

    fn main() {
        print_header();

        let secret = 31;
        let guesses = [50, 25, 37, 31];
        let guess_count = 0;
        let won = 0;
        let i = 0;

        loop (i < 4, 10) {
            if (won == 1) {
                break;
            }

            guess_count = guess_count + 1;
            let guess = guesses[i];

            print_guess(guess_count, guess);

            let result = check_guess(guess, secret);
            print_result(result, guess_count);

            if (result == 0) {
                won = 1;
            }

            print("");
            i = i + 1;
        }

        if (won == 0) {
            print("Out of guesses! The number was:");
            print(secret);
        }

        return 0;
    }
}
```

**Much cleaner!** Each function has one job.

## Version 3: Difficulty Levels

Add easy/medium/hard modes:

```hlx
program guessing_game_v3 {
    fn get_max_number(difficulty) {
        if (difficulty == 1) {
            return 50;   // Easy: 1-50
        } else if (difficulty == 2) {
            return 100;  // Medium: 1-100
        } else {
            return 200;  // Hard: 1-200
        }
    }

    fn get_max_guesses(difficulty) {
        if (difficulty == 1) {
            return 10;  // Easy: 10 guesses
        } else if (difficulty == 2) {
            return 7;   // Medium: 7 guesses
        } else {
            return 5;   // Hard: 5 guesses
        }
    }

    fn print_difficulty_menu() {
        print("Select difficulty:");
        print("1. Easy (1-50, 10 guesses)");
        print("2. Medium (1-100, 7 guesses)");
        print("3. Hard (1-200, 5 guesses)");
        print("");
    }

    fn check_guess(guess, secret) {
        if (guess == secret) {
            return 0;
        } else if (guess > secret) {
            return 1;
        } else {
            return -1;
        }
    }

    fn main() {
        print("===== NUMBER GUESSING GAME =====");
        print("");

        print_difficulty_menu();

        let difficulty = 2;  // Medium
        let max_num = get_max_number(difficulty);
        let max_guesses = get_max_guesses(difficulty);

        print("Playing on");
        if (difficulty == 1) {
            print("EASY");
        } else if (difficulty == 2) {
            print("MEDIUM");
        } else {
            print("HARD");
        }
        print("");

        print("I'm thinking of a number between 1 and");
        print(max_num);
        print("");

        let secret = 31;
        let guesses = [50, 25, 37, 31];
        let guess_count = 0;
        let won = 0;
        let i = 0;

        loop (i < max_guesses, 20) {
            if (won == 1) {
                break;
            }

            guess_count = guess_count + 1;
            let guess = guesses[i];

            print("Guess #");
            print(guess_count);
            print(":");
            print(guess);

            let result = check_guess(guess, secret);

            if (result == 0) {
                print("Correct! You won in");
                print(guess_count);
                print("guesses!");
                won = 1;
            } else if (result == 1) {
                print("Too high!");
            } else {
                print("Too low!");
            }

            print("");
            i = i + 1;
        }

        if (won == 0) {
            print("Out of guesses! The number was:");
            print(secret);
        }

        return 0;
    }
}
```

## Version 4: Score System

Add scoring based on how many guesses:

```hlx
program guessing_game_v4 {
    fn calculate_score(guess_count, max_guesses) {
        // Better score for fewer guesses
        // Score: (max_guesses - guess_count + 1) * 10
        let remaining = max_guesses - guess_count + 1;
        return remaining * 10;
    }

    fn get_rank(score) {
        if (score >= 50) {
            return 1;  // Master
        } else if (score >= 30) {
            return 2;  // Expert
        } else if (score >= 10) {
            return 3;  // Good
        } else {
            return 4;  // Beginner
        }
    }

    fn print_rank(rank) {
        if (rank == 1) {
            print("Rank: MASTER");
        } else if (rank == 2) {
            print("Rank: EXPERT");
        } else if (rank == 3) {
            print("Rank: GOOD");
        } else {
            print("Rank: BEGINNER");
        }
    }

    fn check_guess(guess, secret) {
        if (guess == secret) {
            return 0;
        } else if (guess > secret) {
            return 1;
        } else {
            return -1;
        }
    }

    fn main() {
        print("===== NUMBER GUESSING GAME =====");
        print("");

        let secret = 31;
        let max_guesses = 7;
        let guesses = [50, 25, 37, 31];
        let guess_count = 0;
        let won = 0;
        let i = 0;

        print("I'm thinking of a number between 1 and 100");
        print("");

        loop (i < max_guesses, 20) {
            if (won == 1) {
                break;
            }

            guess_count = guess_count + 1;
            let guess = guesses[i];

            print("Guess #");
            print(guess_count);
            print(":");
            print(guess);

            let result = check_guess(guess, secret);

            if (result == 0) {
                print("Correct!");
                print("");

                // Calculate score
                let score = calculate_score(guess_count, max_guesses);
                print("Guesses:");
                print(guess_count);
                print("Score:");
                print(score);

                let rank = get_rank(score);
                print_rank(rank);

                won = 1;
            } else if (result == 1) {
                print("Too high!");
            } else {
                print("Too low!");
            }

            print("");
            i = i + 1;
        }

        if (won == 0) {
            print("Out of guesses! The number was:");
            print(secret);
            print("Score: 0");
            print("Rank: BEGINNER");
        }

        return 0;
    }
}
```

**Output (winning in 4 guesses):**
```
"===== NUMBER GUESSING GAME ====="
""
"I'm thinking of a number between 1 and 100"
""
"Guess #"
1
":"
50
"Too high!"
""
"Guess #"
2
":"
25
"Too low!"
""
"Guess #"
3
":"
37
"Too high!"
""
"Guess #"
4
":"
31
"Correct!"
""
"Guesses:"
4
"Score:"
40
"Rank: EXPERT"
0
```

## Practice: Add Features!

Try adding these features:

### Feature 1: Hint System
After 3 wrong guesses, give a hint:
- "The number is even/odd"
- "The number is in the range 20-40"

### Feature 2: Statistics
Track:
- Total games played
- Total wins
- Average guesses per win
- Best score

### Feature 3: Binary Search Hint
Teach the player strategy:
- "Try 50 (middle of 1-100)"
- "Try 25 (middle of 1-50)"
- This is the optimal strategy!

## What You Built

✅ Complete working game
✅ User interface (menu, prompts, messages)
✅ Game logic (checking guesses)
✅ Difficulty levels
✅ Scoring system
✅ Organized with functions

## What You Learned

✅ Building a complete program from scratch
✅ Breaking down a big problem into functions
✅ Managing game state (variables tracking progress)
✅ Creating user-friendly output
✅ Adding features incrementally
✅ Testing and debugging

## Next Steps

In **Chapter 9**, you'll build a **calculator program** with a menu system!

Preview:
```
===== CALCULATOR =====
1. Add
2. Subtract
3. Multiply
4. Divide
5. Power
6. Quit
Enter choice: 1
Enter first number: 10
Enter second number: 5
Result: 15
```

Another real, useful program! 🧮

---

**Challenge:** Before moving on, try to add at least ONE new feature to the game. Ideas:
- Custom messages for different score levels
- "You're getting closer!" messages
- A "give up" option
- Multiple rounds with cumulative score

Get creative! 🎨
