use config::Config;
use jira_resolver::JiraResolver;
use log::{info, trace};
use lsp_types::{
    request::DocumentLinkRequest, request::GotoDefinition, request::HoverRequest,
    request::InlayHintRequest, request::Request, DocumentLink, DocumentLinkParams,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
    InitializeParams, InlayHint, InlayHintLabel, InlayHintParams, Location, MarkupContent,
    MarkupKind, Position, Range, Uri,
};
use refrence_finder::{InFileRefrenceType, RefrenceFinder};
use serde::Serialize;
use std::error::Error;
use std::str::FromStr;

use lsp_server;
use lsp_server::{Connection, Message, RequestId, Response};

mod atlassian_markup_transpiler;
pub mod config;
mod jira_resolver;
mod refrence_finder;

pub struct Server {
    connection: Connection,
    params: InitializeParams,
    refrence_finder: RefrenceFinder,
    jira_resolver: JiraResolver,
}

impl Server {
    pub fn new(connection: Connection, params: InitializeParams, config: &Config) -> Server {
        Server {
            connection,
            params,
            refrence_finder: RefrenceFinder::new(),
            jira_resolver: JiraResolver::new(&config.jira),
        }
    }
    pub fn run_loop(&mut self) -> Result<(), Box<dyn Error + Sync + Send>> {
        loop {
            let msg = self.connection.receiver.recv()?;
            trace!("got msg: {msg:?}");
            match msg {
                Message::Request(request) => {
                    if self.connection.handle_shutdown(&request)? {
                        return Ok(());
                    }
                    self.handle_request(request)?
                }
                Message::Response(resp) => {
                    info!("got response: {resp:?}");
                }
                Message::Notification(notification) => self.handle_notification(notification)?,
            }
        }
    }

    fn handle_notification(
        &self,
        notification: lsp_server::Notification,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        // lsp_notification!(notification.method.as_str());
        match notification.method.as_str() {
            // notification::
            _ => info!("got notification: {notification:?}"),
        }
        Ok(())
    }

    fn handle_request(
        &mut self,
        request: lsp_server::Request,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        info!("got request: {request:?}");
        match request.method.as_str() {
            GotoDefinition::METHOD => {
                let (request_id, params) = cast::<GotoDefinition>(request)?;
                self.process_goto_definition(&request_id, &params);
            }
            InlayHintRequest::METHOD => {
                let (request_id, params) = cast::<InlayHintRequest>(request)?;
                self.process_inlay_hint_request(&request_id, &params);
            }
            HoverRequest::METHOD => {
                let (request_id, params) = cast::<HoverRequest>(request)?;
                self.process_hover_request(&request_id, &params);
            }
            DocumentLinkRequest::METHOD => {
                let (request_id, params) = cast::<DocumentLinkRequest>(request)?;
                self.process_document_link_request(&request_id, &params);
            }
            method => panic!("Unknown request {method}"),
        }
        Ok(())
    }

    fn process_document_link_request(
        &self,
        request_id: &RequestId,
        document_link_request_params: &DocumentLinkParams,
    ) {
        return;
        let response = DocumentLink {
            range: Range {
                start: Position {
                    line: 7,
                    character: 7,
                },
                end: Position {
                    line: 7,
                    character: 16,
                },
            },
            target: Some(Uri::from_str("https://danjones.dev").unwrap()),
            tooltip: Some(String::from("View in Jira")),
            data: None,
        };
        self.send_response(request_id, &vec![response]);
    }

