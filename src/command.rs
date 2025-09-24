use kaoruko_derive::CommandParser;

#[derive(CommandParser)]
pub enum Command {
    #[config(
        alias = "c",
        roles = ["anyone"],
        description = "returns solves for a given query string",
        string_options(
            (required, allow_whitespaces)
        ),
    )]
    Search,
    #[config(
        description = "kicks the bot out of the room",
        roles = ["developer", "creator"],
    )]
    Exit,
    #[config(
        alias = "sn",
        roles = ["developer"],
        description = "start the game now",
    )]
    StartNow,
    #[config(alias = "h", description = "get help for a command", roles = ["anyone"])]
    Help,
}
