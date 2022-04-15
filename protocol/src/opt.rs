// use clap::Clap;
//
// #[derive(Clap, Debug)]
// pub struct Opt {
//     #[clap(subcommand)]
//     pub command: Command
// }
//
// #[derive(Clap, Debug)]
// pub enum Command {
//     User(UserOpt)
// }
// #[derive(Clap, Debug)]
// pub struct UserOpt {
//     #[clap(subcommand)]
//     pub command: UserCommand
// }
//
// #[derive(Debug, Clap)]
// pub struct User {
//     #[clap(required = true)]
//     pub name: String
// }
//
// #[derive(Clap, Debug)]
// pub enum UserCommand {
//     Create(User),
//
//     Delete(User),
//
//     Update,
//
//     List(User),
//
//     Lists
// }