    fn process_hover_request(
        &mut self,
        request_id: &RequestId,
        hover_request_params: &HoverParams,
    ) {
        let hover_position = hover_request_params.text_document_position_params.position;
        if hover_request_params
            .text_document_position_params
            .text_document
            .uri
            .scheme()
            .map_or("", |x| x.as_str())
            == "output"
        {
            return;
        }
        let file_path = hover_request_params
            .text_document_position_params
            .text_document
            .uri
            .path()
            .as_str();
        let refrence_at_position = self
            .refrence_finder
            .get_refrences(file_path)
            .filter(|&refrence| refrence.range.contains_position(hover_position))
            .next();

        if refrence_at_position.is_none() {
            self.send_empty_resonse(request_id);
            return;
        }
        let refrence_at_position = refrence_at_position.unwrap();

        // TODO: proper jira intergration
        let tickets_in_jira = self.jira_resolver.get_jira_tickets();
        if let Some(ticket) = tickets_in_jira.get(match &refrence_at_position.marker {
            InFileRefrenceType::JiraRefrence { ticket } => ticket.as_str(),
            _ => "",
        }) {
            let response = Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: ticket.to_string(),
                }),
                range: Some(refrence_at_position.range.to_owned().into()),
            };
            self.send_response(request_id, &response);
            return;
        }
        self.send_empty_resonse(request_id);
    }

    fn process_inlay_hint_request(
        &mut self,
        request_id: &RequestId,
        inlay_hint_params: &InlayHintParams,
    ) {
        let tickets_in_jira = self.jira_resolver.get_jira_tickets();
        if inlay_hint_params
            .text_document
            .uri
            .scheme()
            .map_or("", |x| x.as_str())
            == "output"
        {
            return;
        }

        let inlay_hints: Vec<InlayHint> = self
            .refrence_finder
            .get_refrences(inlay_hint_params.text_document.uri.path().as_str())
            .filter_map(|refrence| {
                let position = refrence.range.end_position();
                let ticket = match &refrence.marker {
                    InFileRefrenceType::JiraRefrence { ticket, .. } => ticket,
                    _ => "UNKNOWN",
                };
                tickets_in_jira.get(ticket).map(|jira_ticket| InlayHint {
                    position: position.to_owned(),
                    label: InlayHintLabel::String(format!(
                        ": {} ({})",
                        jira_ticket.title, jira_ticket.status,
                    )),
                    padding_left: None,
                    padding_right: Some(true),
                    kind: None,
                    text_edits: None,
                    tooltip: None,
                    data: None,
                })
            })
            .collect();
        let hint_length = inlay_hints.len();
        info!("Found {hint_length} hints");
        let result = Some(inlay_hints);
        self.send_response(request_id, &result);
    }

    fn process_goto_definition(
        &self,
        request_id: &RequestId,
        goto_definition_params: &GotoDefinitionParams,
    ) {
        info!("got gotoDefinition request #{request_id}: {goto_definition_params:?}");
        let position = goto_definition_params
            .text_document_position_params
            .position;
        let position = Position {
            line: position.line - 1,
            character: 0,
        };
        let new_location = Location::new(
            goto_definition_params
                .text_document_position_params
                .text_document
                .uri
                .to_owned(),
            Range {
                start: position,
                end: position,
            },
        );
        let other_file = Location::new(
            Uri::from_str("/tmp/aaa.txt").unwrap(),
            Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: position,
            },
        );
        let response = GotoDefinitionResponse::Array(vec![new_location, other_file]);
        // let result = Some(new_location);
        self.send_response(request_id, &response);
    }

    fn send_empty_resonse(&self, request_id: &RequestId) {
        let response = Response {
            id: request_id.to_owned(),
            result: None,
            error: None,
        };
        self.connection
            .sender
            .send(Message::Response(response))
            .unwrap();
    }

    fn send_response<T: Serialize>(&self, request_id: &RequestId, response: &T) {
        let result = serde_json::to_value(&response).unwrap();
        let response = Response {
            id: request_id.to_owned(),
            result: Some(result),
            error: None,
        };
        self.connection
            .sender
            .send(Message::Response(response))
            .unwrap();
    }
}

fn cast<R>(request: lsp_server::Request) -> Result<(RequestId, R::Params), String>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    match request.extract(R::METHOD) {
        Ok(it) => Ok(it),
        Err(_) => Err(String::from("There was an error")),
    }
}
