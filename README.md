# LTL Forge

## About

This repository provides a high-performance, command-line toolchain written in Rust for parsing, translating, and checking Linear Temporal Logic (LTL) specifications against Petri nets.

The project is structured as a Cargo workspace and includes implementations of a complete explicit-state LTL model checking pipeline:

- **Reachability Analysis:** Compute reachable markings and detect deadlocks in standard PNML Petri net files.
- **LTL Translation Pipeline:** Parse LTL formulas and rigorously translate them into Positive Normal Form (PNF), Generalized Non-deterministic Büchi Automata (GNBA), and standard Non-deterministic Büchi Automata (NBA).
- **Satisfiability & Model Checking:** Perform full LTL satisfiability checking and verify properties against complex Petri net models using on-the-fly synchronous product graph exploration.
- **Case Study Generation:** Built-in generators for classical concurrency problems, including highly configurable variants of the Dining Philosophers problem.

---

## Project Structure

The project is organized as a Rust workspace containing the core library, the CLI application, and example models:

```text
.
├── Cargo.toml               # Workspace root configuration
├── Cargo.lock
├── cli/                     # The primary command-line interface
│   ├── Cargo.toml
│   └── src/
│       └── main.rs          # CLI entry point and argument parsing
├── ltl_forge/               # The core library containing all logic
│   ├── Cargo.toml
│   └── src/
│       ├── builder.rs       # Automata and graph builders
│       ├── closure.rs       # Subformula closure computations
│       ├── consistency.rs   # Logic consistency checks
│       ├── emptyness.rs     # Emptiness checks for product graphs
│       ├── explorer.rs      # State space exploration
│       ├── gnba.rs          # Generalized Büchi Automaton logic
│       ├── lib.rs
│       ├── ltl_parser.rs    # AST parsing for LTL text
│       ├── nba.rs           # Normal Büchi Automaton logic
│       ├── petri_net.rs     # PNML interpretation and state management
│       ├── philosophers.rs  # Dining Philosophers generator
│       └── pnf.rs           # Positive Normal Form translations
├── example/                 # Example models and properties
│   ├── airplane/
│   │   ├── airplane.pnml
│   │   └── prop.mcc
│   ├── small/
│   │   ├── net.pnml
│   │   └── prop.mcc
│   └── traffic_light/
│       ├── net.pnml
│       └── prop.mcc
└── README.md

```

---

## Installation Instructions

### 1. Prerequisites

Ensure you have the Rust toolchain installed. If you do not have Rust installed, you can install it via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

_(Optional but Recommended)_: To render automata directly to PNG images via the CLI, ensure you have **Graphviz** installed on your system.

- **Ubuntu/Debian:** `sudo apt install graphviz`
- **macOS:** `brew install graphviz`
- **Windows:** Download from [graphviz.org](https://graphviz.org/download/)

### 2. Clone the Repository

```bash
git clone https://github.com/YOUR_USERNAME/ltlforge.git
cd ltlforge
```

### 3. Build the Project

Because model checking is a compute-heavy process, it is highly recommended to build and run the tools in `--release` mode for maximum optimization.

```bash
cargo build --release
```

---

## Usage Examples

You can run the different commands from the root directory using Cargo. Below are common examples utilizing the provided `example/` directory.

### Reachability & Deadlock Detection

Compute the reachable state space and find all deadlocks in a given Petri net.

```bash
cargo run --release -p cli -- reachability example/small/net.pnml
```

### LTL Translation Pipeline

You can translate LTL properties step-by-step to inspect the AST, PNF, GNBA, and NBA.

**Convert to Positive Normal Form (PNF):**

```bash
cargo run --release -p cli -- pnf example/small/prop.mcc
```

**Generate a Generalized Büchi Automaton (GNBA):**
_Outputs in HOA format by default. You can also render and view it directly as a graph._

```bash
cargo run --release -p cli -- gnba --png --view example/small/prop.mcc
```

**Generate a standard Non-deterministic Büchi Automaton (NBA):**

```bash
cargo run --release -p cli -- nba --dot example/small/prop.mcc
```

### Satisfiability & Model Checking

**Check if an LTL formula is satisfiable:**

```bash
cargo run --release -p cli -- sat --show-counterexample example/small/prop.mcc
```

**Verify an LTL specification against a Petri net:**
_Includes resource limit configurations for deep state-space searches._

```bash
cargo run --release -p cli -- check example/small/net.pnml example/small/prop.mcc --timeout 300 --memory-limit 4096
```

### Dining Philosophers Generator

The tool includes a built-in state space generator for the Dining Philosophers problem to test scalability and performance limits.

The `<MODE>` parameter alters the behavior rules:

- `0`: All philosophers pick up the same side first.
- `1`: Arbitrary (any philosopher can pick up any available fork).
- `2`: Asymmetric (one left, others right).

**Run the problem directly (e.g., 5 philosophers, arbitrary mode):**

```bash
cargo run --release -p cli -- philosophers 5 1
```

**Convert a specific configuration into a standard PNML file:**

```bash
cargo run --release -p cli -- convert 5 1 output.pnml
```

---

## License

This project is dual-licensed under either the [MIT license](https://www.google.com/search?q=LICENSE-MIT) or the [Apache License, Version 2.0](https://www.google.com/search?q=LICENSE-APACHE), at your option.
