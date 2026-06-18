// src/main.rs

use clap::{Parser, Subcommand};
use colored::*;
use std::process::Command;

// Import your domain components from your library crate target
use ltl_forge::{
    builder::PetriNetBuilder, emptyness, explorer, gnba, ltl_parser, nba, petri_net::PetriNet,
    philosophers, pnf,
};

/// LTL Model Checking Toolbox
#[derive(Parser)]
#[command(name = "LTLForge")]
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
        /// Show counterexample paths and cycles if property is violated
        #[arg(long, short)]
        show_counterexample: bool,
        /// Timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,
        /// Memory limit in MB
        #[arg(long, default_value = "4096")]
        memory_limit: u64,
        /// Simple output format (p=pass, f=fail, ?=unknown/error)
        #[arg(long)]
        simple: bool,
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
        /// Output in HOA format
        #[arg(long)]
        hoa: bool,
        /// Output in DOT format
        #[arg(long)]
        dot: bool,
        /// Output in PNG format (requires Graphviz installed)
        #[arg(long)]
        png: bool,
        /// View the generated PNG file (requires Graphviz installed)
        #[arg(long)]
        view: bool,
        /// Specify a viewer application to open the PNG file (requires Graphviz installed)
        #[arg(long)]
        viewer: Option<String>,
    },
    /// Produce an NBA in HOA format
    Nba {
        /// LTL specification file
        ltl_file: String,
        /// Output in HOA format
        #[arg(long)]
        hoa: bool,
        /// Output in DOT format
        #[arg(long)]
        dot: bool,
        /// Output in PNG format (requires Graphviz installed)
        #[arg(long)]
        png: bool,
        /// View the generated PNG file (requires Graphviz installed)
        #[arg(long)]
        view: bool,
        /// Specify a viewer application to open the PNG file (requires Graphviz installed)
        #[arg(long)]
        viewer: Option<String>,
    },
    /// Check satisfiability of LTL specification
    Sat {
        /// LTL specification file
        ltl_file: String,
        /// Timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,
        /// Memory limit in MB
        #[arg(long, default_value = "4096")]
        memory_limit: u64,
        /// Show counterexample paths and cycles if property is satisfiable
        #[arg(long, short)]
        show_counterexample: bool,
    },
    /// Runs the Philosophers problem
    Philosophers {
        /// Number of philosophers
        philosophers: usize,
        /// Mode of the problem (0: all right, 1: any, or 2: one left, others right)
        mode: usize,
        /// Use generic Petri net representation
        #[arg(long, default_value_t = false)]
        generic: bool,
    },
    /// Convert the dining philosophers problem to a PNML file
    Convert {
        /// Number of philosophers
        philosophers: usize,
        /// Mode of the problem (0: all right, 1: any, or 2: one left, others right)
        mode: usize,
        /// Output PNML file
        output_file: String,
    },
}

fn main() {
    // 1GB Stack space for deep nested state space recursion
    const MAIN_STACK_SIZE: usize = 1024 * 1024 * 1024;
    let builder = std::thread::Builder::new().stack_size(MAIN_STACK_SIZE);
    builder
        .spawn(run)
        .expect("Failed to spawn main thread with increased stack size")
        .join()
        .expect("Main thread panicked");
}

