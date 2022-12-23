use crate::controller::utils::extract_object_id_or_die;
use crate::database::daos::dao::DAO;
use crate::database::daos::user_dao::UserDAO;
use crate::database::entities::user::Role;
use crate::SETTINGS;

use clap::{Parser, Subcommand};
use std::process::exit;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// print loaded server settings
    #[arg(short)]
    show_settings: bool,

    /// run server after cmd execution (default: false = stop after cmd execution)
    #[arg(short, long)]
    run_server_after_execution: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage users
    #[command(arg_required_else_help(true))]
    User {
        /// lists registered users
        #[arg(short, long)]
        list: bool,

        /// give a user (identified by id) the administrator role
        #[arg(long, value_name = "user_id")]
        set_administrator_role: Option<String>,

        /// give a user (identified by id) the base user role
        #[arg(long, value_name = "user_id")]
        set_base_user_role: Option<String>,
    },
}

pub async fn process() {
    let cli = Cli::parse();

    let mut run_server_after_cmd_execution = true;
    let settings = SETTINGS.get().unwrap();

    if cli.show_settings {
        println!("{:#?}", settings);
        run_server_after_cmd_execution = false;
    }

    match &cli.command {
        Some(Commands::User {
            list,
            set_administrator_role,
            set_base_user_role,
        }) => {
            run_server_after_cmd_execution = false;

            if *list {
                println!("list users ...");
            } else if let Some(set_administrator_role) = set_administrator_role {
                update_user_role(set_administrator_role.clone(), Role::Admin)
                    .await
                    .unwrap();
                println!("successfully added admin role to user");
            } else if let Some(set_base_user_role) = set_base_user_role {
                update_user_role(set_base_user_role.clone(), Role::BaseUser)
                    .await
                    .unwrap();
                println!("successfully removed admin role from user");
            }
        }
        None => {}
    }

    if !cli.run_server_after_execution && !run_server_after_cmd_execution {
        exit(0);
    }
}

pub async fn update_user_role(uid: String, role: Role) -> actix_web::Result<()> {
    let uid = extract_object_id_or_die(Some(&uid))?;
    let user = UserDAO::get(uid).await?;

    if let Some(mut user) = user {
        user.role = role;
        UserDAO::update(&user).await?;
    }

    Ok(())
}
