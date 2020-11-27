use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

///
/// Print a heading-style message to the console
pub fn heading(string: &str) {
    println!("{}", style(string).green());
}

///
/// Print a warning to the console
pub fn warn(string: &str) {
    println!("{}", style(string).yellow());
}

///
/// Print a blank line to the console
pub fn newline() {
    println!();
}

///
/// Prompt the user to input text on the command line
pub fn prompt(message: &str) -> String {
    heading(message);
    Input::<String>::new().interact_text().unwrap()
}

///
/// Ask the user for confirmation
pub fn confirm(message: &str) -> bool {
    Confirm::new().with_prompt(message).interact().unwrap()
}

///
/// Allow the user to provide a list of items to select from
pub fn select(items: Vec<String>, selected: &str) -> Result<String, git2::Error> {
    let index_of_current_branch = items
        .iter()
        .position(|name| *name == selected)
        .expect("Unable to find current branch in repo branch list");

    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(index_of_current_branch)
        .interact_on_opt(&Term::stderr())
        .expect("You must select an option")
        .unwrap();

    Ok(items[selection].clone())
}