fn run() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Philosophers {
            philosophers,
            mode,
            generic,
        } => {
            println!(
                "{} Number of philosophers: {}, Mode: {}",
                "[Philosophers]".bright_cyan().bold(),
                philosophers.to_string().yellow(),
                mode.to_string().yellow()
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

            println!(
                "{}: {}",
                "Reachable states".bold(),
                states.to_string().green()
            );
            println!(
                "{}: {}",
                "Deadlocks".bold(),
                if deadlocks > 0 {
                    deadlocks.to_string().red()
                } else {
                    "0".green()
                }
            );
        }

        Commands::Reachability { pnml_file } => {
            println!(
                "{} PNML file: {}",
                "[Reachability]".bright_cyan().bold(),
                pnml_file.underline()
            );

            let petri_net = &PetriNetBuilder::build_from_file(&pnml_file)[0];
            let stats = explorer::get_reachability_stats(petri_net);

            println!(
                "{}: {}",
                "Reachable states".bold(),
                stats.reachable_count.to_string().green()
            );
            println!(
                "{}: {}",
                "Deadlocks".bold(),
                if stats.deadlock_count > 0 {
                    stats.deadlock_count.to_string().red()
                } else {
                    "0".green()
                }
            );
        }

        Commands::Check {
            pnml_file,
            ltl_file,
            show_counterexample,
            timeout,
            memory_limit,
            simple,
        } => {
            if !simple {
                println!(
                    "{} PNML: {}, LTL: {}",
                    "[Check]".bright_cyan().bold(),
                    pnml_file.underline(),
                    ltl_file.underline()
                );
                println!(
                    "{} Timeout: {}s, Memory limit: {}MB",
                    "[Config]".bright_cyan().bold(),
                    timeout.to_string().yellow(),
                    memory_limit.to_string().yellow()
                );
            }

            let config = emptyness::ModelCheckConfig::with_limits(timeout, memory_limit);
            let petri_nets = PetriNetBuilder::build_from_file(&pnml_file);

            if petri_nets.is_empty() {
                if simple {
                    eprintln!("ERROR");
                } else {
                    eprintln!(
                        "{} No Petri nets found in file: {}",
                        "Error:".red().bold(),
                        pnml_file
                    );
                }
                return;
            }
            let petri_net = &petri_nets[0];

            match ltl_parser::parse_mcc_file(&ltl_file) {
                Ok(formulas) => {
                    let mut summary = Vec::new();
                    let mut simple_output = String::new();

                    for (name, formula) in formulas {
                        if !simple {
                            println!(
                                "{} \nChecking: {}",
                                format!("[{}]", name).magenta().bold(),
                                formula.to_string().italic()
                            );
                        }

                        match emptyness::model_check(petri_net, &formula, &config) {
                            Ok((result, counterexample_path, counterexample_cycle)) => {
                                summary.push((name.clone(), Ok(result)));
                                simple_output.push(if result { 'p' } else { 'f' });

                                if !simple {
                                    if result {
                                        println!("{}", "Result: Property holds ✔".green().bold());
                                    } else {
                                        println!("{}", "Result: Property is violated ✘ (counterexample exists)".red().bold());

                                        if show_counterexample {
                                            println!();
                                            println!("{}", "Counterexample path:".blue().bold());
                                            printstack(&counterexample_path, petri_net);

                                            println!();
                                            println!("{}", "Counterexample cycle:".blue().bold());
                                            printstack(&counterexample_cycle, petri_net);
                                        }
                                    }
                                }
                            }
                            Err(emptyness::ModelCheckError::Timeout) => {
                                if simple {
                                    simple_output.push('T');
                                } else {
                                    eprintln!(
                                        "{}",
                                        format!(
                                            "  ERROR: Model checking timed out after {}s",
                                            timeout
                                        )
                                        .red()
                                        .bold()
                                    );
                                }
                                summary.push((name.clone(), Err("Timeout".to_string())));
                            }
                            Err(emptyness::ModelCheckError::MemoryLimitExceeded) => {
                                if simple {
                                    simple_output.push('M');
                                } else {
                                    eprintln!(
                                        "{}",
                                        format!(
                                            "  ERROR: Memory limit exceeded ({}MB)",
                                            memory_limit
                                        )
                                        .red()
                                        .bold()
                                    );
                                }
                                summary.push((name.clone(), Err("Memory limit".to_string())));
                            }
                            Err(emptyness::ModelCheckError::CouldNotDetermineMemoryUsage) => {
                                if simple {
                                    simple_output.push('?');
                                } else {
                                    eprintln!(
                                        "{}",
                                        "  ERROR: Could not determine memory usage"
                                            .to_string()
                                            .red()
                                            .bold()
                                    );
                                }
                                summary.push((
                                    name.clone(),
                                    Err("Could not determine memory usage".to_string()),
                                ));
                            }
                        }
                        if !simple {
                            println!();
                        }
                    }

                    if simple {
                        println!("{}", simple_output);
                    } else {
                        println!("{}", "Summary:".yellow().bold());
                        for (name, result) in summary {
                            match result {
                                Ok(true) => println!("  - {}: {}", name, "Holds ✔".green()),
                                Ok(false) => println!("  - {}: {}", name, "Violated ✘".red()),
                                Err(e) => {
                                    println!("  - {}: {}", name, format!("Error ({})", e).red())
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    if simple {
                        eprintln!("PARSE_ERROR");
                    } else {
                        eprintln!("{} {}", "Error parsing LTL file:".red().bold(), e);
                    }
                }
            }
        }

        Commands::Pnf { ltl_file } => {
            println!(
                "{} LTL file: {}",
                "[PNF]".bright_cyan().bold(),
                ltl_file.underline()
            );
            match ltl_parser::parse_mcc_file(&ltl_file) {
                Ok(formulas) => {
                    for (name, formula) in formulas {
                        println!("{}", format!("[{}]", name).magenta().bold());
                        println!("  {} {}", "Original:".blue(), formula);
                        let pnf_formula = pnf::to_pnf(&formula);
                        println!("  {} {}", "PNF:".green(), pnf_formula);
                        println!();
                    }
                }
                Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
            }
        }

        Commands::Gnba {
            ltl_file,
            hoa,
            dot,
            png,
            view,
            viewer,
        } => {
            println!(
                "{} LTL file: {}",
                "[GNBA]".bright_cyan().bold(),
                ltl_file.underline()
            );
            process_automaton(
                "GNBA",
                ltl_file,
                hoa,
                dot,
                png,
                view,
                viewer,
                |f| gnba::GNBA::new(f).to_hoa(),
                |f| gnba::GNBA::new(f).to_dot(),
                |f| gnba::GNBA::new(f).print_stats(),
            );
        }

        Commands::Nba {
            ltl_file,
            hoa,
            dot,
            png,
            view,
            viewer,
        } => {
            println!(
                "{} LTL file: {}",
                "[NBA]".bright_cyan().bold(),
                ltl_file.underline()
            );
            process_automaton(
                "NBA",
                ltl_file,
                hoa,
                dot,
                png,
                view,
                viewer,
                |f| nba::NBA::new(f).to_hoa(),
                |f| nba::NBA::new(f).to_dot(),
                |f| nba::NBA::new(f).print_stats(),
            );
        }

        Commands::Sat {
            ltl_file,
            timeout,
            memory_limit,
            show_counterexample,
        } => {
            println!(
                "{} LTL file: {}",
                "[SAT]".bright_cyan().bold(),
                ltl_file.underline()
            );
            println!(
                "{} Timeout: {}s, Memory limit: {}MB",
                "[Config]".bright_cyan().bold(),
                timeout.to_string().yellow(),
                memory_limit.to_string().yellow()
            );
            let config = emptyness::ModelCheckConfig::with_limits(timeout, memory_limit);
            match ltl_parser::parse_mcc_file(&ltl_file) {
                Ok(formulas) => {
                    for (name, formula) in formulas {
                        println!("{}", format!("[{}]", name).magenta().bold());
                        println!("  {} {}", "Formula:".blue(), formula);
                        match emptyness::is_satisfiable(&formula, &config) {
                            Ok((
                                (nba_sat, stack_path_nba, stack_cycle_nba),
                                (gnba_sat, stack_path_gnba, stack_cycle_gnba),
                            )) => {
                                let sat_str = |val: bool| {
                                    if val {
                                        "Satisfiable".green().bold()
                                    } else {
                                        "Unsatisfiable".red().bold()
                                    }
                                };
                                println!("  NBA:  {}", sat_str(nba_sat));
                                if show_counterexample && nba_sat {
                                    println!("  Counterexample path (NBA):");
                                    printsimplestack(&stack_path_nba);
                                    println!("  Counterexample cycle (NBA):");
                                    printsimplestack(&stack_cycle_nba);
                                    println!();
                                }
                                println!("  GNBA: {}", sat_str(gnba_sat));
                                if show_counterexample && gnba_sat {
                                    println!("  Counterexample path (GNBA):");
                                    printsimplestack(&stack_path_gnba);
                                    println!("  Counterexample cycle (GNBA):");
                                    printsimplestack(&stack_cycle_gnba);
                                }
                                println!();
                            }
                            Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
                        }
                    }
                }
                Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
            }
        }

        Commands::Convert {
            philosophers,
            output_file,
            mode,
        } => {
            println!(
                "{} Philosophers: {}, Output: {}",
                "[Convert]".bright_cyan().bold(),
                philosophers.to_string().yellow(),
                output_file.underline()
            );
            let petri_net = philosophers::build_dining_philosophers(
                philosophers,
                config_generator(mode, philosophers),
            );
            let pnml_content = petri_net.to_pnml();
            if std::fs::write(&output_file, pnml_content).is_ok() {
                println!("{}", "Successfully wrote PNML file.".green());
            } else {
                eprintln!("{}", "Failed to write file.".red().bold());
            }
        }
    }
}

fn process_automaton<FHoa, FDot, FPrint>(
    label: &str,
    ltl_file: String,
    hoa: bool,
    dot: bool,
    png: bool,
    view: bool,
    viewer: Option<String>,
    to_hoa: FHoa,
    to_dot: FDot,
    pretty_print: FPrint,
) where
    FHoa: Fn(&ltl_parser::LTL) -> String,
    FDot: Fn(&ltl_parser::LTL) -> String,
    FPrint: Fn(&ltl_parser::LTL),
{
    match ltl_parser::parse_mcc_file(&ltl_file) {
        Ok(formulas) => {
            for (name, formula) in formulas {
                println!("{}", format!("[{}]", name).magenta().bold());
                if hoa {
                    let hoa_str = to_hoa(&formula);
                    let filename = format!("output/{}_{}.hoa", name, label.to_lowercase());
                    std::fs::create_dir_all("output").ok();
                    std::fs::write(&filename, hoa_str).ok();
                    println!("  {} {}", "Wrote HOA:".blue(), filename.underline());
                } else if dot || png || view || viewer.is_some() {
                    let dot_str = to_dot(&formula);
                    let filename = format!("output/{}_{}.dot", name, label.to_lowercase());
                    std::fs::create_dir_all("output").ok();
                    std::fs::write(&filename, dot_str).ok();
                    println!("  {} {}", "Wrote DOT:".blue(), filename.underline());

                    if is_command_available("dot") && (png || view || viewer.is_some()) {
                        let output_png = format!("output/{}_{}.png", name, label.to_lowercase());
                        let status = Command::new("dot")
                            .args(["-Tpng", &filename, "-o", &output_png])
                            .status();

                        if let Ok(s) = status
                            && s.success()
                        {
                            println!("  {} {}", "Generated PNG:".green(), output_png.underline());
                            if view || viewer.is_some() {
                                if let Some(v) = &viewer {
                                    let _ = open::with(&output_png, v);
                                } else {
                                    let _ = open::that(&output_png);
                                }
                            }
                        }
                    }
                } else {
                    pretty_print(&formula);
                }
                println!();
            }
        }
        Err(e) => eprintln!("{} {}", "Error:".red().bold(), e),
    }
}

fn printstack(stack: &Option<Vec<emptyness::CombinedState>>, pnml: &PetriNet) {
    if let Some(states) = stack {
        for (idx, state) in states.iter().enumerate() {
            println!(
                "  {}: Petri: {}\n     NBA state: {}\n",
                idx,
                pnml.get_state_string(&state.petri_state).yellow(),
                state.nba_state.to_string().cyan()
            );
        }
    } else {
        println!("  (empty)");
    }
}

fn printsimplestack(stack: &Option<Vec<usize>>) {
    if let Some(states) = stack {
        for (idx, state) in states.iter().enumerate() {
            println!("  {}: State {}", idx, state.to_string().yellow());
        }
    } else {
        println!("  (empty)");
    }
}

fn is_command_available(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .output()
        .is_ok()
}

fn config_generator(
    mode: usize,
    n: usize,
) -> Vec<ltl_forge::philosophers::PhilosopherConfiguration> {
    use ltl_forge::philosophers::PhilosopherConfiguration;

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
            2 => PhilosopherConfiguration {
                allowed_left: i == 0,
                allowed_right: i != 0,
            },
            _ => {
                eprintln!(
                    "{} Invalid mode: {}. Using default mode 0.",
                    "Warning:".yellow().bold(),
                    mode
                );
                PhilosopherConfiguration {
                    allowed_left: true,
                    allowed_right: false,
                }
            }
        };
        config.push(philosopher_config);
    }
    config
}
