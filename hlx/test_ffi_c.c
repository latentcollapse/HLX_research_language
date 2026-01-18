#include <stdio.h>
#include <stdint.h>

// Forward declarations for HLX functions
extern int64_t add(int64_t a, int64_t b);
extern int64_t multiply(int64_t x, int64_t y);

int main() {
    int64_t result_add = add(10, 20);
    printf("add(10, 20) = %ld\n", result_add);

    int64_t result_mul = multiply(5, 6);
    printf("multiply(5, 6) = %ld\n", result_mul);

    return 0;
}
