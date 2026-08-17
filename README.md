# VietCalendar (Rust)

Welcome to the Rust port of the **VietCalendar** API! 🎉 

This project was successfully and fully migrated from its original Java 11 / Maven Vert.x codebase to a high-performance, containerized Rust web application built smoothly on top of **Axum** and **Tokio**.

## Highlights of Migration
* **100% Logic Parity**: Mathematical algorithm implementations corresponding to the Julian and Luni-Solar calendar translations (`de.unileipzig.informatik.VietCalendar.java`) were preserved with identical logic. (The original Javadoc implementations were deeply preserved and integrated directly inside the new Rust source).
* **High-Speed Execution**: Replaced the Vert.x EventBus infrastructure with high-throughput, non-blocking HTTP threading through Tokyo scheduling.
* **Modernized Deployment**: Operations were built around a generic Docker container targeting zero-setup deployments mapping any dynamic `$PORT` via environment variables.
* **Preserved Unit Testing**: All core logic assertions originally built via JUnit are successfully encapsulated inside the new built-in test suites, executed using `cargo test`.

For a designated architectural breakdown mapping the original structures to their new modules, design decisions, and verification matrix, refer to the included [`ARCHITECTURE_DECISIONS.md`](ARCHITECTURE_DECISIONS.md) file!
