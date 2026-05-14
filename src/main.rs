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

use crate::petri_net::PetriNet;
use builder::PetriNetBuilder;
use clap::{Parser, Subcommand};
use colored::*;
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
        /// Show counterexample paths and cycles if property is violated
        #[arg(long, short)]
        show_counterexample: bool,
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
        #[arg(long)]
        hoa: bool,
        #[arg(long)]
        dot: bool,
        #[arg(long)]
        png: bool,
        #[arg(long)]
        view: bool,
        #[arg(long)]
        viewer: Option<String>,
    },
    /// Produce an NBA in HOA format
    Nba {
        /// LTL specification file
        ltl_file: String,
        #[arg(long)]
        hoa: bool,
        #[arg(long)]
        dot: bool,
        #[arg(long)]
        png: bool,
        #[arg(long)]
        view: bool,
        #[arg(long)]
        viewer: Option<String>,
    },
    /// Check satisfiability of LTL specification
    Sat {
        /// LTL specification file
        ltl_file: String,
        /// Show counterexample paths and cycles if property is satisfiable
        #[arg(long, short)]
        show_counterexample: bool,
    },
    /// Runs the Philosophers problem
    Philosophers {
        philosophers: usize,
        mode: usize,
        #[arg(long, default_value_t = false)]
        generic: bool,
    },
    /// Convert the dining philosophers problem to a PNML file
    Convert {
        philosophers: usize,
        mode: usize,
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
        } => {
            println!(
                "{} PNML: {}, LTL: {}",
                "[Check]".bright_cyan().bold(),
                pnml_file.underline(),
                ltl_file.underline()
            );
            let petri_nets = PetriNetBuilder::build_from_file(&pnml_file);
            if petri_nets.is_empty() {
                eprintln!(
                    "{} No Petri nets found in file: {}",
                    "Error:".red().bold(),
                    pnml_file
                );
                return;
            }
            let petri_net = &petri_nets[0];

            match ltl_parser::parse_mcc_file(&ltl_file) {
                Ok(formulas) => {
                    let mut summary = Vec::new();
                    for (name, formula) in formulas {
                        println!(
                            "{} \nChecking: {}",
                            format!("[{}]", name).magenta().bold(),
                            formula.to_string().italic()
                        );
                        let (result, counterexample_path, counterexample_cycle) =
                            model_check::model_check(petri_net, &formula);

                        summary.push((name, result));

                        if result {
                            println!("{}", "Result: Property holds ✔".green().bold());
                        } else {
                            println!(
                                "{}",
                                "Result: Property is violated ✘ (counterexample exists)"
                                    .red()
                                    .bold()
                            );

                            if show_counterexample {
                                println!();
                                println!("{}", "Counterexample path:".blue().bold());
                                printstack(&counterexample_path, petri_net);

                                println!();
                                println!("{}", "Counterexample cycle:".blue().bold());
                                printstack(&counterexample_cycle, petri_net);
                            }
                        }
                        println!();
                    }
                    println!("{}", "Summary:".yellow().bold());
                    for (name, result) in summary {
                        println!(
                            "  - {}: {}",
                            name,
                            if result {
                                "Holds ✔".green()
                            } else {
                                "Violated ✘".red()
                            }
                        );
                    }
                }
                Err(e) => eprintln!("{} {}", "Error parsing LTL file:".red().bold(), e),
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
            show_counterexample,
        } => {
            println!(
                "{} LTL file: {}",
                "[SAT]".bright_cyan().bold(),
                ltl_file.underline()
            );
            match ltl_parser::parse_mcc_file(&ltl_file) {
                Ok(formulas) => {
                    for (name, formula) in formulas {
                        println!("{}", format!("[{}]", name).magenta().bold());
                        println!("  {} {}", "Formula:".blue(), formula);
                        let (
                            (nba_sat, stack_path_nba, stack_cycle_nba),
                            (gnba_sat, stack_path_gnba, stack_cycle_gnba),
                        ) = emptyness::is_satisfiable(&formula);

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

                        if let Ok(s) = status {
                            if s.success() {
                                println!(
                                    "  {} {}",
                                    "Generated PNG:".green(),
                                    output_png.underline()
                                );
                                if view || viewer.is_some() {
                                    if let Some(v) = &viewer {
                                        let _ = open::with(&output_png, v);
                                    } else {
                                        let _ = open::that(&output_png);
                                    }
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

fn printstack(stack: &Option<Vec<model_check::CombinedState>>, pnml: &PetriNet) {
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
            2 => PhilosopherConfiguration {
                allowed_left: i == 0,
                allowed_right: i != 0,
            },
            _ => panic!("Invalid mode"),
        };
        config.push(philosopher_config);
    }
    config
}
