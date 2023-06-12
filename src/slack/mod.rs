use slack_rust;
use slack_rust::chat::post_message::{post_message, PostMessageRequest};
use slack_rust::http_client::{default_client, SlackWebAPIClient};
use slack_rust::socket::event::{HelloEvent, InteractiveEvent, SlashCommandsEvent};
use slack_rust::socket::socket_mode::{ack, EventHandler, SocketMode, Stream};
use slack_rust::views::open::{open, OpenRequest};
use slack_rust::views::view::{View, ViewType};
use std::env;

mod handler;

pub async fn start() {
    let slack_app_token = env::var("SLACK_APP_TOKEN").expect("slack app token is not set.");
    let slack_bot_token = env::var("SLACK_BOT_TOKEN").expect("slack bot token is not set.");
    let api_client = default_client();

    SocketMode::new(api_client, slack_app_token, slack_bot_token)
        .option_parameter("SLACK_CHANNEL_ID".to_string(), "channel_id".to_string())
        .run(&mut Handler)
        .await
        .unwrap_or_else(|_| panic!("socket mode run error."));
}

pub struct Handler;

#[allow(unused_variables)]
#[async_trait]
impl<S> EventHandler<S> for Handler
where
    S: SlackWebAPIClient,
{
    async fn on_hello(&mut self, socket_mode: &SocketMode<S>, e: HelloEvent, s: &mut Stream) {
        println!("hello event: {:?}", e);
    }

    async fn on_slash_commands(
        &mut self,
        socket_mode: &SocketMode<S>,
        e: SlashCommandsEvent,
        s: &mut Stream,
    ) {
        let payload = e.payload;

        ack(&e.envelope_id, s)
            .await
            .expect("socket mode ack error.");

        let text = payload
            .text
            .expect("Text missing");
        let opts = text
            .split_whitespace()
            .collect::<Vec<&str>>();

        let channel_id = payload.channel_id.expect("Channel ID missing");

        if opts.len() < 1 {
            handler::command_not_found(socket_mode, &channel_id).await;
            return;
        };

        let (command, args) = &opts.split_at(1);
        let command = command[0];


        match command {
            "help" => handler::help(socket_mode, &channel_id).await,
            _ => handler::command_not_found(socket_mode, &channel_id).await,
        }
    }
}
