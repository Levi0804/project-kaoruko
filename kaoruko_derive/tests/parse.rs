use kaoruko_derive::CommandParser;

#[derive(CommandParser)]
pub enum Command {
    #[config(description = "use for searching words", roles = ["anyone"])]
    Search,
    #[config(description = "kick bot out of room", roles = ["developer"])]
    Exit,
}

fn main() {}
