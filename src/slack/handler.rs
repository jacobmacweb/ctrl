use std::env;

use indoc::indoc;
use rocket::serde::json::serde_json;
use slack_rust::{
    block::{
        self,
        block_elements::{BlockElement, ButtonElement},
        block_object::{TextBlockObject, TextBlockType},
        block_section::SectionBlock,
        blocks::Block,
    },
    chat::post_message::{post_message, PostMessageRequest, PostMessageResponse},
    http_client::{default_client, SlackWebAPIClient},
    socket::socket_mode::SocketMode,
};

use crate::config::{
    get_slack_by_github_username, get_user_by_github_username, get_user_by_slack_id,
    get_user_by_slack_mention, set_user_github_username,
};

pub async fn respond_http_text(
    channel_id: &String,
    text: String,
) -> Result<PostMessageResponse, slack_rust::error::Error> {
    let slack_bot_token = env::var("SLACK_BOT_TOKEN").expect("slack bot token is not set.");
    let request = PostMessageRequest::builder(channel_id.clone())
        .text(text.clone())
        .build();

    post_message(&default_client(), &request, &slack_bot_token).await
}

pub async fn respond_http_blocks(
    channel_id: &String,
    blocks: Vec<Block>,
) -> Result<PostMessageResponse, slack_rust::error::Error> {
    let slack_bot_token = env::var("SLACK_BOT_TOKEN").expect("slack bot token is not set.");
    let request = PostMessageRequest::builder(channel_id.clone())
        .blocks(blocks)
        .build();

    post_message(&default_client(), &request, &slack_bot_token).await
}

async fn respond_text<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
    text: String,
) -> Result<PostMessageResponse, slack_rust::error::Error> {
    let request = PostMessageRequest::builder(channel_id.clone())
        .text(text.clone())
        .build();

    post_message(&socket_mode.api_client, &request, &socket_mode.bot_token).await
}

async fn respond_blocks<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
    blocks: Vec<Block>,
) -> Result<PostMessageResponse, slack_rust::error::Error> {
    let request = PostMessageRequest::builder(channel_id.clone())
        .blocks(blocks)
        .build();

    post_message(&socket_mode.api_client, &request, &socket_mode.bot_token).await
}

pub async fn command_not_found<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
) {
    let _ = respond_text(
        socket_mode,
        channel_id,
        "Invalid command. Use `/ctrl help` for a list of commands.".to_string(),
    )
    .await;
}

pub async fn project_not_found<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
) {
    let _ = respond_text(
        socket_mode,
        channel_id,
        "Project not found. Use `/ctrl list` for a list of projects.".to_string(),
    )
    .await;
}

pub async fn not_enough_arguments<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
) {
    let _ = respond_text(
        socket_mode,
        channel_id,
        "Not enough arguments. Use `/ctrl help` for a list of commands.".to_string(),
    )
    .await;
}

pub async fn user_not_linked<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
) {
    let _ = respond_text(
        socket_mode,
        channel_id,
        "This user must link their GitHub account first. Use `/ctrl me github <github_username>`."
            .to_string(),
    )
    .await;
}

