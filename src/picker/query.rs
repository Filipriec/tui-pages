#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerCommandQuery {
    tokens: Vec<String>,
    clauses: Vec<PickerCommandClause>,
    ends_with_whitespace: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerCommandClause {
    command: Option<String>,
    argument: Option<String>,
    selection_query: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerCommandArgument {
    None,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerCommandSpec {
    name: String,
    aliases: Vec<String>,
    argument: PickerCommandArgument,
}

impl PickerCommandQuery {
    pub fn parse(query: &str) -> Self {
        Self::parse_with_specs(query, &[])
    }

    pub fn parse_with_specs(query: &str, specs: &[PickerCommandSpec]) -> Self {
        let tokens = query
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut clauses = Vec::new();
        let mut index = 0usize;

        while index < tokens.len() {
            let token = &tokens[index];
            if let Some(command) = token.strip_prefix('%').filter(|command| !command.is_empty()) {
                index += 1;
                let argument_kind = specs
                    .iter()
                    .find(|spec| spec.matches(command))
                    .map(|spec| spec.argument)
                    .unwrap_or(PickerCommandArgument::None);
                let argument = if argument_kind == PickerCommandArgument::Required
                    && index < tokens.len()
                    && !tokens[index].starts_with('%')
                {
                    let argument = tokens[index].clone();
                    index += 1;
                    Some(argument)
                } else {
                    None
                };
                let mut selection_query = Vec::new();
                while index < tokens.len() && !tokens[index].starts_with('%') {
                    selection_query.push(tokens[index].clone());
                    index += 1;
                }
                clauses.push(PickerCommandClause {
                    command: Some(command.to_string()),
                    argument,
                    selection_query,
                });
                continue;
            }

            let mut selection_query = Vec::new();
            if token.starts_with('%') {
                selection_query.push(token.clone());
                index += 1;
            } else {
                while index < tokens.len() && !tokens[index].starts_with('%') {
                    selection_query.push(tokens[index].clone());
                    index += 1;
                }
            }
            clauses.push(PickerCommandClause {
                command: None,
                argument: None,
                selection_query,
            });
        }

        Self {
            tokens,
            clauses,
            ends_with_whitespace: query.chars().last().is_some_and(char::is_whitespace),
        }
    }

    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }

    pub fn clauses(&self) -> &[PickerCommandClause] {
        &self.clauses
    }

    pub fn ends_with_whitespace(&self) -> bool {
        self.ends_with_whitespace
    }

    pub fn has_command(&self, names: &[&str]) -> bool {
        self.clauses.iter().any(|clause| clause.is_command(names))
    }

    pub fn first_command_value(&self, names: &[&str]) -> Option<&str> {
        self.clauses
            .iter()
            .find(|clause| clause.is_command(names))
            .and_then(PickerCommandClause::argument)
    }
}

impl PickerCommandSpec {
    pub fn new(name: impl Into<String>, argument: PickerCommandArgument) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            argument,
        }
    }

    pub fn with_aliases<I, S>(mut self, aliases: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.aliases = aliases.into_iter().map(Into::into).collect();
        self
    }

    fn matches(&self, command: &str) -> bool {
        self.name == command || self.aliases.iter().any(|alias| alias == command)
    }
}

impl PickerCommandClause {
    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }

    pub fn argument(&self) -> Option<&str> {
        self.argument.as_deref()
    }

    pub fn argument_or_empty(&self) -> &str {
        self.argument.as_deref().unwrap_or("")
    }

    pub fn selection_query(&self) -> &[String] {
        &self.selection_query
    }

    pub fn selection_query_text(&self) -> String {
        self.selection_query.join(" ")
    }

    pub fn terms(&self) -> Vec<&str> {
        self.argument
            .iter()
            .map(String::as_str)
            .chain(self.selection_query.iter().map(String::as_str))
            .collect()
    }

    pub fn is_command(&self, names: &[&str]) -> bool {
        self.command
            .as_deref()
            .is_some_and(|command| names.iter().any(|name| command == *name))
    }
}

pub fn parse_picker_command_query(query: &str) -> PickerCommandQuery {
    PickerCommandQuery::parse(query)
}

pub fn parse_picker_command_query_with_specs(
    query: &str,
    specs: &[PickerCommandSpec],
) -> PickerCommandQuery {
    PickerCommandQuery::parse_with_specs(query, specs)
}

#[cfg(test)]
mod tests {
    use super::{
        PickerCommandArgument, PickerCommandSpec, parse_picker_command_query,
        parse_picker_command_query_with_specs,
    };

    #[test]
    fn command_clause_collects_values_until_next_command() {
        let parsed = parse_picker_command_query_with_specs(
            "acme %column name %data bratislava",
            &[
                PickerCommandSpec::new("column", PickerCommandArgument::Required),
                PickerCommandSpec::new("data", PickerCommandArgument::None),
            ],
        );
        let tokens = parsed
            .tokens()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();

        assert_eq!(tokens, vec!["acme", "%column", "name", "%data", "bratislava"]);
        assert_eq!(parsed.clauses().len(), 3);
        assert_eq!(parsed.clauses()[0].command(), None);
        assert_eq!(clause_terms(&parsed, 0), vec!["acme"]);
        assert!(parsed.clauses()[1].is_command(&["column", "col"]));
        assert_eq!(parsed.clauses()[1].argument(), Some("name"));
        assert!(parsed.clauses()[1].selection_query().is_empty());
        assert!(parsed.clauses()[2].is_command(&["data"]));
        assert_eq!(parsed.clauses()[2].argument(), None);
        assert_eq!(clause_selection_query(&parsed, 2), vec!["bratislava"]);
    }

    #[test]
    fn extra_terms_after_command_argument_are_selection_query() {
        let parsed = parse_picker_command_query_with_specs(
            "%column city bratislava",
            &[PickerCommandSpec::new("column", PickerCommandArgument::Required)],
        );

        assert_eq!(parsed.clauses()[0].argument(), Some("city"));
        assert_eq!(clause_selection_query(&parsed, 0), vec!["bratislava"]);
    }

    #[test]
    fn commands_without_argument_treat_first_value_as_selection_query() {
        let parsed = parse_picker_command_query_with_specs(
            "%data bratislava",
            &[PickerCommandSpec::new("data", PickerCommandArgument::None)],
        );

        assert_eq!(parsed.clauses()[0].argument(), None);
        assert_eq!(clause_selection_query(&parsed, 0), vec!["bratislava"]);
    }

    #[test]
    fn records_trailing_whitespace() {
        let parsed = parse_picker_command_query("%target customer_id ");

        assert!(parsed.ends_with_whitespace());
    }

    #[test]
    fn standalone_percent_is_plain_text() {
        let parsed = parse_picker_command_query("% %data value");

        assert_eq!(parsed.clauses()[0].command(), None);
        assert_eq!(clause_terms(&parsed, 0), vec!["%"]);
        assert!(parsed.clauses()[1].is_command(&["data"]));
    }

    fn clause_terms(
        parsed: &super::PickerCommandQuery,
        index: usize,
    ) -> Vec<&str> {
        parsed.clauses()[index].terms()
    }

    fn clause_selection_query(
        parsed: &super::PickerCommandQuery,
        index: usize,
    ) -> Vec<&str> {
        parsed.clauses()[index]
            .selection_query()
            .iter()
            .map(String::as_str)
            .collect()
    }
}
