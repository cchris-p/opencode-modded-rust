use async_trait::async_trait;
use serde::Deserialize;

use crate::{QuestionDef, Tool, ToolContext, ToolError, ToolResult};

pub struct QuestionTool;

impl QuestionTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct QuestionInput {
    questions: Vec<QuestionDef>,
}

const DESCRIPTION: &str = "Use this tool when you need to ask the user questions during execution. This allows you to:
1. Gather user preferences or requirements
2. Clarify ambiguous instructions
3. Get decisions on implementation choices as you work
4. Offer choices to the user about what direction to take.

Usage notes:
- When `custom` is enabled (default), a \"Type your own answer\" option is added automatically; don't include \"Other\" or catch-all options
- Answers are returned as arrays of labels; set `multiple: true` to allow selecting more than one
- If you recommend a specific option, make that the first option in the list and add \"(Recommended)\" at the end of the label";

#[async_trait]
impl Tool for QuestionTool {
    fn id(&self) -> &str {
        "question"
    }

    fn description(&self) -> &str {
        DESCRIPTION
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "Questions to ask",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "Complete question"
                            },
                            "header": {
                                "type": "string",
                                "maxLength": 30,
                                "description": "Very short label (max 30 chars)"
                            },
                            "multiple": {
                                "type": "boolean",
                                "default": false,
                                "description": "Allow selecting multiple choices"
                            },
                            "custom": {
                                "type": "boolean",
                                "default": true,
                                "description": "Allow typing a custom answer (default: true)"
                            },
                            "options": {
                                "type": "array",
                                "description": "Available choices",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {
                                            "type": "string",
                                            "description": "Display text (1-5 words, concise)"
                                        },
                                        "description": {
                                            "type": "string",
                                            "description": "Explanation of choice"
                                        }
                                    },
                                    "required": ["label", "description"]
                                }
                            }
                        },
                        "required": ["question", "header", "options"]
                    }
                }
            },
            "required": ["questions"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let input: QuestionInput =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        if input.questions.is_empty() {
            return Err(ToolError::InvalidArguments(
                "at least one question is required".to_string(),
            ));
        }

        let answers = ctx.question(input.questions.clone()).await?;

        let output = to_model_output(&input.questions, &answers);

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("answers".to_string(), serde_json::json!(answers));

        Ok(ToolResult {
            title: "Question answered".to_string(),
            output,
            metadata,
            truncated: false,
        })
    }
}