pub async fn help<S: SlackWebAPIClient>(socket_mode: &SocketMode<S>, channel_id: &String) {
    let _ = respond_text(
        socket_mode,
        channel_id,
        indoc! {"
            ⛑️ Here's a simple help guide for all the commands available.
 
            - /ctrl help: Show this help guide.
            - /ctrl list: List all projects.
            - /ctrl project: Show information about the current channel's project.
            - /ctrl create <project_name>: Create a new project, automatically assigning it to this channel and adding you as a manager.
            - /ctrl add <@user>: Add a user as a manager to this project
            - /ctrl remove <@user>: Remove a user as a manager from this project
            - /ctrl github <repo_name>: Set the GitHub repository for this project (PRs will be automatically merged, assigned, etc.).
            - /ctrl me github <github_username>: Set your GitHub username.
            "}.to_string(),
    )
    .await;
}

pub async fn list<S: SlackWebAPIClient>(socket_mode: &SocketMode<S>, channel_id: &String) {
    let manifest = crate::config::read_manifest();
    let projects = manifest.projects.clone();
    let managers = manifest.managers.clone().join(", ");

    let _ = respond_blocks(
        socket_mode,
        channel_id,
        vec![Block::SectionBlock(SectionBlock {
            text: Some(
                TextBlockObject::builder(
                    TextBlockType::Mrkdwn,
                    format!(
                        "Global project managers: {}.\nHere's a list of all projects.",
                        managers
                    ),
                )
                .build(),
            ),
            ..Default::default()
        })]
        .into_iter()
        .chain(
            projects
                .into_iter()
                .map(|(name, project)| {
                    let project_owners = project
                        .project_owners
                        .iter()
                        .map(|github_username| {
                            get_user_by_github_username(&manifest, github_username)
                        })
                        .filter(|name| name.is_some())
                        .map(|f| f.unwrap().github_username.clone())
                        .collect::<Vec<_>>()
                        .join(", ");

                    match project.github_repo {
                        Some(repo) => Block::SectionBlock(SectionBlock {
                            text: Some(
                                TextBlockObject::builder(
                                    TextBlockType::Mrkdwn,
                                    format!(
                                        "{} in <#{}>.\nProject owners: {}",
                                        name, project.slack_channel, project_owners
                                    ),
                                )
                                .build(),
                            ),
                            accessory: Some(BlockElement::ButtonElement(
                                ButtonElement::builder(
                                    TextBlockObject::builder(
                                        TextBlockType::PlainText,
                                        "GitHub".to_string(),
                                    )
                                    .build(),
                                    "github".to_string(),
                                )
                                .url(format!("https://github.com/{}", repo))
                                .build(),
                            )),
                            ..Default::default()
                        }),
                        None => Block::SectionBlock(SectionBlock {
                            text: Some(
                                TextBlockObject::builder(
                                    TextBlockType::Mrkdwn,
                                    format!("{} in <#{}>", name, project.slack_channel),
                                )
                                .build(),
                            ),
                            ..Default::default()
                        }),
                    }
                })
                .collect::<Vec<_>>(),
        )
        .collect::<Vec<_>>(),
    )
    .await;
}

pub async fn create<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
    project_name: &String,
) {
    let mut manifest = crate::config::read_manifest();

    if manifest.projects.contains_key(project_name) {
        let _ = respond_text(
            socket_mode,
            channel_id,
            format!("Project `{}` already exists.", project_name),
        );
        return;
    }

    manifest.projects.insert(
        project_name.clone(),
        crate::config::Project {
            slack_channel: channel_id.clone(),
            project_owners: vec![],
            github_repo: None,
            jira_project: None,
        },
    );

    let _ = respond_text(
        socket_mode,
        channel_id,
        format!("Project `{}` created.", project_name),
    )
    .await;

    crate::config::write_manifest(&manifest);
}

pub async fn delete<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
    project_name: &String,
) {
    let mut manifest = crate::config::read_manifest();

    if !manifest.projects.contains_key(project_name) {
        let _ = respond_text(
            socket_mode,
            channel_id,
            format!("Project `{}` does not exist.", project_name),
        );
        return;
    }

    manifest.projects.remove(project_name);

    let _ = respond_text(
        socket_mode,
        channel_id,
        format!("Project `{}` deleted.", project_name),
    )
    .await;

    crate::config::write_manifest(&manifest);
}

pub async fn add<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
    project_name: &String,
    user_id: &String,
) {
    let mut manifest = crate::config::read_manifest();
    let manifest_clone = manifest.clone();

    if !manifest.projects.contains_key(project_name) {
        let _ = respond_text(
            socket_mode,
            channel_id,
            format!("Project `{}` does not exist.", project_name),
        );
        return;
    }

    let project = manifest.projects.get_mut(project_name).unwrap();

    let user = get_user_by_slack_mention(&manifest_clone, user_id);

    if user.is_none() {
        user_not_linked(socket_mode, channel_id).await;
        return;
    }

    let user = user.unwrap();

    if project.project_owners.contains(&user.github_username) {
        let _ = respond_text(
            socket_mode,
            channel_id,
            format!(
                "User `{}` is already a manager of `{}`.",
                user_id, project_name
            ),
        );
        return;
    }

    project.project_owners.push(user.github_username.clone());

    let _ = respond_text(
        socket_mode,
        channel_id,
        format!(
            "User `{}` added as a manager of `{}`.",
            user_id, project_name
        ),
    )
    .await;

    crate::config::write_manifest(&manifest);
}

