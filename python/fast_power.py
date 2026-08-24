def fast_power(base: int, exponent: int) -> int:
    """Calculates base^exponent efficiently using exponentiation by squaring.

    Args:
        base: The base integer.
        exponent: The non-negative exponent integer.

    Returns:
        The result of base raised to the power of exponent.
    """
    if exponent < 0:
        raise ValueError("Exponent must be non-negative")
    if exponent == 0:
        return 1
    if exponent == 1:
        return base

    result = 1
    while exponent > 0:
        if exponent % 2 == 1:
            result *= base
        base *= base
        exponent //= 2
    return result

# Example usage:
# print(fast_power(2, 10))  # Output: 1024
# print(fast_power(3, 5))   # Output: 243
# print(fast_power(7, 0))   # Output: 1