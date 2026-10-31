# Keep guarded inventory outside the literal Cargo step

The composition list/count/run block introduced in 2de2b627d1656c50adfc99ab75f4f3242e354c29 violates the existing workflow contract that every command in the native witness step starts with Cargo. The selected Python test passes against the clean PR base and fails against the published workflow.

Move the complete block into its own native-profile step with explicit Bash, preserving literal Cargo flags, the exact five-case count and actual guarded execution. All neighboring source-profile selections stay in their original step. No test, inventory check, warning policy or resource allowance is weakened.

Configured CI Python runs 132 full-workflow/inventory tests successfully. The retained maintained Rust unit binary lists exactly five matching tests and executes all five successfully. This is the published selection before the new cleanup witness; its later count update belongs to that separate compiler change. The system Python attempt lacks the pinned YAML dependency, so validation uses the existing configured CI environment. Exact commands and outputs are archived.