pub async fn remove<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
    project_name: &String,
    user_id: &String,
) {
    let mut manifest = crate::config::read_manifest();
    let manifest_clone = manifest.clone();

    if !manifest.projects.contains_key(project_name) {
        let _ = respond_text(
            socket_mode,
            channel_id,
            format!("Project `{}` does not exist.", project_name),
        );
        return;
    }

    let project = manifest.projects.get_mut(project_name).unwrap();

    let user = get_user_by_slack_mention(&manifest_clone, user_id);

    if user.is_none() {
        user_not_linked(socket_mode, channel_id).await;
        return;
    }

    let user = user.unwrap();

    if !project.project_owners.contains(&user.github_username) {
        let _ = respond_text(
            socket_mode,
            channel_id,
            format!("User `{}` is not a manager of `{}`.", user_id, project_name),
        );
        return;
    }

    project
        .project_owners
        .retain(|x| x != &user.github_username);

    let _ = respond_text(
        socket_mode,
        channel_id,
        format!(
            "User `{}` removed as a manager of `{}`.",
            user_id, project_name
        ),
    )
    .await;

    crate::config::write_manifest(&manifest);
}

pub async fn github<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
    project_name: &String,
    repo_name: &String,
) {
    let mut manifest = crate::config::read_manifest();

    if !manifest.projects.contains_key(project_name) {
        let _ = respond_text(
            socket_mode,
            channel_id,
            format!("Project `{}` does not exist.", project_name),
        );
        return;
    }

    let project = manifest.projects.get_mut(project_name).unwrap();

    project.github_repo = Some(repo_name.clone());

    crate::config::write_manifest(&manifest);

    let _ = respond_text(
        socket_mode,
        channel_id,
        format!(
            "GitHub repository `{}` set for `{}`.",
            repo_name, project_name
        ),
    )
    .await;
}

pub async fn me<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
    user_id: &String,
    subcommand: &str,
    value: &String,
) {
    match subcommand {
        "github" => {
            let mut manifest = crate::config::read_manifest();

            set_user_github_username(&mut manifest, user_id, value);

            let _ = respond_text(
                socket_mode,
                channel_id,
                format!("GitHub username set to `{}`.", value),
            )
            .await;

            crate::config::write_manifest(&manifest);
        }
        _ => {
            command_not_found(socket_mode, channel_id).await;
        }
    }
}

pub async fn project<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
    project_name: &String,
) {
    let manifest = crate::config::read_manifest();

    if !manifest.projects.contains_key(project_name) {
        let _ = respond_text(
            socket_mode,
            channel_id,
            format!("Project `{}` does not exist.", project_name),
        );
        return;
    }

    let project = manifest.projects.get(project_name).unwrap();

    let mut text = format!("*Project*: `{}`\n", project_name);

    if let Some(ref github_repo) = project.github_repo {
        text.push_str(&format!(
            "*GitHub*: <https://github.com/{}|{}>\n",
            github_repo, github_repo
        ));
    }

    text.push_str("*Managers*:\n");

    for manager in &project.project_owners {
        let slack_id = get_slack_by_github_username(&manifest, manager);
        let user = get_user_by_github_username(&manifest, manager);

        if user.is_none() || slack_id.is_none() {
            continue;
        }

        let user = user.unwrap();

        text.push_str(&format!(
            "<@{}> ({})\n",
            slack_id.unwrap(),
            user.github_username
        ));
    }

    let _ = respond_text(socket_mode, channel_id, text).await;
}
