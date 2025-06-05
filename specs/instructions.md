# Instructions

Please look into @/grpcurl , examine every go file (except *_test.go), and learn the arch & design of the software. Then based on your learnings, generate two files under ./specs:

1. the product requirement doc: list all the features grpcurl supports, and their detailed explanation and usage.
2. the rust design doc: generate relevant rust data structure, trait. Also the design charts using mermaid. Make sure you use latest clap and tonic / tonic-reflection. Please follow @rust/core/design-patterns.mdc @rust/core/code-quality.mdc @dependencies.mdc @rust/core/type-system.mdc

Let's call this rust version GrpcClient. Please do not make it too flexible. Focus on a clean design on Rust. Please update specs accordingly.
