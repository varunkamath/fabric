TODOs for library parity:

1. Implement a similar plugin system in the Python version.
2. Standardize the configuration and data structures between both versions.
3. Align the error handling approaches more closely.
4. Ensure that all features available in one version are also available in the other.

To note:

1. Plugin Management:
   The plugin system is implemented differently in Rust and Python. Ensure that node types are registered correctly in both languages if you want to use them interchangeably.
2. Performance Differences:
   Rust nodes might perform better for CPU-intensive tasks, while Python nodes might be easier to prototype and modify quickly.
3. Deployment Complexity:
   Managing dependencies and runtime environments for both Rust and Python in the same deployment might increase operational complexity.
4. Error Handling:
   While both use custom error types (FabricError), the specific error scenarios might differ between languages. Ensure consistent error handling and reporting across both implementations.
5. Async Runtime Differences:
   Rust uses tokio, while Python uses asyncio. While both are async runtimes, there might be subtle differences in behavior or performance.
   To ensure smooth integration:
   Thoroughly test cross-language communication and data exchange.
   Maintain consistent versioning and feature parity between Rust and Python implementations.
   Document any language-specific behaviors or limitations clearly.
   Consider creating a common test suite that can be run against both implementations to ensure consistent behavior.
