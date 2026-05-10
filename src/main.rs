mod builder;
mod closure;
mod consistency;
mod emptyness;
mod explorer;
mod gnba;
mod ltl_parser;
mod model_check;
mod nba;
mod petri_net;
mod philosophers;
mod pnf;

use builder::PetriNetBuilder;
use clap::{Parser, Subcommand};
use std::process::Command;

use crate::philosophers::PhilosopherConfiguration;

/// LTL Model Checking Toolbox
#[derive(Parser)]
#[command(name = "ltltools")]
#[command(about = "A toolbox for LTL model checking on Petri nets", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compute reachable markings and deadlocks
    Reachability { pnml_file: String },
    /// Check LTL specification on Petri net
    Check {
        /// PNML file
        pnml_file: String,
        /// LTL specification file
        ltl_file: String,
    },
    /// Convert LTL formula to positive normal form
    Pnf {
        /// LTL specification file
        ltl_file: String,
    },
    /// Produce a GNBA in HOA format
    Gnba {
        /// LTL specification file
        ltl_file: String,
        /// Output Graphviz DOT
        #[arg(long)]
        dot: bool,

        /// Generate PNG with dot (requires dot to be installed and in PATH)
        #[arg(long)]
        png: bool,

        /// Open generated PNG after creating it with dot
        #[arg(long)]
        view: bool,

        /// Custom command to open the generated file (e.g. "feh", "xdg-open", "open")
        #[arg(long)]
        viewer: Option<String>,
    },
    /// Produce an NBA in HOA format
    Nba {
        /// LTL specification file
        ltl_file: String,

        /// Output Graphviz DOT
        #[arg(long)]
        dot: bool,

        /// Generate PNG with dot (requires dot to be installed and in PATH)
        #[arg(long)]
        png: bool,

        /// Open generated PNG after creating it with dot
        #[arg(long)]
        view: bool,

        /// Custom command to open the generated file (e.g. "feh", "xdg-open", "open")
        #[arg(long)]
        viewer: Option<String>,
    },
    /// Check satisfiability of LTL specification
    Sat {
        /// LTL specification file
        ltl_file: String,
    },
    /// Runs the Philosophers problem with the given configurations and computes reachable states and deadlocks
    Philosophers {
        /// Number of philosophers for the dining philosophers problem
        philosophers: usize,

        /// Mode for the dining philosophers problem (0: philosophers pickup first the left fork, 1: philosophers pickup either fork first 2: one philosopher picks up left fork first, the other right fork first)
        mode: usize,

        /// Use generic engine to compute reachable states and deadlocks
        #[arg(long, default_value_t = false)]
        generic: bool,
    },
    /// Convert the dining philosophers problem to a PNML file
    Convert {
        /// Input PhiloSophers number
        philosophers: usize,

        /// Mode for the dining philosophers problem (0: philosophers pickup first the left fork, 1: philosophers pickup either fork first 2: one philosopher picks up left fork first, the other right fork first)
        mode: usize,

        // Output PNML file
        output_file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Philosophers {
            philosophers,
            mode,
            generic,
        } => {
            println!(
                "[Philosophers] Number of philosophers: {}, Mode: {}",
                philosophers, mode
            );
            let config = config_generator(mode, philosophers);
            let (states, deadlocks) = match generic {
                true => {
                    let petri_net =
                        philosophers::build_dining_philosophers(philosophers, config.clone());
                    let stats = explorer::get_reachability_stats(&petri_net);
                    (stats.reachable_count, stats.deadlock_count)
                }
                false => {
                    let (states, deadlocks) = philosophers::compute_reachable_states_and_deadlocks(
                        philosophers,
                        config.clone(),
                    );
                    (states.len(), deadlocks.len())
                }
            };

            println!("Reachable states: {}", states);
            println!("Deadlocks: {}", deadlocks);
        }
        Commands::Reachability { pnml_file } => {
            println!("[Reachability] PNML file: {}", pnml_file);

            let petri_net = &PetriNetBuilder::build_from_file(&pnml_file)[0];

            let stats = explorer::get_reachability_stats(petri_net);
            println!("Reachable states: {}", stats.reachable_count);
            println!("Deadlocks: {}", stats.deadlock_count);
        }
        Commands::Check {
            pnml_file,
            ltl_file,
        } => {
            println!("[Check] PNML file: {}, LTL file: {}", pnml_file, ltl_file);
            let petri_nets = PetriNetBuilder::build_from_file(&pnml_file);
            if petri_nets.is_empty() {
                eprintln!("No Petri nets found in file: {}", pnml_file);
                return;
            }
            let petri_net = &petri_nets[0];

            let result = ltl_parser::parse_mcc_file(&ltl_file);
            let formulas = match result {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Error parsing LTL file: {}", e);
                    return;
                }
            };

            for (name, formula) in formulas {
                println!("[{}] Checking: {}", name, formula);
                let holds = model_check::model_check(petri_net, &formula);
                if holds {
                    println!("Property {} holds on the given Petri net.", name);
                } else {
                    println!("Property {} is violated (counterexample exists).", name);
                }
                println!();
            }
        }
        Commands::Pnf { ltl_file } => {
            println!("[PNF] LTL file: {}", ltl_file);
            let result = ltl_parser::parse_mcc_file(&ltl_file);
            let formulas = match result {
                Ok(formulas) => formulas,
                Err(e) => {
                    eprintln!("Error parsing LTL file: {}", e);
                    return;
                }
            };
            for (name, formula) in formulas {
                println!("[{}]", name);
                println!("Original formula: {}", formula);
                let pnf_formula = pnf::to_pnf(&formula);
                println!("PNF formula: {}", pnf_formula);
                println!();
            }
        }
        Commands::Gnba {
            ltl_file,
            dot,
            png,
            view,
            viewer,
        } => {
            println!("[GNBA] LTL file: {}", ltl_file);
            let result = ltl_parser::parse_mcc_file(&ltl_file);
            let formulas = match result {
                Ok(formulas) => formulas,
                Err(e) => {
                    eprintln!("Error parsing LTL file: {}", e);
                    return;
                }
            };
            for (name, formula) in formulas {
                println!("[{}]", name);
                println!("Original formula: {}", formula);
                let gnba = gnba::GNBA::new(&formula);
                if dot || png || view || viewer.is_some() {
                    let dot_str = gnba.to_dot();
                    let filename = format!("output/{}_gnba.dot", name);
                    std::fs::create_dir_all("output").expect("Failed to create output directory");
                    std::fs::write(&filename, dot_str).expect("Unable to write DOT file");
                    println!("Wrote DOT to {}", filename);

                    let dot_command_is_available = is_command_available("dot");
                    if dot_command_is_available && (png || view || viewer.is_some()) {
                        let output_png = format!("output/{}_gnba.png", name);
                        let status = Command::new("dot")
                            .args(["-Tpng", &filename, "-o", &output_png])
                            .status();

                        match status {
                            Ok(s) if s.success() => {
                                println!("Generated PNG with dot: {}", output_png);

                                if view || viewer.is_some() {
                                    if let Some(viewer) = &viewer {
                                        open::with(&output_png, viewer)
                                            .expect("Failed to open PNG with custom viewer");
                                    } else {
                                        open::that(&output_png).expect("Failed to open PNG file");
                                    }
                                }
                            }
                            Ok(s) => {
                                eprintln!("dot failed with exit code: {:?}", s.code());
                            }
                            Err(e) => {
                                eprintln!("Failed to run dot: {}", e);
                            }
                        }
                    }
                } else {
                    gnba.pretty_print();
                }
                println!();
            }
        }
        Commands::Nba {
            ltl_file,
            dot,
            png,
            view,
            viewer,
        } => {
            println!("[NBA] LTL file: {}", ltl_file);
            let result = ltl_parser::parse_mcc_file(&ltl_file);
            let formulas = match result {
                Ok(formulas) => formulas,
                Err(e) => {
                    eprintln!("Error parsing LTL file: {}", e);
                    return;
                }
            };
            for (name, formula) in formulas {
                println!("[{}]", name);
                println!("Original formula: {}", formula);
                let nba = nba::NBA::new(&formula);
                if dot || png || view || viewer.is_some() {
                    let dot_str = nba.to_dot();
                    let filename = format!("output/{}_nba.dot", name);
                    std::fs::create_dir_all("output").expect("Failed to create output directory");
                    std::fs::write(&filename, dot_str).expect("Unable to write DOT file");
                    println!("Wrote DOT to {}", filename);

                    let dot_command_is_available = is_command_available("dot");
                    if dot_command_is_available && (png || view || viewer.is_some()) {
                        let output_png = format!("output/{}_nba.png", name);
                        let status = Command::new("dot")
                            .args(["-Tpng", &filename, "-o", &output_png])
                            .status();

                        match status {
                            Ok(s) if s.success() => {
                                println!("Generated PNG with dot: {}", output_png);

                                if view || viewer.is_some() {
                                    if let Some(viewer) = &viewer {
                                        open::with(&output_png, viewer)
                                            .expect("Failed to open PNG with custom viewer");
                                    } else {
                                        open::that(&output_png).expect("Failed to open PNG file");
                                    }
                                }
                            }
                            Ok(s) => {
                                eprintln!("dot failed with exit code: {:?}", s.code());
                            }
                            Err(e) => {
                                eprintln!("Failed to run dot: {}", e);
                            }
                        }
                    }
                } else {
                    nba.pretty_print();
                }
                println!();
            }
        }
        Commands::Sat { ltl_file } => {
            println!("[SAT] LTL file: {}", ltl_file);
            let result = ltl_parser::parse_mcc_file(&ltl_file);
            let formulas = match result {
                Ok(formulas) => formulas,
                Err(e) => {
                    eprintln!("Error parsing LTL file: {}", e);
                    return;
                }
            };
            for (name, formula) in formulas {
                println!("[{}]", name);
                println!("Original formula: {}", formula);
                let (nba_is_satisfiable, gnba_is_satisfiable) = emptyness::is_satisfiable(&formula);
                println!("NBA Satisfiable: {}", nba_is_satisfiable);
                println!("GNBA Satisfiable: {}", gnba_is_satisfiable);
                println!();
            }
        }
        Commands::Convert {
            philosophers,
            output_file,
            mode,
        } => {
            println!(
                "[Convert] Philosophers: {}, Output file: {}",
                philosophers, output_file
            );
            let petri_net = philosophers::build_dining_philosophers(
                philosophers,
                config_generator(mode, philosophers),
            );
            let pnml_content = petri_net.to_pnml();
            std::fs::write(output_file, pnml_content).expect("Unable to write file");
        }
    }
}

fn is_command_available(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .output()
        .is_ok()
}

fn config_generator(mode: usize, n: usize) -> Vec<PhilosopherConfiguration> {
    let mut config = Vec::new();

    for i in 0..n {
        let philosopher_config = match mode {
            0 => PhilosopherConfiguration {
                allowed_left: true,
                allowed_right: false,
            },
            1 => PhilosopherConfiguration {
                allowed_left: true,
                allowed_right: true,
            },
            2 => {
                if i == 0 {
                    PhilosopherConfiguration {
                        allowed_left: true,
                        allowed_right: false,
                    }
                } else {
                    PhilosopherConfiguration {
                        allowed_left: false,
                        allowed_right: true,
                    }
                }
            }
            _ => panic!("Invalid mode"),
        };
        config.push(philosopher_config);
    }

    config
}
