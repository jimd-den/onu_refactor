### Summary
Zero-extension (`zext`) is used for integer promotion, which is incorrect for signed numbers and can lead to security vulnerabilities. It treats the sign bit as a data bit, resulting in large positive numbers.

### Recommendation
Adopt sign-extension to handle signed integers correctly during promotion, preserving their value.