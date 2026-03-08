# Issue Summary
The result type `res_type` for arithmetic operations is hardcoded as `OnuType::I64`. This can cause truncation issues when working with wider integer types added in PR-18.

## Recommendation
Use operand types to infer the required output format automatically recommending wider flexibility without breaking older datasets-statistics.