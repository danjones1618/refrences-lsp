use crate::atlassian_markup_transpiler::transpile_atlassian_markup_to_markdown;
use crate::config::JiraConfig;
use gouqi::{Credentials, Error, Issue, Jira};
use log::warn;
use std::collections::{BTreeMap, HashMap};

pub struct JiraTicket {
    pub key: String,
    pub title: String,
    pub description: String,
    pub assignee: Option<String>,
    pub status: String,
}

impl ToString for JiraTicket {
    fn to_string(&self) -> String {
        format!(
            r"
# {}
---
󱖫 {} | 󰃭 2024-12-31 |  {}

---
{}
",
            self.title,
            self.status,
            self.assignee.as_ref().map_or("Unassigned", |x| x.as_str()),
            self.description
        )
    }
}

impl TryFrom<Issue> for JiraTicket {
    type Error = gouqi::Error;

    fn try_from(ticket: Issue) -> Result<Self, Self::Error> {
        let title = ticket
            .field::<String>("summary")
            .transpose()?
            .unwrap_or("No title".to_owned());
        // let description = ticket.fields.keys().fold(String::new(), |mut acc, x| {
        //     acc.push_str("- ");
        //     acc.push_str(x.as_str());
        //     acc.push_str("\n");
        //     acc
        // });
        let description = ticket
            .field::<Option<String>>("description")
            .transpose()?
            .flatten()
            .map(|mut x| {
                x.push_str("\n\nHere is transpiled:\n\n");
                x.push_str(transpile_atlassian_markup_to_markdown(x.as_str()).as_str());
                x
            })
            .unwrap_or("No description".to_owned());
        let status = ticket
            .field::<BTreeMap<String, ::serde_json::Value>>("status")
            .unwrap()?
            .get("name")
            .map(|value| serde_json::value::from_value::<String>(value.clone()))
            .unwrap();
        let status = match status {
            Ok(value) => value,
            Err(error) => return Err(Error::Serde(error)),
        };

        Ok(JiraTicket {
            key: ticket.key,
            title,
            description,
            assignee: None,
            status,
        })
    }
}

pub struct JiraResolver {
    jira: Jira,
}

impl JiraResolver {
    pub fn new(jira_config: &JiraConfig) -> JiraResolver {
        JiraResolver {
            jira: Jira::new(
                jira_config.host.to_owned(),
                Credentials::Basic(
                    jira_config.email.to_owned(),
                    jira_config.api_token.to_owned(),
                ),
            )
            .expect("err with jira connection"),
        }
    }

    pub fn get_jira_tickets(&self) -> HashMap<String, JiraTicket> {
        self.jira
            .search()
            .iter("project = AUTO", &Default::default())
            .expect("error in jira")
            .filter_map(|issue| {
                let key = issue.key.to_owned();
                match <Issue as TryInto<JiraTicket>>::try_into(issue) {
                    Ok(ticket) => Some((ticket.key.to_owned(), ticket)),
                    Err(e) => {
                        warn!("Dropping ticket {} because {:?}", key, e);
                        None
                    }
                }
            })
            .collect()
    }
}
