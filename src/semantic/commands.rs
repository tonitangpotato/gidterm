//! Semantic Commands - template-based command execution
//!
//! Tasks can define semantic_commands in their graph YAML:
//! ```yaml
//! tasks:
//!   train_model:
//!     command: python train.py
//!     semantic_commands:
//!       save_checkpoint: "model.save('checkpoint.pth')"
//!       adjust_lr: "optimizer.param_groups[0]['lr'] = {value}"
//!       early_stop: "trainer.should_stop = True"
//! ```

use std::collections::HashMap;

/// A semantic command definition
#[derive(Debug, Clone)]
pub struct SemanticCommand {
    /// Display label for UI
    pub label: String,
    /// Command template (may contain {param} placeholders)
    pub template: String,
    /// Extracted parameter names from template
    pub params: Vec<String>,
}

impl SemanticCommand {
    /// Create from a label and template string
    pub fn new(label: impl Into<String>, template: impl Into<String>) -> Self {
        let template = template.into();
        let params = Self::extract_params(&template);
        Self {
            label: label.into(),
            template,
            params,
        }
    }

    /// Extract {param} placeholders from template
    fn extract_params(template: &str) -> Vec<String> {
        let re = regex::Regex::new(r"\{(\w+)\}").unwrap();
        re.captures_iter(template)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    /// Check if this command requires parameters
    pub fn needs_params(&self) -> bool {
        !self.params.is_empty()
    }

    /// Render the template with provided parameter values
    pub fn render(&self, params: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();
        for (key, value) in params {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }
}

/// Registry of semantic commands for a task
#[derive(Debug, Clone, Default)]
pub struct TaskCommands {
    pub commands: Vec<SemanticCommand>,
}

impl TaskCommands {
    /// Build from the semantic_commands HashMap in a Task
    pub fn from_map(map: &HashMap<String, String>) -> Self {
        let commands = map
            .iter()
            .map(|(label, template)| SemanticCommand::new(label.clone(), template.clone()))
            .collect();
        Self { commands }
    }

    /// Get command by label
    pub fn get(&self, label: &str) -> Option<&SemanticCommand> {
        self.commands.iter().find(|c| c.label == label)
    }

    /// List all command labels
    pub fn labels(&self) -> Vec<&str> {
        self.commands.iter().map(|c| c.label.as_str()).collect()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let cmd = SemanticCommand::new("save", "model.save('checkpoint.pth')");
        assert!(!cmd.needs_params());
        assert_eq!(cmd.render(&HashMap::new()), "model.save('checkpoint.pth')");
    }

    #[test]
    fn test_parameterized_command() {
        let cmd = SemanticCommand::new("adjust_lr", "optimizer.param_groups[0]['lr'] = {value}");
        assert!(cmd.needs_params());
        assert_eq!(cmd.params, vec!["value"]);

        let mut params = HashMap::new();
        params.insert("value".to_string(), "0.0001".to_string());
        assert_eq!(
            cmd.render(&params),
            "optimizer.param_groups[0]['lr'] = 0.0001"
        );
    }

    #[test]
    fn test_task_commands_from_map() {
        let mut map = HashMap::new();
        map.insert("save".to_string(), "model.save('ckpt.pth')".to_string());
        map.insert("stop".to_string(), "trainer.stop()".to_string());

        let cmds = TaskCommands::from_map(&map);
        assert_eq!(cmds.commands.len(), 2);
        assert!(cmds.get("save").is_some());
        assert!(cmds.get("stop").is_some());
        assert!(cmds.get("nonexistent").is_none());
    }
}
