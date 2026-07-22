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

        // Build batch for execution
        let batch: Vec<(String, String, serde_json::Value)> = result
            .tool_calls
            .iter()
            .map(|tc| {
                let args: serde_json::Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::Value::Null);
                (tc.id.clone(), tc.function.name.clone(), args)
            })
            .collect();

        let batch_size = batch.len();
        if batch_size > 1 {
            let readonly_count = batch.iter().filter(|(_, name, _)| is_read_only(name)).count();
            if readonly_count > 1 {
                eprintln!(
                    "\n  [batch] {readonly_count} read-only tools running concurrently + {} sequential",
                    batch_size - readonly_count
                );
            }
        }

        for (name, args_str) in result
            .tool_calls
            .iter()
            .map(|tc| (tc.function.name.as_str(), tc.function.arguments.as_str()))
        {
            print_tool_call(name, args_str);
        }
        let _ = io::stdout().flush();

        let results = tools.execute_batch(&batch);

        for (tc, (_id, output)) in result.tool_calls.iter().zip(results.iter()) {
            print_tool_result(output);
            history.push(Message::tool_result(tc.id.clone(), output.clone()));
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

fn is_read_only(name: &str) -> bool {
    matches!(name, "read_file" | "list_files" | "grep" | "glob")
}

fn print_tool_call(name: &str, args: &str) {
    let display_args = if args.len() > 200 {
        format!("{}...", &args[..197])
    } else {
        args.to_string()
    };
    eprintln!("  [tool] {name}({display_args})");
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
