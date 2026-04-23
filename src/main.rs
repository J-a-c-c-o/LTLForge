mod builder;
mod petri_net;
mod algorithms;
mod philosophers;
mod ltl_parser;
mod pnf;

use builder::PetriNetBuilder;
use clap::{Parser, Subcommand, Args};

/// LTL Model Checking Toolbox
#[derive(Parser)]
#[command(name = "ltltools")]
#[command(about = "A toolbox for LTL model checking on Petri nets", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}


#[derive(Args)]
struct PetriNetSource {
    /// Input PNML file
    #[arg(short = 'f', long, group = "source")]
    pnml_file: Option<String>,

    /// Number of philosophers for the dining philosophers problem
    #[arg(short = 'p', long, group = "source")]
    philosophers: Option<usize>,
}

#[derive(Subcommand)]
enum Commands {
    /// Compute reachable markings and deadlocks
    Reachability {
        #[command(flatten)]
        source: PetriNetSource,
    },
    /// Check LTL specification on Petri net
    Check {
        #[command(flatten)]
        source: PetriNetSource,
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
    },
    /// Produce an NBA in HOA format
    Nba {
        /// LTL specification file
        ltl_file: String,
    },
    /// Check satisfiability of LTL specification
    Sat {
        /// LTL specification file
        ltl_file: String,
    },
    Convert {
        /// Input PhiloSophers number
        #[arg(short = 'p', long)]
        philosophers: usize,

        // Output PNML file        
        #[arg(short = 'f', long)]
        output_file: String,
    },
}



fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Reachability { source } => {
            match (&source.pnml_file, &source.philosophers) {
                (Some(pnml), None) => println!("[Reachability] PNML file: {}", pnml),
                (None, Some(n)) => println!("[Reachability] Philosophers: {}", n),
                _ => println!("Please provide either --pnml-file <FILE> or --philosophers <N> (not both)."),
            }

            let petri_net = get_petri_net_from_source(&source);

            let stats = algorithms::get_reachability_stats(&petri_net);
            println!("Reachable states: {}", stats.reachable_count);
            println!("Deadlocks: {}", stats.deadlock_count);
        }
        Commands::Check { source, ltl_file } => {
            match (&source.pnml_file, &source.philosophers) {
                (Some(pnml), None) => println!("[Check] PNML file: {}, LTL file: {}", pnml, ltl_file),
                (None, Some(n)) => println!("[Check] Philosophers: {}, LTL file: {}", n, ltl_file),
                _ => println!("Please provide either --pnml-file <FILE> or --philosophers <N> (not both)."),
            }
            // TODO: Implement LTL model checking
        }
        Commands::Pnf { ltl_file } => {
            println!("[PNF] LTL file: {}", ltl_file);
            // TODO: Implement PNF conversion
        }
        Commands::Gnba { ltl_file } => {
            println!("[GNBA] LTL file: {}", ltl_file);
            // TODO: Implement GNBA generation
        }
        Commands::Nba { ltl_file } => {
            println!("[NBA] LTL file: {}", ltl_file);
            // TODO: Implement NBA generation
        }
        Commands::Sat { ltl_file } => {
            println!("[SAT] LTL file: {}", ltl_file);
            // TODO: Implement satisfiability checking
        }
        Commands::Convert { philosophers, output_file } => {
            println!("[Convert] Philosophers: {}, Output file: {}", philosophers, output_file);
            let petri_net = philosophers::build_dining_philosophers(philosophers);
            let pnml_content = petri_net.to_pnml();
            std::fs::write(output_file, pnml_content).expect("Unable to write file");
        }

    }
}


fn get_petri_net_from_source(source: &PetriNetSource) -> petri_net::PetriNet {
    match (&source.pnml_file, &source.philosophers) {
        (Some(pnml), None) => PetriNetBuilder::build_from_file(pnml)[0].clone(),
        (None, Some(n)) => philosophers::build_dining_philosophers(*n),
        _ => panic!("Invalid source configuration"),
    }
}


