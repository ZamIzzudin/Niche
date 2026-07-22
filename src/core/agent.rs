use super::client::{Client, StreamResult};
use super::tools::ToolRegistry;
use super::types::{ApiError, Message};
use std::io::{self, Write};

const MAX_ITERATIONS: usize = 30;

pub async fn run_agent_turn(
    client: &Client,
    tools: &ToolRegistry,
    history: &mut Vec<Message>,
    mut on_token: impl FnMut(&str),
) -> Result<(), ApiError> {
    let tool_defs = tools.definitions();

    for iteration in 0..MAX_ITERATIONS {
        let result: StreamResult = client
            .chat_stream(history, &tool_defs, &mut on_token)
            .await?;

        if result.tool_calls.is_empty() {
            if !result.content.is_empty() {
                history.push(Message::assistant(result.content));
            }
            return Ok(());
        }

        let assistant_msg = Message::assistant_with_tools(
            if result.content.is_empty() {
                None
            } else {
                Some(result.content.clone())
            },
            result.tool_calls.clone(),
        );
        history.push(assistant_msg);

        if !result.content.is_empty() {
            println!();
        }

        for tc in &result.tool_calls {
            print_tool_call(&tc.function.name, &tc.function.arguments);
            let _ = io::stdout().flush();

            let args: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::Value::Null);

            let tool_result = match tools.execute(&tc.function.name, &args) {
                Ok(output) => output,
                Err(e) => format!("Error: {e}"),
            };

            print_tool_result(&tool_result);
            history.push(Message::tool_result(tc.id.clone(), tool_result));
        }

        if iteration == MAX_ITERATIONS - 1 {
            eprintln!("\n[warning] Reached max iterations ({MAX_ITERATIONS}), stopping.");
            return Ok(());
        }

        if iteration == MAX_ITERATIONS - 4 {
            eprintln!(
                "\n[warning] Approaching max iterations ({}/{}), wrapping up...",
                iteration + 1,
                MAX_ITERATIONS
            );
        }

        print!("\n");
        let _ = io::stdout().flush();
    }

    Ok(())
}

fn print_tool_call(name: &str, args: &str) {
    let display_args = if args.len() > 200 {
        format!("{}...", &args[..197])
    } else {
        args.to_string()
    };
    eprintln!("\n  [tool] {name}({display_args})");
}

fn print_tool_result(result: &str) {
    let preview = if result.len() > 500 {
        format!("{}...\n  ({} total bytes)", &result[..497], result.len())
    } else {
        result.to_string()
    };

    for line in preview.lines() {
        eprintln!("  {line}");
    }
}
