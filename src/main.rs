use colored::*;
use std::io::{self, Write};
use std::path::Path;
use std::process;

mod utils;
mod errors;

use errors::InstallerError;
use utils::geode_installer::GeodeInstaller;

enum MenuChoice {
    InstallToSteam,
    InstallToWine,
    Quit,
}

struct UserInterface;

impl UserInterface {
    fn clear_screen() {
        let _ = process::Command::new("clear").status();
    }

    fn print_header() {
        println!("{}", "======================================".yellow().bold());
        println!("{}", "       Geode Installer for Linux     ".yellow().bold());
        println!("{}", "======================================".yellow().bold());
        println!();
    }

    fn print_menu() {
        println!("{}", "Select an action:".white().bold());
        println!();
        println!("{} Install to {}", "1.".blue().bold(), "Steam".blue());
        println!("{} Install to {} prefix", "2.".magenta().bold(), "Wine".magenta());
        println!("{} Quit", "0.".red().bold());
        println!();
    }

    fn read_input(prompt: &str) -> String {
        print!("{}", prompt.white().bold());
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        input.trim().to_string()
    }

    fn read_menu_choice() -> Result<MenuChoice, InstallerError> {
        let input = Self::read_input("What do you want to do: ");
        let n: i32 = input.parse().map_err(|_| InstallerError::NotANumber)?;

        match n {
            1 => Ok(MenuChoice::InstallToSteam),
            2 => Ok(MenuChoice::InstallToWine),
            0 => Ok(MenuChoice::Quit),
            _ => Err(InstallerError::InvalidNumber),
        }
    }

    fn print_success() {
        println!();
        println!("{}", "✅ Geode has been successfully installed!".green().bold());
    }

    fn print_error(message: &InstallerError) {
        println!();
        println!("{}", message.format());
        println!();
        Self::read_input("Press Enter to continue...");
    }
}

struct InstallationHandler {
    installer: GeodeInstaller,
}

impl InstallationHandler {
    fn new() -> Result<Self, InstallerError> {
        Ok(Self {
            installer: GeodeInstaller::new()?,
        })
    }

    fn handle_steam_installation(&self) -> Result<(), InstallerError> {
        println!("{}", "🎮 Installing to Steam...".blue().bold());
        self.installer.install_to_steam()
    }

    fn handle_wine_installation(&self) -> Result<(), InstallerError> {
        println!("{}", "🍷 Wine Installation".magenta().bold());

        let game_path = UserInterface::read_input("Enter your Geometry Dash path: ");
        let wine_prefix = UserInterface::read_input("Enter your Wine prefix path: ");

        self.installer.install_to_wine(
            Path::new(&wine_prefix),
            Path::new(&game_path),
        )
    }

    fn execute(&self, choice: MenuChoice) -> Result<(), InstallerError> {
        match choice {
            MenuChoice::InstallToSteam => Ok(self.handle_steam_installation()?),
            MenuChoice::InstallToWine => Ok(self.handle_wine_installation()?),
            MenuChoice::Quit => Ok(()),
        }
    }
}

fn run_interactive_loop(handler: &InstallationHandler) {
    loop {
        UserInterface::clear_screen();
        UserInterface::print_header();
        UserInterface::print_menu();

        match UserInterface::read_menu_choice() {
            Ok(MenuChoice::Quit) => {
                println!("{}", "👋 Exiting...".yellow().bold());
                break;
            }
            Ok(choice) => match handler.execute(choice) {
                Ok(_) => UserInterface::print_success(),
                Err(e) => UserInterface::print_error(&e),
            },
            Err(e) => UserInterface::print_error(&e),
        }
    }
}

fn main() {
    let handler = InstallationHandler::new().map_err(|e| InstallerError::Init(e.to_string()))
        .unwrap_or_else(|err| {
            eprintln!("{}", err.format());
            process::exit(1);
        });

    run_interactive_loop(&handler);
}
