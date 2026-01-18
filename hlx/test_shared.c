#include <stdio.h>
#include "test_ffi_lib.h"

int main() {
    printf("Testing shared library linking...\n\n");

    int64_t sum = add(50, 75);
    printf("add(50, 75) = %ld\n", sum);

    int64_t product = multiply(12, 13);
    printf("multiply(12, 13) = %ld\n", product);

    // Test chain
    int64_t chain = add(multiply(5, 5), add(10, 10));
    printf("add(multiply(5, 5), add(10, 10)) = %ld\n", chain);

    printf("\nShared library test successful!\n");
    return 0;
}
