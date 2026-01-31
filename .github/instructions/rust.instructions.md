---
applyTo: '**'
---
code should be readable. that means using clear and descriptive names for variables, functions, and types. Avoid overly complex constructs when simpler ones will do.

prefer cargo check over lint errors for quick feedback during development. Use clippy for more thorough linting before commits.

don't author big files. Break them into smaller modules to improve maintainability and readability.

comment your code where necessary, especially for complex logic. However, avoid redundant comments that do not add value. 

Do not make changelog style comments.

document public APIs properly and make sure to update documentation when APIs change.


code should be robust, maintain integrity, modular, readable and performant and efficient.


we're in embedded systems, so be mindful of resource constraints like memory and processing power
and speed.

use clever optimization tricks when necessary.