fn to_model_output(questions: &[QuestionDef], answers: &[Vec<String>]) -> String {
    let formatted = questions
        .iter()
        .enumerate()
        .map(|(index, question)| {
            let rendered = answers
                .get(index)
                .filter(|answer| !answer.is_empty())
                .map(|answer| answer.join(", "))
                .unwrap_or_else(|| "Unanswered".to_string());
            format!("\"{}\"=\"{}\"", question.question, rendered)
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "User has answered your questions: {}. You can now continue with the user's answers in mind.",
        formatted
    )
}

impl Default for QuestionTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QuestionOption;

    fn question(header: &str) -> QuestionDef {
        QuestionDef {
            question: "Which database?".to_string(),
            header: header.to_string(),
            options: vec![
                QuestionOption {
                    label: "PostgreSQL".to_string(),
                    description: "Relational database".to_string(),
                },
                QuestionOption {
                    label: "SQLite".to_string(),
                    description: "Embedded database".to_string(),
                },
            ],
            multiple: false,
            custom: true,
        }
    }

    #[test]
    fn parses_question_with_required_header_and_option_description() {
        let input: QuestionInput = serde_json::from_value(serde_json::json!({
            "questions": [{
                "question": "Which database?",
                "header": "Database",
                "options": [{"label": "PostgreSQL", "description": "Relational database"}]
            }]
        }))
        .expect("valid question input");

        assert_eq!(input.questions[0].header, "Database");
        assert_eq!(
            input.questions[0].options[0].description,
            "Relational database"
        );
        assert!(!input.questions[0].multiple);
        assert!(input.questions[0].custom);
    }

    #[test]
    fn missing_header_is_rejected() {
        let result: Result<QuestionInput, _> = serde_json::from_value(serde_json::json!({
            "questions": [{
                "question": "Which database?",
                "options": [{"label": "PostgreSQL", "description": "Relational database"}]
            }]
        }));
        assert!(result.is_err());
    }

    #[test]
    fn missing_option_description_is_rejected() {
        let result: Result<QuestionInput, _> = serde_json::from_value(serde_json::json!({
            "questions": [{
                "question": "Which database?",
                "header": "Database",
                "options": [{"label": "PostgreSQL"}]
            }]
        }));
        assert!(result.is_err());
    }

    #[test]
    fn multiple_defaults_to_false_and_custom_to_true() {
        let input: QuestionInput = serde_json::from_value(serde_json::json!({
            "questions": [{
                "question": "Q",
                "header": "H",
                "options": []
            }]
        }))
        .expect("valid question input");

        assert!(!input.questions[0].multiple);
        assert!(input.questions[0].custom);
    }

    #[test]
    fn custom_can_be_explicitly_disabled() {
        let input: QuestionInput = serde_json::from_value(serde_json::json!({
            "questions": [{
                "question": "Q",
                "header": "H",
                "options": [],
                "custom": false
            }]
        }))
        .expect("valid question input");

        assert!(!input.questions[0].custom);
    }

    #[test]
    fn model_output_single_answer() {
        let questions = vec![question("Database")];
        let output = to_model_output(&questions, &[vec!["PostgreSQL".to_string()]]);
        assert_eq!(
            output,
            "User has answered your questions: \"Which database?\"=\"PostgreSQL\". You can now continue with the user's answers in mind."
        );
    }

    #[test]
    fn model_output_joins_multi_select_and_marks_unanswered() {
        let mut first = question("Database");
        first.multiple = true;
        let second = QuestionDef {
            question: "Extras?".to_string(),
            header: "Extras".to_string(),
            options: vec![],
            multiple: false,
            custom: true,
        };
        let output = to_model_output(
            &[first, second],
            &[vec!["A".to_string(), "B".to_string()], vec![]],
        );
        assert_eq!(
            output,
            "User has answered your questions: \"Which database?\"=\"A, B\", \"Extras?\"=\"Unanswered\". You can now continue with the user's answers in mind."
        );
    }

    #[test]
    fn model_output_preserves_custom_text_answer() {
        let questions = vec![question("Database")];
        let output = to_model_output(&questions, &[vec!["My custom database".to_string()]]);
        assert!(output.contains("\"Which database?\"=\"My custom database\""));
    }

    #[test]
    fn parameters_require_header_options_and_option_description() {
        let schema = QuestionTool::new().parameters();
        let item = &schema["properties"]["questions"]["items"];
        let required = item["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("question")));
        assert!(required.contains(&serde_json::json!("header")));
        assert!(required.contains(&serde_json::json!("options")));
        let option_required = item["properties"]["options"]["items"]["required"]
            .as_array()
            .unwrap();
        assert!(option_required.contains(&serde_json::json!("label")));
        assert!(option_required.contains(&serde_json::json!("description")));
        assert_eq!(
            item["properties"]["custom"]["default"],
            serde_json::json!(true)
        );
    }

    #[tokio::test]
    async fn execute_uses_callback_and_preserves_per_question_answers() {
        let ctx = ToolContext::new("session".into(), "message".into(), "/tmp".into())
            .with_ask_question(|questions| async move {
                assert_eq!(questions.len(), 2);
                Ok(vec![
                    vec!["A".to_string(), "B".to_string()],
                    vec!["C".to_string()],
                ])
            });

        let result = QuestionTool::new()
            .execute(
                serde_json::json!({
                    "questions": [
                        {
                            "question": "Q1",
                            "header": "H1",
                            "options": [
                                {"label": "A", "description": "a"},
                                {"label": "B", "description": "b"}
                            ],
                            "multiple": true
                        },
                        {
                            "question": "Q2",
                            "header": "H2",
                            "options": [{"label": "C", "description": "c"}]
                        }
                    ]
                }),
                ctx,
            )
            .await
            .expect("question callback resolves");

        assert_eq!(
            result.metadata["answers"],
            serde_json::json!([["A", "B"], ["C"]])
        );
        assert_eq!(
            result.output,
            "User has answered your questions: \"Q1\"=\"A, B\", \"Q2\"=\"C\". You can now continue with the user's answers in mind."
        );
    }

    #[tokio::test]
    async fn execute_without_callback_errors_instead_of_reading_stdin() {
        let ctx = ToolContext::new("session".into(), "message".into(), "/tmp".into());

        let result = QuestionTool::new()
            .execute(
                serde_json::json!({
                    "questions": [{
                        "question": "Q",
                        "header": "H",
                        "options": [{"label": "A", "description": "a"}]
                    }]
                }),
                ctx,
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_rejects_empty_question_list() {
        let ctx = ToolContext::new("session".into(), "message".into(), "/tmp".into());

        let result = QuestionTool::new()
            .execute(serde_json::json!({ "questions": [] }), ctx)
            .await;

        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }
}
