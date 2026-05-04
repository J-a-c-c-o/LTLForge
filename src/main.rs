mod builder;
mod petri_net;
mod algorithms;
mod philosophers;
mod ltl_parser;
mod pnf;
mod closure;
mod consistency;
mod gnba;
mod nba;

use builder::PetriNetBuilder;
use clap::{Parser, Subcommand};

use crate::philosophers::{PhilosopherConfiguration};


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
    Reachability {
        pnml_file: String,
    },
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
    },
    /// Produce an NBA in HOA format
    Nba {
        /// LTL specification file
        ltl_file: String,
        /// Output Graphviz DOT
        #[arg(long)]
        dot: bool,
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
        Commands::Philosophers { philosophers, mode, generic } => {
            println!("[Philosophers] Number of philosophers: {}, Mode: {}", philosophers, mode);
            let config = config_generator(mode, philosophers);
            let (states, deadlocks) =match generic {
                true => {
                    let petri_net = philosophers::build_dining_philosophers(philosophers, config.clone());
                    let stats = algorithms::get_reachability_stats(&petri_net);
                    (stats.reachable_count, stats.deadlock_count)
                }
                false => {
                    let (states, deadlocks) = philosophers::compute_reachable_states_and_deadlocks(philosophers, config.clone());
                    (states.len(), deadlocks.len())
                }
            };
            
            println!("Reachable states: {}", states);
            println!("Deadlocks: {}", deadlocks);
        }
        Commands::Reachability { pnml_file } => {
            println!("[Reachability] PNML file: {}", pnml_file);

            let petri_net = &PetriNetBuilder::build_from_file(&pnml_file)[0];

            let stats = algorithms::get_reachability_stats(&petri_net);
            println!("Reachable states: {}", stats.reachable_count);
            println!("Deadlocks: {}", stats.deadlock_count);
        }
        Commands::Check { pnml_file, ltl_file } => {
            println!("[Check] PNML file: {}, LTL file: {}", pnml_file, ltl_file);
            // TODO: Implement LTL model checking
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
        Commands::Gnba { ltl_file, dot } => {
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
                if dot {
                    let dot_str = gnba.to_dot();
                    let filename = format!("{}_gnba.dot", name);
                    std::fs::write(&filename, dot_str).expect("Unable to write DOT file");
                    println!("Wrote DOT to {}", filename);
                } else {
                    gnba.pretty_print();
                }
                println!();
                break;
                
            }
        }
        Commands::Nba { ltl_file, dot } => {
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
                if dot {
                    let dot_str = nba.to_dot();
                    let filename = format!("{}_nba.dot", name);
                    std::fs::write(&filename, dot_str).expect("Unable to write DOT file");
                    println!("Wrote DOT to {}", filename);
                } else {
                    nba.pretty_print();
                }
                println!();
                break;
            }
        }
        Commands::Sat { ltl_file } => {
            println!("[SAT] LTL file: {}", ltl_file);
            // TODO: Implement satisfiability checking
        }
        Commands::Convert { philosophers, output_file, mode } => {
            println!("[Convert] Philosophers: {}, Output file: {}", philosophers, output_file);
            let petri_net = philosophers::build_dining_philosophers(philosophers, config_generator(mode, philosophers));
            let pnml_content = petri_net.to_pnml();
            std::fs::write(output_file, pnml_content).expect("Unable to write file");
        }

    }
}


fn config_generator(mode: usize, n: usize) -> Vec<PhilosopherConfiguration> {
    let mut config = Vec::new();

    for i in 0..n {
        let philosopher_config = match mode {
            0 => PhilosopherConfiguration { allowed_left: true, allowed_right: false },
            1 => PhilosopherConfiguration { allowed_left: true, allowed_right: true },
            2 => if i == 0 {
                PhilosopherConfiguration { allowed_left: true, allowed_right: false }
            } else {
                PhilosopherConfiguration { allowed_left: false, allowed_right: true }
            },
            _ => panic!("Invalid mode"),
        };
        config.push(philosopher_config);
    }

    config
}


